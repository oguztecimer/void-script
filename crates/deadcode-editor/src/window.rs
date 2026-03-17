use std::borrow::Cow;
use std::time::Instant;
use crossbeam_channel::Sender;
use wry::WebView;
use crate::embedded_assets;
use crate::ipc::{JsToRust, RustToJs, WindowControlEvent};

pub const MIN_WINDOW_WIDTH: i32 = 400;
pub const MIN_WINDOW_HEIGHT: i32 = 400;

#[cfg(target_os = "macos")]
use objc2::{rc::Retained, MainThreadOnly};
#[cfg(target_os = "macos")]
use objc2_app_kit::{NSWindow, NSWindowStyleMask, NSBackingStoreType};
#[cfg(target_os = "macos")]
use objc2_foundation::{NSRect, NSPoint, NSSize, NSString, MainThreadMarker};
#[cfg(target_os = "macos")]
use raw_window_handle::{AppKitWindowHandle, HasWindowHandle, RawWindowHandle, WindowHandle};
#[cfg(target_os = "macos")]
use std::ptr::NonNull;

#[cfg(target_os = "windows")]
use raw_window_handle::{HasWindowHandle, RawWindowHandle, Win32WindowHandle, WindowHandle};
#[cfg(target_os = "windows")]
use std::num::NonZeroIsize;

/// Holds the editor webview and native window.
pub struct WebViewManager {
    pub webview: Option<WebView>,
    #[cfg(target_os = "macos")]
    pub ns_window: Option<Retained<NSWindow>>,
    #[cfg(target_os = "windows")]
    pub hwnd: Option<isize>,
    /// Whether the editor window has been shown at least once.
    /// Used to avoid detect_native_close destroying the window before it loads.
    shown_once: bool,
    /// Window shake animation state.
    shake_start: Option<Instant>,
    shake_origin: Option<(f64, f64)>,
}

impl Default for WebViewManager {
    fn default() -> Self {
        Self {
            webview: None,
            #[cfg(target_os = "macos")]
            ns_window: None,
            #[cfg(target_os = "windows")]
            hwnd: None,
            shown_once: false,
            shake_start: None,
            shake_origin: None,
        }
    }
}

impl WebViewManager {
    pub fn send_to_all(&self, msg: &RustToJs) {
        let Some(webview) = &self.webview else { return };
        let json = serde_json::to_string(msg).expect("serialize IPC");
        let js = format!("window.__IPC_RECEIVE({})", json);
        let _ = webview.evaluate_script(&js);
    }

    pub fn show(&mut self) {
        self.shown_once = true;
        #[cfg(target_os = "macos")]
        {
            if let Some(ns_window) = &self.ns_window {
                ns_window.setAlphaValue(1.0);
                ns_window.makeKeyAndOrderFront(None);
            }
        }
        #[cfg(target_os = "windows")]
        {
            if let Some(hwnd) = self.hwnd {
                use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SetForegroundWindow, SW_SHOW};
                use windows::Win32::Foundation::HWND;
                unsafe {
                    let h = HWND(hwnd as *mut _);
                    ShowWindow(h, SW_SHOW);
                    let _ = SetForegroundWindow(h);
                }
            }
        }
    }

    pub fn is_open(&self) -> bool {
        self.webview.is_some()
    }

    /// Returns true if the editor window exists AND is visible on screen.
    pub fn is_visible(&self) -> bool {
        if self.webview.is_none() {
            return false;
        }
        #[cfg(target_os = "windows")]
        {
            if let Some(hwnd) = self.hwnd {
                use windows::Win32::UI::WindowsAndMessaging::IsWindowVisible;
                use windows::Win32::Foundation::HWND;
                return unsafe { IsWindowVisible(HWND(hwnd as *mut _)).as_bool() };
            }
            return false;
        }
        #[cfg(target_os = "macos")]
        {
            if let Some(ns_window) = &self.ns_window {
                return ns_window.isVisible();
            }
            return false;
        }
    }

    /// Resize the window and optionally lock/unlock resizing.
    pub fn set_size(&mut self, width: u32, height: u32, resizable: bool) {
        // Cancel any active shake so it doesn't restore the old position
        self.shake_start = None;
        self.shake_origin = None;
        #[cfg(target_os = "macos")]
        {
            if let Some(ns_window) = &self.ns_window {
                // Center on screen
                let screen_frame = objc2_app_kit::NSScreen::mainScreen(
                    MainThreadMarker::new().expect("must be on main thread"),
                )
                .map(|s| s.visibleFrame())
                .unwrap_or(NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1920.0, 1080.0)));
                let new_frame = NSRect::new(
                    NSPoint::new(
                        screen_frame.origin.x + (screen_frame.size.width - width as f64) / 2.0,
                        screen_frame.origin.y + (screen_frame.size.height - height as f64) / 2.0,
                    ),
                    NSSize::new(width as f64, height as f64),
                );
                // Unlock constraints BEFORE resizing so the frame isn't clamped
                if resizable {
                    ns_window.setMinSize(NSSize::new(MIN_WINDOW_WIDTH as f64, MIN_WINDOW_HEIGHT as f64));
                    ns_window.setMaxSize(NSSize::new(f64::MAX, f64::MAX));
                } else {
                    ns_window.setMinSize(NSSize::new(width as f64, height as f64));
                    ns_window.setMaxSize(NSSize::new(width as f64, height as f64));
                }
                ns_window.setFrame_display_animate(new_frame, true, true);
            }
        }
        #[cfg(target_os = "windows")]
        {
            if let Some(hwnd) = self.hwnd {
                use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN, SWP_NOZORDER, SWP_NOACTIVATE};
                use windows::Win32::Foundation::HWND;
                unsafe {
                    let h = HWND(hwnd as *mut _);
                    let sw = GetSystemMetrics(SM_CXSCREEN);
                    let sh = GetSystemMetrics(SM_CYSCREEN);
                    let _ = SetWindowPos(
                        h,
                        HWND::default(),
                        (sw - width as i32) / 2,
                        (sh - height as i32) / 2,
                        width as i32,
                        height as i32,
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                }
                // Note: On Windows, min/max size is enforced via WM_GETMINMAXINFO in wndproc.
                // A full implementation would store resizable state and use it there.
            }
        }
    }

    pub fn close(&mut self) {
        self.webview = None;
        self.shown_once = false;
        #[cfg(target_os = "macos")]
        {
            if let Some(ns_window) = self.ns_window.take() {
                ns_window.orderOut(None);
            }
        }
        #[cfg(target_os = "windows")]
        {
            if let Some(hwnd) = self.hwnd.take() {
                use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
                use windows::Win32::Foundation::HWND;
                unsafe {
                    ShowWindow(HWND(hwnd as *mut _), SW_HIDE);
                }
            }
        }
    }

    pub fn handle_window_control(&mut self, event: WindowControlEvent, maximized: &mut bool) {
        #[cfg(target_os = "macos")]
        {
            let Some(ns_window) = &self.ns_window else { return };
            match event {
                WindowControlEvent::Minimize => {
                    ns_window.miniaturize(None);
                }
                WindowControlEvent::Maximize => {
                    *maximized = !*maximized;
                    ns_window.zoom(None);
                }
                WindowControlEvent::Close => {
                    self.webview = None;
                    self.shown_once = false;
                    if let Some(w) = self.ns_window.take() {
                        w.orderOut(None);
                    }
                }
            }
        }
        #[cfg(target_os = "windows")]
        {
            let Some(hwnd) = self.hwnd else { return };
            use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_MINIMIZE, SW_MAXIMIZE, SW_RESTORE, SW_HIDE};
            use windows::Win32::Foundation::HWND;
            let h = HWND(hwnd as *mut _);
            match event {
                WindowControlEvent::Minimize => unsafe {
                    ShowWindow(h, SW_MINIMIZE);
                },
                WindowControlEvent::Maximize => unsafe {
                    *maximized = !*maximized;
                    ShowWindow(h, if *maximized { SW_MAXIMIZE } else { SW_RESTORE });
                },
                WindowControlEvent::Close => {
                    self.webview = None;
                    self.shown_once = false;
                    unsafe { ShowWindow(h, SW_HIDE); }
                    self.hwnd = None;
                },
            }
        }
    }

    /// Start a window shake animation.
    pub fn start_shake(&mut self) {
        if self.shake_start.is_some() {
            return; // already shaking
        }
        // Capture the current origin so we can restore it
        let origin = self.get_window_origin();
        self.shake_origin = origin;
        self.shake_start = Some(Instant::now());
    }

    /// Tick the shake animation. Call every frame. Returns true while shaking.
    pub fn tick_shake(&mut self) -> bool {
        let Some(start) = self.shake_start else { return false };
        let Some((ox, oy)) = self.shake_origin else {
            self.shake_start = None;
            return false;
        };

        let elapsed = start.elapsed().as_secs_f64();
        let duration = 0.35;

        if elapsed >= duration {
            // Restore original position
            self.set_window_origin(ox, oy);
            self.shake_start = None;
            self.shake_origin = None;
            return false;
        }

        // Damped random-ish shake using sine waves at different frequencies
        let t = elapsed / duration;
        let decay = 1.0 - t;
        let intensity = 6.0 * decay;
        let dx = intensity * (t * 73.0).sin() + (intensity * 0.5) * (t * 137.0).cos();
        let dy = intensity * (t * 97.0).cos() + (intensity * 0.5) * (t * 163.0).sin();
        self.set_window_origin(ox + dx, oy + dy);
        true
    }

    fn get_window_origin(&self) -> Option<(f64, f64)> {
        #[cfg(target_os = "macos")]
        {
            let ns_window = self.ns_window.as_ref()?;
            let frame = ns_window.frame();
            Some((frame.origin.x, frame.origin.y))
        }
        #[cfg(target_os = "windows")]
        {
            let hwnd = self.hwnd?;
            use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;
            use windows::Win32::Foundation::{HWND, RECT};
            let mut rect = RECT::default();
            unsafe { GetWindowRect(HWND(hwnd as *mut _), &mut rect).ok()? };
            Some((rect.left as f64, rect.top as f64))
        }
    }

    fn set_window_origin(&self, x: f64, y: f64) {
        #[cfg(target_os = "macos")]
        {
            if let Some(ns_window) = &self.ns_window {
                let frame = ns_window.frame();
                let new_origin = NSPoint::new(x, y);
                let new_frame = NSRect::new(new_origin, frame.size);
                ns_window.setFrame_display(new_frame, true);
            }
        }
        #[cfg(target_os = "windows")]
        {
            if let Some(hwnd) = self.hwnd {
                use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, SWP_NOSIZE, SWP_NOZORDER, SWP_NOACTIVATE};
                use windows::Win32::Foundation::HWND;
                unsafe {
                    let _ = SetWindowPos(
                        HWND(hwnd as *mut _),
                        HWND::default(),
                        x as i32, y as i32,
                        0, 0,
                        SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                }
            }
        }
    }

    /// Detects when the native close button was clicked (bypassing our IPC)
    /// and cleans up editor resources. Returns the window geometry if a close
    /// was detected (so the caller can persist it).
    pub fn detect_native_close(&mut self) -> Option<(i32, i32, i32, i32)> {
        if !self.shown_once {
            return None;
        }
        #[cfg(target_os = "macos")]
        {
            if let Some(ns_window) = &self.ns_window {
                if !ns_window.isVisible() && !ns_window.isMiniaturized() {
                    let frame = ns_window.frame();
                    let geometry = (
                        frame.origin.x as i32,
                        frame.origin.y as i32,
                        frame.size.width as i32,
                        frame.size.height as i32,
                    );
                    self.webview = None;
                    self.ns_window = None;
                    self.shown_once = false;
                    return Some(geometry);
                }
            }
        }
        #[cfg(target_os = "windows")]
        {
            if let Some(hwnd) = self.hwnd {
                use windows::Win32::UI::WindowsAndMessaging::IsWindowVisible;
                use windows::Win32::Foundation::HWND;
                unsafe {
                    if !IsWindowVisible(HWND(hwnd as *mut _)).as_bool() {
                        self.webview = None;
                        self.hwnd = None;
                        self.shown_once = false;
                    }
                }
            }
        }
        None
    }

    pub fn cleanup(&mut self) {
        self.webview = None;
        self.shown_once = false;
        #[cfg(target_os = "macos")]
        {
            self.ns_window = None;
        }
        #[cfg(target_os = "windows")]
        {
            if let Some(hwnd) = self.hwnd.take() {
                use windows::Win32::UI::WindowsAndMessaging::DestroyWindow;
                use windows::Win32::Foundation::HWND;
                unsafe { let _ = DestroyWindow(HWND(hwnd as *mut _)); }
            }
        }
    }
}

#[derive(Default)]
pub struct MaximizedState {
    pub maximized: bool,
}

// ---------------------------------------------------------------------------
// macOS: NativeViewHandle
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
struct NativeViewHandle {
    ns_view: NonNull<std::ffi::c_void>,
}

#[cfg(target_os = "macos")]
impl HasWindowHandle for NativeViewHandle {
    fn window_handle(&self) -> Result<WindowHandle<'_>, raw_window_handle::HandleError> {
        let handle = AppKitWindowHandle::new(self.ns_view);
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::AppKit(handle)) })
    }
}

// ---------------------------------------------------------------------------
// Windows: HwndHandle
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
struct HwndHandle {
    hwnd: NonZeroIsize,
}

#[cfg(target_os = "windows")]
impl HasWindowHandle for HwndHandle {
    fn window_handle(&self) -> Result<WindowHandle<'_>, raw_window_handle::HandleError> {
        let handle = Win32WindowHandle::new(self.hwnd);
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::Win32(handle)) })
    }
}

// ---------------------------------------------------------------------------
// Shared: webview builder (protocol + IPC, no platform specifics)
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn build_webview_common(
    ipc_sender: &Sender<JsToRust>,
    hwnd_val: isize,
) -> wry::WebViewBuilder<'static> {
    let tx = ipc_sender.clone();

    wry::WebViewBuilder::new()
        .with_background_color((0x1E, 0x1F, 0x22, 0xFF))
        .with_custom_protocol("deadcode".into(), |_webview_id, request| {
            let path = request.uri().path().to_string();
            match embedded_assets::get_asset(&path) {
                Some((body, mime)) => http::Response::builder()
                    .header("Content-Type", mime)
                    .header("Access-Control-Allow-Origin", "*")
                    .body(Cow::Owned(body))
                    .unwrap(),
                None => http::Response::builder()
                    .status(404)
                    .body(Cow::Borrowed(b"Not Found" as &[u8]))
                    .unwrap(),
            }
        })
        .with_url("deadcode://localhost/index.html")
        .with_ipc_handler(move |request| {
            let body = request.body();
            match serde_json::from_str::<JsToRust>(body) {
                Ok(JsToRust::WindowDragStart) => {
                    // Initiate native window drag on Windows.
                    use windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture;
                    use windows::Win32::UI::WindowsAndMessaging::*;
                    use windows::Win32::Foundation::{HWND, WPARAM, LPARAM};
                    unsafe {
                        let h = HWND(hwnd_val as *mut _);
                        let _ = ReleaseCapture();
                        SendMessageW(h, WM_NCLBUTTONDOWN, WPARAM(HTCAPTION as usize), LPARAM(0));
                    }
                }
                Ok(JsToRust::WindowResizeStart { direction }) => {
                    use windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture;
                    use windows::Win32::UI::WindowsAndMessaging::*;
                    use windows::Win32::Foundation::{HWND, WPARAM, LPARAM};
                    let hit = match direction.as_str() {
                        "n"  => HTTOP,
                        "s"  => HTBOTTOM,
                        "w"  => HTLEFT,
                        "e"  => HTRIGHT,
                        "nw" => HTTOPLEFT,
                        "ne" => HTTOPRIGHT,
                        "sw" => HTBOTTOMLEFT,
                        "se" => HTBOTTOMRIGHT,
                        _ => return,
                    };
                    unsafe {
                        let h = HWND(hwnd_val as *mut _);
                        let _ = ReleaseCapture();
                        SendMessageW(h, WM_NCLBUTTONDOWN, WPARAM(hit as usize), LPARAM(0));
                    }
                }
                Ok(parsed @ JsToRust::WindowClose) => {
                    let _ = tx.send(parsed);
                }
                Ok(parsed) => {
                    let _ = tx.send(parsed);
                }
                Err(e) => eprintln!("IPC parse error: {e}"),
            }
        })
}

// ---------------------------------------------------------------------------
// open_editor: platform dispatch
// ---------------------------------------------------------------------------

/// Open the editor window with optional saved geometry `(x, y, width, height)`.
pub fn open_editor(
    webview_manager: &mut WebViewManager,
    ipc_sender: &Sender<JsToRust>,
    saved_geometry: Option<(i32, i32, i32, i32)>,
) {
    if webview_manager.webview.is_some() {
        return;
    }

    #[cfg(target_os = "macos")]
    open_editor_macos(webview_manager, ipc_sender, saved_geometry);

    #[cfg(target_os = "windows")]
    open_editor_windows(webview_manager, ipc_sender, saved_geometry);
}

/// Query the current window geometry `(x, y, width, height)` from the native window.
pub fn get_window_geometry(webview_manager: &WebViewManager) -> Option<(i32, i32, i32, i32)> {
    #[cfg(target_os = "windows")]
    {
        let hwnd = webview_manager.hwnd?;
        use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;
        use windows::Win32::Foundation::{HWND, RECT};
        let mut rect = RECT::default();
        unsafe { GetWindowRect(HWND(hwnd as *mut _), &mut rect).ok()? };
        Some((rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top))
    }
    #[cfg(target_os = "macos")]
    {
        let ns_window = webview_manager.ns_window.as_ref()?;
        let frame = ns_window.frame();
        Some((
            frame.origin.x as i32,
            frame.origin.y as i32,
            frame.size.width as i32,
            frame.size.height as i32,
        ))
    }
}

// ---------------------------------------------------------------------------
// macOS implementation
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
fn open_editor_macos(
    webview_manager: &mut WebViewManager,
    ipc_sender: &Sender<JsToRust>,
    saved_geometry: Option<(i32, i32, i32, i32)>,
) {
    let mtm = MainThreadMarker::new().expect("must be on main thread");

    let frame = if let Some((x, y, w, h)) = saved_geometry {
        NSRect::new(
            NSPoint::new(x as f64, y as f64),
            NSSize::new(
                w.max(MIN_WINDOW_WIDTH) as f64,
                h.max(MIN_WINDOW_HEIGHT) as f64,
            ),
        )
    } else {
        // Default to 70% of screen size, centered
        let screen_frame = objc2_app_kit::NSScreen::mainScreen(mtm)
            .map(|s| s.frame())
            .unwrap_or(NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1920.0, 1080.0)));
        let w = screen_frame.size.width * 0.7;
        let h = screen_frame.size.height * 0.7;
        NSRect::new(
            NSPoint::new(
                screen_frame.origin.x + (screen_frame.size.width - w) / 2.0,
                screen_frame.origin.y + (screen_frame.size.height - h) / 2.0,
            ),
            NSSize::new(w, h),
        )
    };
    let style = NSWindowStyleMask::Titled
        | NSWindowStyleMask::Closable
        | NSWindowStyleMask::Miniaturizable
        | NSWindowStyleMask::Resizable
        | NSWindowStyleMask::FullSizeContentView;

    let ns_window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            frame,
            style,
            NSBackingStoreType::Buffered,
            false,
        )
    };

    unsafe { ns_window.setReleasedWhenClosed(false) };

    // Enforce minimum window size
    ns_window.setMinSize(NSSize::new(MIN_WINDOW_WIDTH as f64, MIN_WINDOW_HEIGHT as f64));

    ns_window.setTitle(&NSString::from_str("DEADCODE Editor"));
    ns_window.setTitlebarAppearsTransparent(true);
    ns_window.setTitleVisibility(objc2_app_kit::NSWindowTitleVisibility::Hidden);
    if saved_geometry.is_none() {
        ns_window.center();
    }
    ns_window.setMovableByWindowBackground(true);

    let bg = objc2_app_kit::NSColor::colorWithSRGBRed_green_blue_alpha(
        0x1E as f64 / 255.0,
        0x1F as f64 / 255.0,
        0x22 as f64 / 255.0,
        1.0,
    );
    ns_window.setBackgroundColor(Some(&bg));

    let content_view = ns_window.contentView().expect("window has content view");
    let view_ptr = Retained::as_ptr(&content_view) as *mut std::ffi::c_void;
    let handle = NativeViewHandle {
        ns_view: NonNull::new(view_ptr).unwrap(),
    };

    let ns_window_for_ipc = ns_window.clone();
    let tx_drag = ipc_sender.clone();

    use wry::WebViewBuilderExtDarwin;

    let builder = wry::WebViewBuilder::new()
        .with_traffic_light_inset(wry::dpi::LogicalPosition::new(12.0, 22.0))
        .with_background_color((0x1E, 0x1F, 0x22, 0xFF))
        .with_custom_protocol("deadcode".into(), |_webview_id, request| {
            let path = request.uri().path().to_string();
            match embedded_assets::get_asset(&path) {
                Some((body, mime)) => http::Response::builder()
                    .header("Content-Type", mime)
                    .header("Access-Control-Allow-Origin", "*")
                    .body(Cow::Owned(body))
                    .unwrap(),
                None => http::Response::builder()
                    .status(404)
                    .body(Cow::Borrowed(b"Not Found" as &[u8]))
                    .unwrap(),
            }
        })
        .with_url("deadcode://localhost/index.html")
        .with_accept_first_mouse(true)
        .with_ipc_handler(move |request| {
            let body = request.body();
            match serde_json::from_str::<JsToRust>(body) {
                Ok(JsToRust::WindowDragStart) => {
                    let mtm = MainThreadMarker::new().unwrap();
                    let app = objc2_app_kit::NSApplication::sharedApplication(mtm);
                    if let Some(event) = app.currentEvent() {
                        ns_window_for_ipc.performWindowDragWithEvent(&event);
                    }
                }
                Ok(parsed) => {
                    let _ = tx_drag.send(parsed);
                }
                Err(e) => eprintln!("IPC parse error: {e}"),
            }
        });

    match builder.build(&handle) {
        Ok(webview) => {
            ns_window.setAlphaValue(0.0);
            ns_window.makeKeyAndOrderFront(None);
            webview_manager.webview = Some(webview);
            webview_manager.ns_window = Some(ns_window);
        }
        Err(e) => {
            eprintln!("Failed to create webview: {e}");
        }
    }
}

// ---------------------------------------------------------------------------
// Windows implementation
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn open_editor_windows(
    webview_manager: &mut WebViewManager,
    ipc_sender: &Sender<JsToRust>,
    saved_geometry: Option<(i32, i32, i32, i32)>,
) {
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::Win32::Graphics::Gdi::CreateSolidBrush;
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::core::w;

    unsafe {
        let hinstance = GetModuleHandleW(None).unwrap_or_default();

        // Register window class
        let class_name = w!("DeadcodeEditor");
        let hinstance_handle: windows::Win32::Foundation::HINSTANCE = hinstance.into();
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(wndproc),
            hInstance: hinstance_handle,
            lpszClassName: class_name,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            hbrBackground: CreateSolidBrush(windows::Win32::Foundation::COLORREF(0x00221F1E)),
            ..Default::default()
        };
        RegisterClassExW(&wc);

        // Use saved geometry or center on screen with default size
        let (x, y, win_w, win_h) = if let Some((gx, gy, gw, gh)) = saved_geometry {
            (gx, gy, gw.max(MIN_WINDOW_WIDTH), gh.max(MIN_WINDOW_HEIGHT))
        } else {
            let screen_w = GetSystemMetrics(SM_CXSCREEN);
            let screen_h = GetSystemMetrics(SM_CYSCREEN);
            let w = (screen_w as f64 * 0.7) as i32;
            let h = (screen_h as f64 * 0.7) as i32;
            ((screen_w - w) / 2, (screen_h - h) / 2, w, h)
        };

        // WS_POPUP + WS_THICKFRAME: borderless but resizable.
        // WM_NCCALCSIZE returns 0 to remove the visible frame.
        // Child windows are inset by BORDER_WIDTH so the parent handles edge mouse events.
        let style = WS_POPUP | WS_THICKFRAME | WS_SYSMENU | WS_MINIMIZEBOX | WS_MAXIMIZEBOX;

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class_name,
            w!("DEADCODE Editor"),
            style,
            x, y, win_w, win_h,
            None,
            None,
            hinstance_handle,
            None,
        ).unwrap();

        // Request rounded corners on Windows 11.
        use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE};
        let preference = 2u32; // DWMWCP_ROUND
        let _ = unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &preference as *const u32 as *const _,
                std::mem::size_of::<u32>() as u32,
            )
        };

        let hwnd_val = hwnd.0 as isize;
        let handle = HwndHandle {
            hwnd: NonZeroIsize::new(hwnd_val).unwrap(),
        };

        eprintln!("[deadcode] Created HWND: {:?}", hwnd);

        let builder = build_webview_common(ipc_sender, hwnd_val);

        match builder.build(&handle) {
            Ok(webview) => {
                eprintln!("[deadcode] WebView created successfully");
                // Don't show yet — wait for EditorReady.
                webview_manager.webview = Some(webview);
                webview_manager.hwnd = Some(hwnd_val);
            }
            Err(e) => {
                eprintln!("[deadcode] Failed to create webview: {e}");
                let _ = DestroyWindow(hwnd);
            }
        }
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn wndproc(
    hwnd: windows::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::Win32::Foundation::LRESULT;

    match msg {
        WM_NCCALCSIZE => {
            // Return 0 to remove the non-client area (no visible titlebar/border).
            LRESULT(0)
        }
        WM_GETMINMAXINFO => {
            let info = unsafe { &mut *(lparam.0 as *mut MINMAXINFO) };
            info.ptMinTrackSize.x = MIN_WINDOW_WIDTH;
            info.ptMinTrackSize.y = MIN_WINDOW_HEIGHT;
            LRESULT(0)
        }
        WM_CLOSE => {
            // Hide instead of destroy so the editor HWND stays alive as the
            // app's "main window", preventing strip windows from appearing
            // in the taskbar. The IPC channel will handle full cleanup.
            unsafe { ShowWindow(hwnd, SW_HIDE); }
            LRESULT(0)
        }
        WM_DESTROY => {
            // Don't PostQuitMessage — the winit event loop manages app lifetime.
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
