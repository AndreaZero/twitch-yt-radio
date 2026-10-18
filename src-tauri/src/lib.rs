mod dependencies;
mod playback;
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
    stopped: bool,
}

#[derive(Serialize)]
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
                        let stopped = playback::manually_closed(player.mode, status.code());
                        player.child.take();
                        player.current_url = None;
                        player.paused = false;
                        Some((!status.success() && !stopped, stopped))
                    }
                    Ok(None) => None,
                    Err(_) => {
                        player.child.take();
                        player.current_url = None;
                        Some((true, false))
                    }
                },
                None => return,
            }
        };
        if let Some((failed, stopped)) = ended {
            let _ = app.emit(
                "playback-ended",
                EndEvent {
                    generation,
                    failed,
                    stopped,
                },
            );
            return;
        }
    });
}

fn do_start(
    url: String,
    volume: u8,
    mode: PlaybackMode,
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
    let video_input = if mode == PlaybackMode::VideoAudio {
        let directory = app
            .path()
            .app_cache_dir()
            .map_err(|e| format!("Cartella cache non disponibile: {e}"))?;
        std::fs::create_dir_all(&directory)
            .map_err(|e| format!("Impossibile creare la cache video: {e}"))?;
        let path = directory.join("video-input.conf");
        // Only this fixed binding is loaded; controls remain in the widget.
        std::fs::write(&path, playback::VIDEO_INPUT)
            .map_err(|e| format!("Impossibile configurare la finestra video: {e}"))?;
        Some(path)
    } else {
        None
    };
    let mut player = shared
        .lock()
        .map_err(|_| "Player non disponibile".to_string())?;
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
    if let Some(path) = video_input {
        command.arg(format!("--input-conf={}", path.display()));
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
    tauri::async_runtime::spawn_blocking(move || {
        do_start(url, volume, mode.unwrap_or_default(), shared, app)
    })
    .await
    .map_err(|e| format!("Errore interno: {e}"))?
}

#[tauri::command]
fn stop_playback(state: State<'_, SharedPlayer>) -> Result<(), String> {
    let mut player = state
        .lock()
        .map_err(|_| "Player non disponibile".to_string())?;
    player.generation = player.generation.wrapping_add(1);
    kill_current(&mut player);
    Ok(())
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
fn set_paused(paused: bool, state: State<'_, SharedPlayer>) -> Result<(), String> {
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
    Ok(())
}

#[tauri::command]
fn set_volume(volume: u8, state: State<'_, SharedPlayer>) -> Result<(), String> {
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
    Ok(())
}

#[tauri::command]
fn get_playback_state(state: State<'_, SharedPlayer>) -> Result<Snapshot, String> {
    let p = state
        .lock()
        .map_err(|_| "Player non disponibile".to_string())?;
    Ok(Snapshot {
        active: p.child.is_some(),
        current_url: p.current_url.clone(),
        generation: p.generation,
        paused: p.paused,
        volume: p.volume,
        mode: p.mode,
    })
}

pub fn run() {
    tauri::Builder::default()
        .manage(Arc::new(Mutex::new(Player {
            volume: 70,
            ..Player::default()
        })))
        .invoke_handler(tauri::generate_handler![
            check_dependencies,
            start_playback,
            stop_playback,
            set_paused,
            set_volume,
            get_playback_state
        ])
        .setup(|app| {
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
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("Impossibile avviare Background Audio");
}
