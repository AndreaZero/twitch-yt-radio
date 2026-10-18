//! Standalone EXEs need a native recovery path before any WebView can exist.
#[cfg(windows)]
pub fn ensure_webview() -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, IDYES, MB_ICONERROR, MB_ICONINFORMATION, MB_OK, MB_YESNO,
    };
    if tauri::webview_version().is_ok() {
        return true;
    }
    let title: Vec<u16> = "Background Audio · configurazione iniziale"
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let prompt: Vec<u16> = "Per aprire l’app serve Microsoft Edge WebView2. Vuoi scaricare da Microsoft e avviare l’installazione guidata? Serve Internet; il download può richiedere qualche minuto. In seguito configureremo gli strumenti audio/video.".encode_utf16().chain(Some(0)).collect();
    if unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            prompt.as_ptr(),
            title.as_ptr(),
            MB_YESNO | MB_ICONINFORMATION,
        )
    } != IDYES
    {
        return false;
    }
    match install_webview() {
        Ok(()) if tauri::webview_version().is_ok() => true,
        result => {
            let error = result.err().unwrap_or(
                "WebView2 non è ancora disponibile. Completa l’installazione e riapri l’app".into(),
            );
            let message: Vec<u16> = format!("{error}\n\nAprirò la pagina Microsoft: scegli Evergreen Bootstrapper, installalo e riapri l’EXE.").encode_utf16().chain(Some(0)).collect();
            unsafe {
                MessageBoxW(
                    std::ptr::null_mut(),
                    message.as_ptr(),
                    title.as_ptr(),
                    MB_OK | MB_ICONERROR,
                );
            }
            let _ = crate::setup::open_fixed(
                "https://developer.microsoft.com/microsoft-edge/webview2/",
            );
            false
        }
    }
}
#[cfg(not(windows))]
pub fn ensure_webview() -> bool {
    true
}

#[cfg(windows)]
fn install_webview() -> Result<(), String> {
    use std::{
        io::Write,
        os::windows::{ffi::OsStrExt, process::CommandExt},
        process::{Command, Stdio},
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    };
    use windows_sys::Win32::Security::WinTrust::*;
    let directory = std::env::temp_dir().join(format!(
        "background-audio-webview-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    std::fs::create_dir(&directory)
        .map_err(|_| "Impossibile creare la cartella temporanea WebView2")?;
    let file = directory.join("MicrosoftEdgeWebview2Setup.exe");
    let result = (|| -> Result<(), String> {
        tauri::async_runtime::block_on(async {
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(120))
                .redirect(reqwest::redirect::Policy::custom(|attempt| {
                    let url = attempt.url();
                    let host = url.host_str().unwrap_or("");
                    if attempt.previous().len() < 8
                        && url.scheme() == "https"
                        && (host == "microsoft.com" || host.ends_with(".microsoft.com"))
                    {
                        attempt.follow()
                    } else {
                        attempt.stop()
                    }
                }))
                .build()
                .map_err(|_| "Download WebView2 non disponibile")?;
            let mut response = client
                .get("https://go.microsoft.com/fwlink/p/?LinkId=2124703")
                .send()
                .await
                .map_err(|_| "Download WebView2 interrotto. Verifica la connessione")?
                .error_for_status()
                .map_err(|_| "Microsoft non ha fornito il programma di installazione")?;
            if !response.status().is_success() {
                return Err("Microsoft ha richiesto un reindirizzamento non consentito. Usa la pagina ufficiale WebView2".into());
            }
            let mut output = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&file)
                .map_err(|_| "Impossibile salvare il bootstrapper WebView2")?;
            let mut size = 0;
            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|_| "Download WebView2 incompleto")?
            {
                size += chunk.len();
                if size > 10 * 1024 * 1024 {
                    return Err("Bootstrapper WebView2 oltre il limite previsto".into());
                }
                output
                    .write_all(&chunk)
                    .map_err(|_| "Impossibile scrivere il bootstrapper WebView2")?;
            }
            output
                .sync_all()
                .map_err(|_| "Impossibile completare il download WebView2")?;
            Ok::<_, String>(())
        })?;
        let path: Vec<u16> = file.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut info = WINTRUST_FILE_INFO {
            cbStruct: std::mem::size_of::<WINTRUST_FILE_INFO>() as u32,
            pcwszFilePath: path.as_ptr(),
            ..Default::default()
        };
        let mut data = WINTRUST_DATA {
            cbStruct: std::mem::size_of::<WINTRUST_DATA>() as u32,
            dwUIChoice: WTD_UI_NONE,
            fdwRevocationChecks: WTD_REVOKE_WHOLECHAIN,
            dwUnionChoice: WTD_CHOICE_FILE,
            Anonymous: WINTRUST_DATA_0 { pFile: &mut info },
            dwStateAction: WTD_STATEACTION_VERIFY,
            dwProvFlags: WTD_DISABLE_MD2_MD4,
            ..Default::default()
        };
        let mut action = WINTRUST_ACTION_GENERIC_VERIFY_V2;
        let trusted =
            unsafe { WinVerifyTrust(std::ptr::null_mut(), &mut action, &mut data as *mut _ as _) };
        data.dwStateAction = WTD_STATEACTION_CLOSE;
        unsafe {
            WinVerifyTrust(std::ptr::null_mut(), &mut action, &mut data as *mut _ as _);
        }
        if trusted != 0 {
            return Err(
                "Firma Windows del bootstrapper non verificata. Il file non è stato eseguito"
                    .into(),
            );
        }
        let mut child = Command::new(&file)
            .arg("/install")
            .creation_flags(0x08000000)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| "Impossibile avviare il programma Microsoft WebView2")?;
        let deadline = Instant::now() + Duration::from_secs(20 * 60);
        loop {
            match child.try_wait() {
                Ok(Some(status)) if status.success() => return Ok(()),
                Ok(Some(status)) => {
                    return Err(format!("Installazione WebView2 interrotta: {status}"))
                }
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(300))
                }
                _ => {
                    return Err(
                        "Completa la finestra di installazione WebView2, poi riapri l’app".into(),
                    )
                }
            }
        }
    })();
    // Only remove the exact file and empty directory created by this invocation.
    let _ = std::fs::remove_file(file);
    let _ = std::fs::remove_dir(directory);
    result
}
