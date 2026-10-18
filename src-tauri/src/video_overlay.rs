//! Owned transparent windows sit above MPV's native HWND without shrinking the picture.
use tauri::{AppHandle, Emitter, Manager};
static INTERACTION: std::sync::Mutex<Option<(String, std::time::Instant)>> =
    std::sync::Mutex::new(None);

#[tauri::command]
pub fn set_video_overlay_interacting(
    window: tauri::WebviewWindow,
    interacting: bool,
) -> Result<(), String> {
    if !matches!(window.label(), "controls-main" | "controls-video") {
        return Err("Finestra controlli non autorizzata".into());
    }
    let mut state = INTERACTION
        .lock()
        .map_err(|_| "Controlli video non disponibili")?;
    *state = if interacting {
        Some((window.label().into(), std::time::Instant::now()))
    } else {
        None
    };
    Ok(())
}

pub fn start(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        use tauri::{WebviewUrl, WebviewWindowBuilder};
        for (label, parent) in [("controls-main", "main"), ("controls-video", "video")] {
            if app.get_webview_window(label).is_some() {
                continue;
            }
            let owner = app
                .get_webview_window(parent)
                .ok_or("Finestra video mancante")?;
            WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
                .title("Controlli video")
                .inner_size(420.0, 60.0)
                .decorations(false)
                .transparent(true)
                .shadow(false)
                .resizable(false)
                .skip_taskbar(true)
                .visible(false)
                .focused(false)
                .owner(&owner)?
                .build()?;
        }
        let app = app.clone();
        std::thread::spawn(move || {
            let mut hover = Hover::default();
            loop {
                std::thread::sleep(std::time::Duration::from_millis(120));
                match crate::video_host::on_ui(&app, move |app| {
                    tick(app, &mut hover)?;
                    Ok(hover)
                }) {
                    Ok(next) => hover = next,
                    Err(error) => {
                        let _ = app.emit("video-view-error", error);
                        break;
                    }
                }
            }
        });
    }
    #[cfg(not(windows))]
    let _ = app;
    Ok(())
}

#[cfg(windows)]
#[derive(Default)]
struct Hover {
    point: Option<(i32, i32)>,
    moved_at: Option<std::time::Instant>,
    inside: bool,
    owner: usize,
    rect: Option<(i32, i32, i32, i32)>,
}

#[cfg(windows)]
fn tick(app: &AppHandle, hover: &mut Hover) -> Result<(), String> {
    use windows_sys::Win32::{
        Foundation::{POINT, RECT},
        UI::WindowsAndMessaging::{
            GetAncestor, GetCursorPos, GetForegroundWindow, GetWindowRect, IsIconic,
            IsWindowVisible, SetWindowPos, ShowWindow, WindowFromPoint, GA_ROOT, HWND_TOP,
            SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_HIDE,
        },
    };
    let (video, detached, enabled) = crate::video_host::overlay_target(app)?;
    let owner = app
        .get_webview_window(if detached { "video" } else { "main" })
        .ok_or("Player chiuso")?;
    let active = app
        .get_webview_window(if detached {
            "controls-video"
        } else {
            "controls-main"
        })
        .ok_or("Controlli chiusi")?;
    let inactive = app
        .get_webview_window(if detached {
            "controls-main"
        } else {
            "controls-video"
        })
        .ok_or("Controlli chiusi")?;
    let owner_hwnd = owner.hwnd().map_err(|e| e.to_string())?.0;
    let overlay = active.hwnd().map_err(|e| e.to_string())?.0;
    let other = inactive.hwnd().map_err(|e| e.to_string())?.0;
    let mut rect: RECT = unsafe { std::mem::zeroed() };
    let mut point = POINT { x: 0, y: 0 };
    // All HWNDs are app-owned; this runs on the UI thread. Cursor coordinates are
    // used only for local hover detection, never recorded or sent to the frontend.
    unsafe {
        if IsWindowVisible(other) != 0 {
            ShowWindow(other, SW_HIDE);
        }
        let foreground = GetAncestor(GetForegroundWindow(), GA_ROOT);
        let app_in_foreground = ["main", "video", "chat", "controls-main", "controls-video"]
            .iter()
            .any(|label| {
                app.get_webview_window(label)
                    .and_then(|window| window.hwnd().ok())
                    .is_some_and(|handle| handle.0 == foreground)
            });
        let visible = enabled
            && video != 0
            && IsWindowVisible(video as _) != 0
            && IsWindowVisible(owner_hwnd) != 0
            && IsIconic(owner_hwnd) == 0
            && app_in_foreground
            && GetWindowRect(video as _, &mut rect) != 0
            && GetCursorPos(&mut point) != 0;
        let root_at_pointer = GetAncestor(WindowFromPoint(point), GA_ROOT);
        let inside = visible
            && (root_at_pointer == owner_hwnd || root_at_pointer == overlay)
            && point.x >= rect.left
            && point.x < rect.right
            && point.y >= rect.top
            && point.y < rect.bottom;
        let now = std::time::Instant::now();
        if inside
            && (!hover.inside
                || hover.point != Some((point.x, point.y))
                || hover.owner != owner_hwnd as usize)
        {
            hover.moved_at = Some(now);
        }
        hover.point = Some((point.x, point.y));
        hover.inside = inside;
        hover.owner = owner_hwnd as usize;
        let over_controls = root_at_pointer == overlay && IsWindowVisible(overlay) != 0;
        let dragging = INTERACTION.lock().ok().is_some_and(|state| {
            state.as_ref().is_some_and(|(label, at)| {
                label == active.label() && now.duration_since(*at).as_secs() < 30
            })
        });
        let show = visible
            && (dragging
                || (inside
                    && (over_controls
                        || hover
                            .moved_at
                            .is_some_and(|at| now.duration_since(at).as_millis() < 2000))));
        if !show {
            if IsWindowVisible(overlay) != 0 {
                ShowWindow(overlay, SW_HIDE);
            }
            hover.rect = None;
            return Ok(());
        }
        let scale = owner.scale_factor().map_err(|e| e.to_string())?;
        let inset = (6.0 * scale).round() as i32;
        let width = (rect.right - rect.left - 2 * inset)
            .min((620.0 * scale).round() as i32)
            .max(1);
        let height = ((60.0 * scale).round() as i32)
            .min(rect.bottom - rect.top)
            .max(1);
        let x = rect.left + (rect.right - rect.left - width) / 2;
        let y = (rect.bottom - height - inset).max(rect.top);
        let next = (x, y, width, height);
        if hover.rect != Some(next) || IsWindowVisible(overlay) == 0 {
            if SetWindowPos(
                overlay,
                HWND_TOP,
                x,
                y,
                width,
                height,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            ) == 0
            {
                return Err("Impossibile posizionare i controlli video".into());
            }
            hover.rect = Some(next);
        }
    }
    Ok(())
}
