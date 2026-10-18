use serde::{Deserialize, Serialize};
use std::sync::{mpsc, Mutex};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

#[derive(Clone, Copy, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoView {
    pub detached: bool,
    pub fullscreen: bool,
    pub enabled: bool,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoBounds {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    scale: f64,
    visible: bool,
    revision: f64,
}

impl VideoBounds {
    fn validate(self) -> Result<Self, String> {
        if ![
            self.x,
            self.y,
            self.width,
            self.height,
            self.scale,
            self.revision,
        ]
        .iter()
        .all(|v| v.is_finite())
            || self.revision < 0.0
            || !(0.5..=8.0).contains(&self.scale)
            || [self.x, self.y, self.width, self.height]
                .iter()
                .any(|v| !(0.0..=32768.0).contains(v))
        {
            return Err("Dimensioni dell'area video non valide".into());
        }
        Ok(self)
    }

    fn pixels(self, client_width: u32, client_height: u32) -> Option<(i32, i32, i32, i32)> {
        if !self.visible {
            return None;
        }
        let x = (self.x * self.scale).round().min(client_width as f64) as i32;
        let y = (self.y * self.scale).round().min(client_height as f64) as i32;
        let width = (self.width * self.scale)
            .round()
            .min(client_width as f64 - x as f64) as i32;
        let height = (self.height * self.scale)
            .round()
            .min(client_height as f64 - y as f64) as i32;
        (width > 0 && height > 0).then_some((x, y, width, height))
    }
}

#[derive(Default)]
pub struct VideoHost {
    // Stored as an integer so the state is Send; native use is confined to the UI thread.
    handle: usize,
    main_bounds: Option<VideoBounds>,
    detached_bounds: Option<VideoBounds>,
    view: VideoView,
}

pub type SharedVideoHost = Mutex<VideoHost>;

pub fn overlay_target(app: &AppHandle) -> Result<(usize, bool, bool), String> {
    let state = app.state::<SharedVideoHost>();
    let host = state.lock().map_err(|_| "Area video non disponibile")?;
    Ok((host.handle, host.view.detached, host.view.enabled))
}

// Called only from blocking worker threads, never from a window event handler.
pub fn on_ui<T: Send + 'static>(
    app: &AppHandle,
    task: impl FnOnce(&AppHandle) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let (tx, rx) = mpsc::sync_channel(1);
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let _ = tx.send(task(&handle));
    })
    .map_err(|e| format!("Interfaccia video non disponibile: {e}"))?;
    rx.recv()
        .map_err(|_| "Interfaccia video chiusa".to_string())?
}

fn app_window(app: &AppHandle, detached: bool) -> Result<WebviewWindow, String> {
    app.get_webview_window(if detached { "video" } else { "main" })
        .ok_or_else(|| "Finestra del player non disponibile".into())
}

fn announce(app: &AppHandle, host: &VideoHost) {
    let _ = app.emit("video-view-changed", host.view);
}

fn layout(app: &AppHandle, host: &VideoHost) -> Result<(), String> {
    if host.handle == 0 {
        return Ok(());
    }
    let owner = app_window(app, host.view.detached)?;
    let bounds = if host.view.detached {
        host.detached_bounds
    } else {
        host.main_bounds
    };
    let size = owner.inner_size().map_err(|e| e.to_string())?;
    let rect = if host.view.enabled {
        bounds.and_then(|b| b.pixels(size.width, size.height))
    } else {
        None
    };
    native::place(host.handle, rect)
}

pub fn prepare(app: &AppHandle) -> Result<u32, String> {
    on_ui(app, |app| {
        if app.get_webview_window("controls-main").is_none()
            || app.get_webview_window("controls-video").is_none()
        {
            crate::video_overlay::start(app).map_err(|e| format!("Controlli video: {e}"))?;
        }
        let state = app.state::<SharedVideoHost>();
        let mut host = state.lock().map_err(|_| "Area video non disponibile")?;
        if host.handle == 0 {
            host.handle = native::create(&app_window(app, false)?)?;
        }
        // MPV's Win32 --wid requires a nonnegative uint32 HWND.
        Ok(host.handle as u32)
    })
}

pub fn set_enabled(app: &AppHandle, enabled: bool, reset: bool) -> Result<(), String> {
    on_ui(app, move |app| set_enabled_on_ui(app, enabled, reset))
}

pub fn set_enabled_on_ui(app: &AppHandle, enabled: bool, reset: bool) -> Result<(), String> {
    let state = app.state::<SharedVideoHost>();
    let mut host = state.lock().map_err(|_| "Area video non disponibile")?;
    if reset {
        dock(app, &mut host, false)?;
    }
    host.view.enabled = enabled;
    layout(app, &host)?;
    announce(app, &host);
    Ok(())
}

fn dock(app: &AppHandle, host: &mut VideoHost, show_main: bool) -> Result<(), String> {
    app_window(app, host.view.detached)?
        .set_fullscreen(false)
        .map_err(|e| e.to_string())?;
    if host.view.detached && host.handle != 0 {
        native::reparent(host.handle, &app_window(app, false)?)?;
    }
    host.view.detached = false;
    host.view.fullscreen = false;
    app_window(app, true)?.hide().map_err(|e| e.to_string())?;
    if show_main {
        let main = app_window(app, false)?;
        main.show().map_err(|e| e.to_string())?;
        main.set_focus().map_err(|e| e.to_string())?;
    }
    Ok(())
}

// Already on the UI thread. Closing the popout returns the same video to the widget.
pub fn close_detached(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<SharedVideoHost>();
    let mut host = state.lock().map_err(|_| "Area video non disponibile")?;
    dock(app, &mut host, true)?;
    layout(app, &host)?;
    announce(app, &host);
    Ok(())
}

fn validate_caller(window: &WebviewWindow) -> Result<(), String> {
    if matches!(
        window.label(),
        "main" | "video" | "controls-main" | "controls-video"
    ) {
        Ok(())
    } else {
        Err("Finestra non autorizzata per il player".into())
    }
}

#[tauri::command]
pub fn get_video_view(
    window: WebviewWindow,
    state: tauri::State<'_, SharedVideoHost>,
) -> Result<VideoView, String> {
    validate_caller(&window)?;
    Ok(state.lock().map_err(|_| "Area video non disponibile")?.view)
}

#[tauri::command]
pub async fn update_video_bounds(
    app: AppHandle,
    window: WebviewWindow,
    bounds: VideoBounds,
) -> Result<(), String> {
    validate_caller(&window)?;
    if !matches!(window.label(), "main" | "video") {
        return Err("Questa finestra non possiede l'area video".into());
    }
    let bounds = bounds.validate()?;
    let detached = window.label() == "video";
    tauri::async_runtime::spawn_blocking(move || {
        on_ui(&app, move |app| {
            let state = app.state::<SharedVideoHost>();
            let mut host = state.lock().map_err(|_| "Area video non disponibile")?;
            let previous = if detached {
                &mut host.detached_bounds
            } else {
                &mut host.main_bounds
            };
            if previous.is_some_and(|old| old.revision >= bounds.revision) {
                return Ok(());
            }
            *previous = Some(bounds);
            // A stale layout from the other window cannot move or hide the active video.
            if detached == host.view.detached {
                layout(app, &host)?;
            }
            Ok(())
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn set_video_detached(
    app: AppHandle,
    window: WebviewWindow,
    detached: bool,
) -> Result<(), String> {
    validate_caller(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        on_ui(&app, move |app| {
            let state = app.state::<SharedVideoHost>();
            let mut host = state.lock().map_err(|_| "Area video non disponibile")?;
            if detached && (!host.view.enabled || host.handle == 0) {
                return Err("Avvia un video prima di staccare il player".into());
            }
            if detached == host.view.detached {
                return Ok(());
            }
            if detached {
                let popout = app_window(app, true)?;
                app_window(app, false)?
                    .set_fullscreen(false)
                    .map_err(|e| e.to_string())?;
                native::reparent(host.handle, &popout)?;
                host.view.detached = true;
                host.view.fullscreen = false;
                popout.show().map_err(|e| e.to_string())?;
                popout.set_focus().map_err(|e| e.to_string())?;
            } else {
                dock(app, &mut host, true)?;
            }
            layout(app, &host)?;
            announce(app, &host);
            Ok(())
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn set_video_fullscreen(
    app: AppHandle,
    window: WebviewWindow,
    fullscreen: bool,
) -> Result<(), String> {
    validate_caller(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        on_ui(&app, move |app| {
            let state = app.state::<SharedVideoHost>();
            let mut host = state.lock().map_err(|_| "Area video non disponibile")?;
            if fullscreen && !host.view.enabled {
                return Err("Avvia un video prima di aprire lo schermo intero".into());
            }
            app_window(app, host.view.detached)?
                .set_fullscreen(fullscreen)
                .map_err(|e| e.to_string())?;
            host.view.fullscreen = fullscreen;
            // Hide the old rectangle until React has measured the new fullscreen layout.
            native::place(host.handle, None)?;
            announce(app, &host);
            Ok(())
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(windows)]
mod native {
    use super::*;
    use std::ptr::null_mut;
    use windows_sys::Win32::{
        Foundation::HWND,
        UI::WindowsAndMessaging::{
            CreateWindowExW, SetParent, SetWindowPos, ShowWindow, HWND_TOP, SWP_NOACTIVATE,
            SWP_SHOWWINDOW, SW_HIDE, WS_CHILD, WS_CLIPCHILDREN, WS_CLIPSIBLINGS,
        },
    };

    pub fn create(parent: &WebviewWindow) -> Result<usize, String> {
        let parent = parent.hwnd().map_err(|e| e.to_string())?.0;
        let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
        // SAFETY: UI thread, live app-owned parent, built-in STATIC class. SS_BLACKRECT=4.
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_CLIPCHILDREN | WS_CLIPSIBLINGS | 4,
                0,
                0,
                1,
                1,
                parent,
                null_mut(),
                null_mut(),
                std::ptr::null(),
            )
        };
        if hwnd.is_null() {
            return Err(format!(
                "Impossibile creare l'area video: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(hwnd as usize)
    }

    pub fn reparent(handle: usize, parent: &WebviewWindow) -> Result<(), String> {
        let parent = parent.hwnd().map_err(|e| e.to_string())?.0;
        // SAFETY: both parent windows and the child host belong to this app and UI thread.
        // The host always remains WS_CHILD; no cross-process window is reparented here.
        unsafe {
            ShowWindow(handle as HWND, SW_HIDE);
            if SetParent(handle as HWND, parent).is_null() {
                return Err(format!(
                    "Impossibile spostare il video: {}",
                    std::io::Error::last_os_error()
                ));
            }
        }
        Ok(())
    }

    pub fn place(handle: usize, rect: Option<(i32, i32, i32, i32)>) -> Result<(), String> {
        if handle == 0 {
            return Ok(());
        }
        // SAFETY: called on the UI thread with the app-owned host, clipped client coordinates.
        unsafe {
            if let Some((x, y, width, height)) = rect {
                if SetWindowPos(
                    handle as HWND,
                    HWND_TOP,
                    x,
                    y,
                    width,
                    height,
                    SWP_NOACTIVATE | SWP_SHOWWINDOW,
                ) == 0
                {
                    return Err(format!(
                        "Impossibile ridimensionare il video: {}",
                        std::io::Error::last_os_error()
                    ));
                }
            } else {
                ShowWindow(handle as HWND, SW_HIDE);
            }
        }
        Ok(())
    }
}

#[cfg(not(windows))]
mod native {
    use super::*;
    pub fn create(_: &WebviewWindow) -> Result<usize, String> {
        Err("Video integrato disponibile solo su Windows".into())
    }
    pub fn reparent(_: usize, _: &WebviewWindow) -> Result<(), String> {
        Err("Video integrato disponibile solo su Windows".into())
    }
    pub fn place(_: usize, _: Option<(i32, i32, i32, i32)>) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds() -> VideoBounds {
        VideoBounds {
            x: 10.0,
            y: 20.0,
            width: 400.0,
            height: 225.0,
            scale: 1.5,
            visible: true,
            revision: 1.0,
        }
    }

    #[test]
    fn scales_and_clips_video_to_its_own_client_area() {
        assert_eq!(bounds().pixels(600, 360), Some((15, 30, 585, 330)));
        assert_eq!(bounds().pixels(10, 10), None);
        let mut hidden = bounds();
        hidden.visible = false;
        assert_eq!(hidden.pixels(600, 360), None);
    }

    #[test]
    fn rejects_invalid_native_coordinates() {
        assert!(bounds().validate().is_ok());
        for invalid in [f64::NAN, f64::INFINITY, -1.0, 40000.0] {
            let mut b = bounds();
            b.width = invalid;
            assert!(b.validate().is_err());
        }
        let mut b = bounds();
        b.scale = 0.0;
        assert!(b.validate().is_err());
    }
}
