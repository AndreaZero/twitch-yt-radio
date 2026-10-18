//! Guided setup for installer and standalone EXE alike. Only allowlisted package
//! IDs and fixed browser destinations are exposed, never a frontend shell.
use crate::dependencies;
use serde::{Deserialize, Serialize};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::{
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tool {
    Mpv,
    YtDlp,
    Streamlink,
    Ffmpeg,
    Deno,
}
impl Tool {
    fn name(self) -> &'static str {
        match self {
            Self::Mpv => "mpv",
            Self::YtDlp => "yt-dlp",
            Self::Streamlink => "streamlink",
            Self::Ffmpeg => "ffmpeg",
            Self::Deno => "deno",
        }
    }
    fn package(self) -> &'static str {
        match self {
            Self::Mpv => "shinchiro.mpv",
            Self::YtDlp => "yt-dlp.yt-dlp",
            Self::Streamlink => "Streamlink.Streamlink",
            Self::Ffmpeg => "Gyan.FFmpeg.Essentials",
            Self::Deno => "DenoLand.Deno",
        }
    }
    fn url(self) -> &'static str {
        match self {
            Self::Mpv => "https://mpv.io/installation/",
            Self::YtDlp => "https://github.com/yt-dlp/yt-dlp#installation",
            Self::Streamlink => "https://streamlink.github.io/install.html",
            Self::Ffmpeg => "https://www.gyan.dev/ffmpeg/builds/",
            Self::Deno => "https://docs.deno.com/runtime/getting_started/installation/",
        }
    }
}
const TOOLS: [Tool; 5] = [
    Tool::Mpv,
    Tool::YtDlp,
    Tool::Streamlink,
    Tool::Ffmpeg,
    Tool::Deno,
];

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolStatus {
    name: String,
    available: bool,
    path: Option<String>,
    configured_path: Option<String>,
    error: String,
    package: String,
}
#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    version: u64,
    running: bool,
    current: String,
    message: String,
    completed: Vec<String>,
    errors: Vec<String>,
}
#[derive(Default)]
pub struct Setup {
    running: AtomicBool,
    progress: Mutex<Progress>,
    configuration_error: Mutex<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    tools: Vec<ToolStatus>,
    winget: bool,
    progress: Progress,
    configuration_error: String,
}

pub fn initialize(app: &AppHandle) {
    if let Err(error) = dependencies::initialize(app) {
        let state = app.state::<Setup>();
        if let Ok(mut message) = state.configuration_error.lock() {
            *message = error;
        };
    }
}
fn available(tool: Tool) -> Result<std::path::PathBuf, String> {
    let path = dependencies::resolve(tool.name())?;
    if matches!(tool, Tool::Ffmpeg) {
        dependencies::resolve("ffprobe")?;
    }
    Ok(path)
}
fn report(app: &AppHandle) -> Result<Report, String> {
    let tools = TOOLS
        .iter()
        .map(|tool| {
            let result = available(*tool);
            ToolStatus {
                name: tool.name().into(),
                available: result.is_ok(),
                path: result
                    .as_ref()
                    .ok()
                    .map(|path| path.to_string_lossy().into_owned()),
                configured_path: dependencies::configured(tool.name())
                    .map(|path| path.to_string_lossy().into_owned()),
                error: result.err().unwrap_or_default(),
                package: tool.package().into(),
            }
        })
        .collect();
    let state = app.state::<Setup>();
    let progress = state
        .progress
        .lock()
        .map_err(|_| "Configurazione occupata")?
        .clone();
    let configuration_error = state
        .configuration_error
        .lock()
        .map_err(|_| "Configurazione occupata")?
        .clone();
    Ok(Report {
        tools,
        winget: dependencies::resolve("winget").is_ok(),
        progress,
        configuration_error,
    })
}
fn progress(app: &AppHandle, change: impl FnOnce(&mut Progress)) {
    let state = app.state::<Setup>();
    if let Ok(mut value) = state.progress.lock() {
        change(&mut value);
        value.version += 1;
        let _ = app.emit("setup-progress", value.clone());
    };
}

#[tauri::command]
pub async fn setup_report(app: AppHandle) -> Result<Report, String> {
    tauri::async_runtime::spawn_blocking(move || {
        dependencies::clear_cache();
        report(&app)
    })
    .await
    .map_err(|_| "Rilevamento componenti interrotto")?
}

#[tauri::command]
pub async fn setup_set_path(
    app: AppHandle,
    state: State<'_, Setup>,
    tool: Tool,
    path: String,
) -> Result<Report, String> {
    if state
        .running
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("Attendi il completamento della configurazione in corso".into());
    }
    let worker = app.clone();
    // The lock is released by the worker, even when a caller closes its window.
    tauri::async_runtime::spawn_blocking(move || {
        let result = dependencies::set_override(tool.name(), &path);
        worker
            .state::<Setup>()
            .running
            .store(false, Ordering::SeqCst);
        result?;
        let state = worker.state::<Setup>();
        if let Ok(mut error) = state.configuration_error.lock() {
            error.clear();
        }
        report(&worker)
    })
    .await
    .map_err(|_| "Salvataggio percorso interrotto")?
}

#[tauri::command]
pub fn setup_install(
    app: AppHandle,
    state: State<'_, Setup>,
    tool: Option<Tool>,
) -> Result<(), String> {
    if state
        .running
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("Installazione già in corso".into());
    }
    progress(&app, |value| {
        let version = value.version;
        *value = Progress {
            version,
            running: true,
            message: "Preparazione installazione…".into(),
            ..Default::default()
        };
    });
    // No terminal windows, shell interpolation or package IDs supplied by React.
    std::thread::spawn(move || {
        let result = install(&app, tool);
        dependencies::clear_cache();
        progress(&app, |value| {
            value.running = false;
            value.current.clear();
            if let Err(error) = result {
                value.errors.push(error);
            }
            value.message = if value.errors.is_empty() {
                "Componenti pronti. Puoi riprodurre senza riavviare l’app."
            } else {
                "Configurazione incompleta. Controlla i dettagli e riprova i componenti mancanti."
            }
            .into();
        });
        app.state::<Setup>().running.store(false, Ordering::SeqCst);
    });
    Ok(())
}

fn install(app: &AppHandle, selected: Option<Tool>) -> Result<(), String> {
    let winget = dependencies::resolve("winget").map_err(|_| "WinGet non disponibile. Installa/aggiorna App Installer dal pulsante della guida, poi premi Rileva di nuovo")?;
    for tool in selected
        .map(|tool| vec![tool])
        .unwrap_or_else(|| TOOLS.to_vec())
    {
        if available(tool).is_ok() {
            continue;
        }
        progress(app, |value| {
            value.current = tool.name().into();
            value.message = format!("Installazione {} tramite WinGet…", tool.name());
        });
        let mut command = Command::new(&winget);
        command
            .args([
                "install",
                "--id",
                tool.package(),
                "--exact",
                "--source",
                "winget",
                "--silent",
                "--disable-interactivity",
                "--accept-source-agreements",
                "--accept-package-agreements",
                "--force",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(windows)]
        command.creation_flags(0x08000000);
        let installed = match run_installer(&mut command) {
            Ok(()) => Ok(()),
            Err((message, true)) => return Err(format!("{}: {message}", tool.name())),
            Err((message, false)) => Err(message),
        };
        let result = installed.and_then(|()| {
            dependencies::clear_cache();
            available(tool).map(|_| ()).map_err(|error| format!("WinGet terminato, ma {} non è pronto: {error}. Se era già installato, verifica i percorsi o reinstalla dal sito ufficiale.", tool.name()))
        });
        progress(app, |value| match result {
            Ok(()) => value.completed.push(tool.name().into()),
            Err(error) => value.errors.push(format!("{}: {error}", tool.name())),
        });
    }
    Ok(())
}
// The bool marks an installer that may still be running: stop the queue rather
// than launching another installer concurrently after a timeout/monitor error.
fn run_installer(command: &mut Command) -> Result<(), (String, bool)> {
    let mut child = command
        .spawn()
        .map_err(|e| (format!("Impossibile avviare WinGet: {e}"), false))?;
    let deadline = Instant::now() + Duration::from_secs(20 * 60);
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(status)) => return Err((format!("WinGet: codice 0x{:08X}. Verifica rete, richieste di autorizzazione Windows e App Installer; puoi usare il sito ufficiale o indicare un percorso locale.", status.code().unwrap_or(-1) as u32), false)),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(300)),
            _ => {
                // An installer may already be changing files: do not kill its process tree.
                return Err(("Installazione oltre 20 minuti o stato non leggibile. Un installer potrebbe essere ancora attivo: controlla Windows prima di riprovare".into(), true));
            }
        }
    }
}

#[tauri::command]
pub fn setup_open_download(tool: Option<Tool>) -> Result<(), String> {
    open_fixed(
        tool.map(Tool::url)
            .unwrap_or("https://apps.microsoft.com/detail/9nblggh4nns1"),
    )
}
pub fn open_fixed(url: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        let url: Vec<u16> = url.encode_utf16().chain(Some(0)).collect();
        let verb: Vec<u16> = "open".encode_utf16().chain(Some(0)).collect();
        if unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                verb.as_ptr(),
                url.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1,
            )
        } as isize
            <= 32
        {
            return Err("Impossibile aprire il sito ufficiale nel browser".into());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = url;
        Err("Configurazione supportata su Windows".into())
    }
}
