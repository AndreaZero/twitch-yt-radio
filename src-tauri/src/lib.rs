mod chat;
mod dependencies;
mod playback;
mod token_store;
mod video_host;
mod video_overlay;
mod youtube;
use playback::PlaybackMode;
use serde::Serialize;
use std::{
    io::Write,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, State, WindowEvent,
};
use url::Url;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Default)]
struct Player {
    child: Option<Child>,
    generation: u64,
    ipc_name: String,
    current_url: Option<String>,
    paused: bool,
    volume: u8,
    mode: PlaybackMode,
}

type SharedPlayer = Arc<Mutex<Player>>;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct EndEvent {
    generation: u64,
    failed: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    active: bool,
    current_url: Option<String>,
    generation: u64,
    paused: bool,
    volume: u8,
    mode: PlaybackMode,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Dependencies {
    mpv: bool,
    yt_dlp: bool,
    streamlink: bool,
}

#[derive(Clone, Copy)]
enum Source {
    Youtube,
    Twitch,
}

fn validate_source(value: &str) -> Result<Source, String> {
    if value.len() > 2048 {
        return Err("URL troppo lungo".into());
    }
    let url =
        Url::parse(value).map_err(|_| "Inserisci un URL completo (https://...)".to_string())?;
    if url.scheme() != "https" {
        return Err("Sono consentiti soltanto URL HTTPS".into());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("L'URL non può contenere credenziali".into());
    }
    match url.host_str().unwrap_or("").to_ascii_lowercase().as_str() {
        "youtube.com" | "www.youtube.com" | "m.youtube.com" | "music.youtube.com" | "youtu.be" => {
            Ok(Source::Youtube)
        }
        "twitch.tv" | "www.twitch.tv" | "m.twitch.tv" => Ok(Source::Twitch),
        _ => Err("Per ora sono supportati solo YouTube e Twitch".into()),
    }
}

fn hide_console(command: &mut Command) {
    #[cfg(target_os = "windows")]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(target_os = "windows"))]
    let _ = command;
}

fn available(binary: &str) -> bool {
    dependencies::resolve(binary).is_ok()
}

#[tauri::command]
fn check_dependencies() -> Dependencies {
    Dependencies {
        mpv: available("mpv"),
        yt_dlp: available("yt-dlp"),
        streamlink: available("streamlink"),
    }
}

fn resolve_twitch(url: &str, mode: PlaybackMode) -> Result<String, String> {
    // Twitch espone audio_only soltanto quando il flusso lo prevede.
    // Non ripieghiamo su un flusso video nascosto: consumerebbe banda inutilmente.
    let streamlink = dependencies::resolve("streamlink")?;
    let mut cmd = Command::new(streamlink);
    cmd.args(["--stream-url", url, mode.twitch_quality()])
        .stdin(Stdio::null());
    hide_console(&mut cmd);
    let output = cmd
        .output()
        .map_err(|_| "Streamlink non trovato. Installalo e riavvia l'app".to_string())?;
    if !output.status.success() {
        return Err(match mode {
            PlaybackMode::AudioOnly => "Canale offline, audio_only non disponibile o Twitch temporaneamente non accessibile",
            PlaybackMode::VideoAudio => "Canale offline o flusso video Twitch non disponibile",
        }.into());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let stream_url = text
        .lines()
        .map(str::trim)
        .find(|v| v.starts_with("https://"))
        .ok_or_else(|| "Streamlink non ha restituito un flusso valido".to_string())?;
    Ok(stream_url.to_owned())
}

fn kill_current(player: &mut Player) {
    if let Some(mut child) = player.child.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
    player.current_url = None;
    player.paused = false;
    player.ipc_name.clear();
}

fn monitor(shared: SharedPlayer, app: AppHandle, generation: u64) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(350));
        let ended = {
            let mut player = match shared.lock() {
                Ok(p) => p,
                Err(_) => return,
            };
            if player.generation != generation {
                return;
            }
            match player.child.as_mut() {
                Some(child) => match child.try_wait() {
                    Ok(Some(status)) => {
                        player.child.take();
                        player.current_url = None;
                        player.paused = false;
                        Some(!status.success())
                    }
                    Ok(None) => None,
                    Err(_) => {
                        player.child.take();
                        player.current_url = None;
                        Some(true)
                    }
                },
                None => return,
            }
        };
        if let Some(failed) = ended {
            // Recheck on the UI thread: a new generation may already have started.
            let shared = shared.clone();
            let _ = video_host::on_ui(&app, move |app| {
                let current = shared.lock().map_err(|_| "Player non disponibile")?;
                if current.generation == generation && current.child.is_none() {
                    video_host::set_enabled_on_ui(app, false, false)?;
                }
                Ok(())
            });
            let _ = app.emit("playback-ended", EndEvent { generation, failed });
            return;
        }
    });
}

fn do_start(
    url: String,
    volume: u8,
    mode: PlaybackMode,
    expected_generation: u64,
    shared: SharedPlayer,
    app: AppHandle,
) -> Result<u64, String> {
    let source = validate_source(&url)?;
    let mpv = dependencies::resolve("mpv")?;
    let yt_dlp = if matches!(source, Source::Youtube) {
        Some(dependencies::resolve("yt-dlp")?)
    } else {
        None
    };
    let playback_url = match source {
        Source::Youtube => url.clone(),
        Source::Twitch => resolve_twitch(&url, mode)?,
    };
    let video_handle = if mode == PlaybackMode::VideoAudio {
        Some(video_host::prepare(&app)?)
    } else {
        None
    };
    let mut player = shared
        .lock()
        .map_err(|_| "Player non disponibile".to_string())?;
    if player.generation != expected_generation {
        return Err("Avvio annullato: riproduzione cambiata o arrestata".into());
    }
    let generation = player.generation.wrapping_add(1);
    let pipe_name = format!("background_audio_{}_{}", std::process::id(), generation);
    let pipe_path = format!(r"\\.\pipe\{}", pipe_name);
    let clamped_volume = volume.min(100);
    let mut command = Command::new(mpv);
    command.args([
        "--no-config",
        "--audio-display=no",
        "--no-terminal",
        "--no-osc",
        "--no-input-default-bindings",
        "--volume-max=100",
    ]);
    command.args(mode.window_args());
    if let Some(handle) = video_handle {
        command.arg(format!("--wid={handle}"));
    }
    command.arg(format!("--volume={clamped_volume}"));
    command.arg(format!("--input-ipc-server={pipe_path}"));
    if matches!(source, Source::Youtube) {
        command.arg("--ytdl=yes");
        command.arg(format!("--ytdl-format={}", mode.youtube_format()));
        if let Some(path) = &yt_dlp {
            command.arg(dependencies::ytdl_option(path));
        }
    }
    command.arg(playback_url);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    hide_console(&mut command);
    let child = command
        .spawn()
        .map_err(|e| format!("Impossibile avviare MPV: {e}"))?;
    kill_current(&mut player);
    player.generation = generation;
    player.child = Some(child);
    player.current_url = Some(url);
    player.ipc_name = pipe_name;
    player.volume = clamped_volume;
    player.paused = false;
    player.mode = mode;
    drop(player);
    let shared_for_view = shared.clone();
    if let Err(error) = video_host::on_ui(&app, move |app| {
        let player = shared_for_view
            .lock()
            .map_err(|_| "Player non disponibile")?;
        if player.generation != generation || player.child.is_none() {
            return Err("Avvio annullato: riproduzione cambiata o arrestata".into());
        }
        video_host::set_enabled_on_ui(
            app,
            mode == PlaybackMode::VideoAudio,
            mode == PlaybackMode::AudioOnly,
        )
    }) {
        if let Ok(mut player) = shared.lock() {
            if player.generation == generation {
                kill_current(&mut player);
            }
        }
        let shared_for_cleanup = shared.clone();
        let _ = video_host::on_ui(&app, move |app| {
            let player = shared_for_cleanup
                .lock()
                .map_err(|_| "Player non disponibile")?;
            if player.generation == generation && player.child.is_none() {
                video_host::set_enabled_on_ui(app, false, true)?;
            }
            Ok(())
        });
        emit_playback_state(&app, &shared);
        return Err(error);
    }
    emit_playback_state(&app, &shared);
    monitor(shared, app, generation);
    Ok(generation)
}

#[tauri::command]
async fn start_playback(
    url: String,
    volume: u8,
    mode: Option<PlaybackMode>,
    app: AppHandle,
    state: State<'_, SharedPlayer>,
) -> Result<u64, String> {
    let shared = state.inner().clone();
    let expected_generation = shared
        .lock()
        .map_err(|_| "Player non disponibile")?
        .generation;
    tauri::async_runtime::spawn_blocking(move || {
        do_start(
            url,
            volume,
            mode.unwrap_or_default(),
            expected_generation,
            shared,
            app,
        )
    })
    .await
    .map_err(|e| format!("Errore interno: {e}"))?
}

#[tauri::command]
async fn stop_playback(app: AppHandle, state: State<'_, SharedPlayer>) -> Result<(), String> {
    let shared = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut player = shared
            .lock()
            .map_err(|_| "Player non disponibile".to_string())?;
        player.generation = player.generation.wrapping_add(1);
        kill_current(&mut player);
        drop(player);
        video_host::set_enabled(&app, false, true)?;
        emit_playback_state(&app, &shared);
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

// IPC locale di MPV: non è un endpoint remoto e il nome è unico per processo/sessione.
#[cfg(target_os = "windows")]
fn send_ipc(pipe_name: &str, command: serde_json::Value) -> Result<(), String> {
    let path = format!(r"\\.\pipe\{}", pipe_name);
    let message = format!("{}\n", command);
    for _ in 0..12 {
        match std::fs::OpenOptions::new().write(true).open(&path) {
            Ok(mut pipe) => {
                return pipe
                    .write_all(message.as_bytes())
                    .map_err(|e| format!("IPC MPV: {e}"))
            }
            Err(_) => thread::sleep(Duration::from_millis(100)),
        }
    }
    Err("MPV non è ancora pronto per i comandi. Riprova tra un momento".into())
}

#[cfg(not(target_os = "windows"))]
fn send_ipc(_pipe_name: &str, _command: serde_json::Value) -> Result<(), String> {
    Err("Questa prima versione supporta i controlli IPC su Windows".into())
}

#[tauri::command]
fn set_paused(paused: bool, app: AppHandle, state: State<'_, SharedPlayer>) -> Result<(), String> {
    let (pipe_name, generation) = {
        let p = state
            .lock()
            .map_err(|_| "Player non disponibile".to_string())?;
        if p.child.is_none() {
            return Err("Nessuna riproduzione attiva".into());
        }
        (p.ipc_name.clone(), p.generation)
    };
    send_ipc(
        &pipe_name,
        serde_json::json!({ "command": ["set_property", "pause", paused] }),
    )?;
    let mut p = state
        .lock()
        .map_err(|_| "Player non disponibile".to_string())?;
    if p.generation == generation {
        p.paused = paused;
    }
    drop(p);
    emit_playback_state(&app, state.inner());
    Ok(())
}

#[tauri::command]
fn set_volume(volume: u8, app: AppHandle, state: State<'_, SharedPlayer>) -> Result<(), String> {
    let volume = volume.min(100);
    let (pipe_name, generation, active) = {
        let p = state
            .lock()
            .map_err(|_| "Player non disponibile".to_string())?;
        (p.ipc_name.clone(), p.generation, p.child.is_some())
    };
    if active {
        send_ipc(
            &pipe_name,
            serde_json::json!({ "command": ["set_property", "volume", volume] }),
        )?;
    }
    let mut p = state
        .lock()
        .map_err(|_| "Player non disponibile".to_string())?;
    if p.generation == generation {
        p.volume = volume;
    }
    drop(p);
    emit_playback_state(&app, state.inner());
    Ok(())
}

#[tauri::command]
fn get_playback_state(state: State<'_, SharedPlayer>) -> Result<Snapshot, String> {
    let p = state
        .lock()
        .map_err(|_| "Player non disponibile".to_string())?;
    Ok(snapshot(&p))
}

fn snapshot(p: &Player) -> Snapshot {
    Snapshot {
        active: p.child.is_some(),
        current_url: p.current_url.clone(),
        generation: p.generation,
        paused: p.paused,
        volume: p.volume,
        mode: p.mode,
    }
}

fn emit_playback_state(app: &AppHandle, shared: &SharedPlayer) {
    if let Ok(player) = shared.lock() {
        let _ = app.emit("playback-state-changed", snapshot(&player));
    }
}

pub fn run() {
    tauri::Builder::default()
        .manage(chat::SharedChat::default())
        .manage(youtube::SharedYoutube::default())
        .manage(video_host::SharedVideoHost::default())
        .manage(Arc::new(Mutex::new(Player {
            volume: 70,
            ..Player::default()
        })))
        .invoke_handler(tauri::generate_handler![
            check_dependencies,
            chat::chat_get_state,
            chat::chat_set_detached,
            chat::chat_open_settings,
            chat::chat_login,
            chat::chat_logout,
            chat::chat_watch,
            chat::chat_retry,
            chat::chat_send_message,
            chat::chat_open_activation,
            chat::chat_open_developer_console,
            youtube::youtube_get_state,
            youtube::youtube_login,
            youtube::youtube_logout,
            youtube::youtube_watch,
            youtube::youtube_retry,
            youtube::youtube_more_comments,
            youtube::youtube_send_message,
            youtube::youtube_set_detached,
            youtube::youtube_open_console,
            start_playback,
            stop_playback,
            set_paused,
            set_volume,
            get_playback_state,
            video_overlay::set_video_overlay_interacting,
            video_host::get_video_view,
            video_host::update_video_bounds,
            video_host::set_video_detached,
            video_host::set_video_fullscreen
        ])
        .setup(|app| {
            chat::restore(app.handle());
            youtube::restore(app.handle());
            let show = MenuItem::with_id(app, "show", "Apri player", true, None::<&str>)?;
            let hide = MenuItem::with_id(app, "hide", "Nascondi", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Esci", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &hide, &quit])?;
            let icon = app
                .default_window_icon()
                .ok_or("Icona applicazione mancante")?
                .clone();
            TrayIconBuilder::with_id("background-audio")
                .icon(icon)
                .tooltip("Background Audio")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "hide" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.hide();
                        }
                    }
                    "quit" => {
                        if let Some(shared) = app.try_state::<SharedPlayer>() {
                            if let Ok(mut p) = shared.lock() {
                                kill_current(&mut p);
                            }
                        }
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if window.label() == "video" {
                    if let Err(error) = video_host::close_detached(window.app_handle()) {
                        let _ = window.emit("video-view-error", error);
                    }
                } else if window.label() == "youtube-chat" {
                    if let Err(error) = youtube::close_detached(window.app_handle()) {
                        let _ = window.emit("youtube-window-error", error);
                    }
                } else if window.label() == "chat" {
                    if let Err(error) = chat::close_detached(window.app_handle()) {
                        let _ = window.emit("chat-window-error", error);
                    }
                } else {
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("Impossibile avviare Background Audio");
}
