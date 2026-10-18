use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

fn push_directory(paths: &mut Vec<PathBuf>, directory: PathBuf, name: &str) {
    let file = directory.join(format!("{name}{}", env::consts::EXE_SUFFIX));
    if file.is_file() && !paths.contains(&file) {
        paths.push(file);
    }
}

fn candidates(name: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(path) = env::var_os("PATH") {
        for directory in env::split_paths(&path) {
            if !directory.as_os_str().is_empty() {
                push_directory(&mut paths, directory, name);
            }
        }
    }
    if let Ok(exe) = env::current_exe() {
        if let Some(directory) = exe.parent() {
            push_directory(&mut paths, directory.join("bin"), name);
        }
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
        }
        for local in locals {
            for relative in [
                "Microsoft/WinGet/Links",
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
    let mut errors = Vec::new();
    for path in candidates(name) {
        let mut command = Command::new(&path);
        command
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(target_os = "windows")]
        command.creation_flags(0x08000000);
        match command.status() {
            Ok(status) if status.success() => return Ok(path),
            Ok(status) => errors.push(format!("{}: {status}", path.display())),
            Err(error) => errors.push(format!("{}: {error}", path.display())),
        }
    }
    if errors.is_empty() {
        Err(format!("{name}: eseguibile assente nel PATH e nelle cartelle di installazione. Reinstalla {name}; un collegamento WinGet senza il file di destinazione non basta."))
    } else {
        Err(format!(
            "{name}: trovato ma non avviabile. {}",
            errors.join("; ")
        ))
    }
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
