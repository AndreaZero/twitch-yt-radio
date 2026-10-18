use std::{
    collections::BTreeMap,
    env,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{OnceLock, RwLock},
    time::{Duration, Instant},
};
use tauri::Manager;

static CONFIG: OnceLock<PathBuf> = OnceLock::new();
static OVERRIDES: RwLock<BTreeMap<String, PathBuf>> = RwLock::new(BTreeMap::new());
static RESOLVED: RwLock<BTreeMap<String, PathBuf>> = RwLock::new(BTreeMap::new());
pub const TOOLS: &[&str] = &["mpv", "yt-dlp", "streamlink", "ffmpeg", "deno"];

pub fn initialize(app: &tauri::AppHandle) -> Result<(), String> {
    let file = app
        .path()
        .app_config_dir()
        .map_err(|_| "Cartella configurazione non disponibile")?
        .join("tools.json");
    CONFIG
        .set(file.clone())
        .map_err(|_| "Configurazione strumenti già inizializzata")?;
    if file.is_file() {
        let bytes = std::fs::read(&file).map_err(|_| "Configurazione strumenti non leggibile")?;
        let values: BTreeMap<String, PathBuf> = serde_json::from_slice(&bytes).map_err(|_| {
            "Configurazione strumenti non valida: ripristina i percorsi nelle impostazioni"
        })?;
        *OVERRIDES
            .write()
            .map_err(|_| "Configurazione strumenti occupata")? = values
            .into_iter()
            .filter(|(key, path)| TOOLS.contains(&key.as_str()) && path.is_absolute())
            .collect();
    }
    Ok(())
}

pub fn configured(name: &str) -> Option<PathBuf> {
    OVERRIDES.read().ok()?.get(name).cloned()
}
pub fn clear_cache() {
    if let Ok(mut cache) = RESOLVED.write() {
        cache.clear();
    }
}

pub fn set_override(name: &str, value: &str) -> Result<(), String> {
    if !TOOLS.contains(&name) {
        return Err("Componente non supportato".into());
    }
    let value = value.trim().trim_matches('"');
    let path = PathBuf::from(value);
    if !value.is_empty() {
        let expected = format!("{name}{}", env::consts::EXE_SUFFIX);
        if value.len() > 32768
            || !path.is_absolute()
            || !path
                .file_name()
                .is_some_and(|v| v.to_string_lossy().eq_ignore_ascii_case(&expected))
        {
            return Err(format!("Indica il percorso completo di {expected}"));
        }
        probe(&path)?;
        if name == "ffmpeg" {
            probe(&path.with_file_name(format!("ffprobe{}", env::consts::EXE_SUFFIX)))?;
        }
    }
    let mut values = OVERRIDES
        .write()
        .map_err(|_| "Configurazione strumenti occupata")?;
    let mut next = values.clone();
    if value.is_empty() {
        next.remove(name);
    } else {
        next.insert(name.into(), path);
    }
    let file = CONFIG
        .get()
        .ok_or("Cartella configurazione non disponibile")?;
    std::fs::create_dir_all(file.parent().ok_or("Cartella configurazione non valida")?)
        .map_err(|_| "Impossibile creare la configurazione strumenti")?;
    std::fs::write(
        file,
        serde_json::to_vec_pretty(&next).map_err(|_| "Configurazione strumenti non valida")?,
    )
    .map_err(|_| "Impossibile salvare i percorsi strumenti")?;
    *values = next;
    clear_cache();
    Ok(())
}

// Read saved Windows PATH values as well as the inherited PATH. Never mutate the
// global process environment while other playback/installer threads are running.
fn path_directories() -> Vec<PathBuf> {
    let mut values: Vec<_> = env::var_os("PATH").into_iter().collect();
    #[cfg(windows)]
    for value in registry_paths() {
        values.push(value.into());
    }
    let mut paths = Vec::new();
    for value in values {
        for path in env::split_paths(&value) {
            if !path.as_os_str().is_empty() && !paths.contains(&path) {
                paths.push(path);
            }
        }
    }
    paths
}

#[cfg(windows)]
fn registry_paths() -> Vec<String> {
    use windows_sys::Win32::System::Registry::{
        RegGetValueW, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_EXPAND_SZ, RRF_RT_REG_SZ,
    };
    let mut values = Vec::new();
    for (key, subkey) in [
        (HKEY_CURRENT_USER, "Environment"),
        (
            HKEY_LOCAL_MACHINE,
            r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment",
        ),
    ] {
        let subkey: Vec<u16> = subkey.encode_utf16().chain(Some(0)).collect();
        let name: Vec<u16> = "Path".encode_utf16().chain(Some(0)).collect();
        let mut buffer = vec![0u16; 32768];
        let mut size = (buffer.len() * 2) as u32;
        if unsafe {
            RegGetValueW(
                key,
                subkey.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ,
                std::ptr::null_mut(),
                buffer.as_mut_ptr() as _,
                &mut size,
            )
        } == 0
        {
            let count = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
            values.push(String::from_utf16_lossy(&buffer[..count]));
        }
    }
    values
}

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

fn push_directory(paths: &mut Vec<PathBuf>, directory: PathBuf, name: &str) {
    let file = directory.join(format!("{name}{}", env::consts::EXE_SUFFIX));
    if executable_file(&file) && !paths.contains(&file) {
        paths.push(file);
    }
}

fn executable_file(path: &Path) -> bool {
    if path.is_file() {
        return true;
    }
    // Windows App Execution Aliases (WinGet) are reparse points; metadata that
    // follows them can fail even though CreateProcess can activate the alias.
    #[cfg(windows)]
    if std::fs::symlink_metadata(path).is_ok_and(|metadata| !metadata.is_dir()) {
        return true;
    }
    false
}

fn candidates(name: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(path) = configured(name) {
        paths.push(path);
    }
    if name == "ffprobe" {
        if let Some(path) = configured("ffmpeg") {
            push_directory(
                &mut paths,
                path.parent().unwrap_or(Path::new("")).to_owned(),
                name,
            );
        }
    }
    if let Ok(exe) = env::current_exe() {
        if let Some(directory) = exe.parent() {
            push_directory(&mut paths, directory.join("bin"), name);
        }
    }
    for directory in path_directories() {
        push_directory(&mut paths, directory, name);
    }
    #[cfg(target_os = "windows")]
    {
        // Install locations remain discoverable even when Explorer inherited an old PATH.
        let mut locals = Vec::new();
        if let Some(local) = env::var_os("LOCALAPPDATA") {
            locals.push(PathBuf::from(local));
        }
        if let Some(home) = env::var_os("USERPROFILE") {
            let home = PathBuf::from(home);
            locals.push(home.join("AppData/Local"));
            push_directory(&mut paths, home.join("scoop/shims"), name);
            push_directory(&mut paths, home.join(".local/bin"), name);
            push_directory(&mut paths, home.join(".deno/bin"), name);
        }
        for local in locals {
            for relative in [
                "Microsoft/WinGet/Links",
                "Microsoft/WindowsApps",
                "Programs/Streamlink/bin",
                "Programs/MPV Player",
                "Programs/mpv",
            ] {
                push_directory(&mut paths, local.join(relative), name);
            }
            let packages = local.join("Microsoft/WinGet/Packages");
            if let Ok(entries) = std::fs::read_dir(packages) {
                for entry in entries.flatten() {
                    let package = entry.file_name().to_string_lossy().to_ascii_lowercase();
                    let relevant = match name {
                        "yt-dlp" => package.starts_with("yt-dlp.yt-dlp_"),
                        "mpv" => {
                            package.starts_with("shinchiro.mpv_") || package.starts_with("mpv.mpv_")
                        }
                        "streamlink" => package.starts_with("streamlink.streamlink_"),
                        "ffmpeg" | "ffprobe" => {
                            package.starts_with("gyan.ffmpeg") || package.starts_with("btbn.ffmpeg")
                        }
                        "deno" => package.starts_with("denoland.deno_"),
                        _ => false,
                    };
                    if relevant {
                        push_directory(&mut paths, entry.path(), name);
                        push_directory(&mut paths, entry.path().join("bin"), name);
                        if let Ok(children) = std::fs::read_dir(entry.path()) {
                            for child in children.flatten() {
                                if child.path().is_dir() {
                                    push_directory(&mut paths, child.path(), name);
                                    push_directory(&mut paths, child.path().join("bin"), name);
                                }
                            }
                        }
                    }
                }
            }
        }
        for variable in ["ProgramW6432", "ProgramFiles", "ProgramFiles(x86)"] {
            if let Some(root) = env::var_os(variable) {
                for relative in ["MPV Player", "mpv", "Streamlink/bin"] {
                    push_directory(&mut paths, PathBuf::from(&root).join(relative), name);
                }
            }
        }
    }
    paths
}

pub fn resolve(name: &str) -> Result<PathBuf, String> {
    if let Some(path) = RESOLVED
        .read()
        .ok()
        .and_then(|cache| cache.get(name).cloned())
        .filter(|path| executable_file(path))
    {
        return Ok(path);
    }
    let mut errors = Vec::new();
    for path in candidates(name) {
        match probe(&path) {
            Ok(()) => {
                if let Ok(mut cache) = RESOLVED.write() {
                    cache.insert(name.into(), path.clone());
                }
                return Ok(path);
            }
            Err(error) => errors.push(error),
        }
    }
    if errors.is_empty() {
        Err(format!(
            "{name}: componente assente. Apri Configurazione guidata nelle impostazioni."
        ))
    } else {
        Err(format!(
            "{name}: trovato ma non avviabile. {}",
            errors.join("; ")
        ))
    }
}

fn probe(path: &Path) -> Result<(), String> {
    if !executable_file(path) {
        return Err(format!("{}: file assente", path.display()));
    }
    let mut command = Command::new(path);
    // ffmpeg uses -version, whereas the other tools accept --version.
    command
        .arg(
            if path.file_stem().is_some_and(|n| {
                matches!(
                    n.to_string_lossy().to_ascii_lowercase().as_str(),
                    "ffmpeg" | "ffprobe"
                )
            }) {
                "-version"
            } else {
                "--version"
            },
        )
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let mut child = command
        .spawn()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(status)) => return Err(format!("{}: errore {status}", path.display())),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(80)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "{}: verifica versione interrotta o scaduta",
                    path.display()
                ));
            }
        }
    }
}

pub fn configure(command: &mut Command) -> Result<(), String> {
    let mut paths = Vec::new();
    for name in ["mpv", "yt-dlp", "streamlink", "ffmpeg", "ffprobe", "deno"] {
        if let Ok(path) = resolve(name) {
            if let Some(parent) = path.parent() {
                if !paths.contains(&parent.to_owned()) {
                    paths.push(parent.to_owned());
                }
            }
        }
    }
    paths.extend(path_directories());
    command.env(
        "PATH",
        env::join_paths(paths).map_err(|_| "Percorsi strumenti non validi")?,
    );
    Ok(())
}

pub fn ytdl_option(path: &Path) -> String {
    // Append passes one map entry without parsing comma-separated items.
    format!(
        "--script-opts-append=ytdl_hook-ytdl_path={}",
        path.display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_executable_is_not_detected() {
        assert!(resolve("background-audio-nonexistent-dependency-7f42").is_err());
    }

    #[test]
    fn skips_missing_files_and_deduplicates_existing_files() {
        let exe = env::current_exe().unwrap();
        let directory = exe.parent().unwrap();
        let name = exe.file_stem().unwrap().to_str().unwrap();
        let mut paths = Vec::new();
        push_directory(&mut paths, directory.to_owned(), "absent-dependency-7f42");
        assert!(paths.is_empty());
        push_directory(&mut paths, directory.to_owned(), name);
        push_directory(&mut paths, directory.to_owned(), name);
        assert_eq!(paths, vec![exe]);
    }

    #[test]
    fn mpv_option_preserves_paths_with_spaces_and_map_separators() {
        assert_eq!(
            ytdl_option(Path::new(r"C:\Tools, Audio\yt-dlp.exe")),
            r"--script-opts-append=ytdl_hook-ytdl_path=C:\Tools, Audio\yt-dlp.exe"
        );
    }
}
