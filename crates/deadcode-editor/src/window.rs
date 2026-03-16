use std::borrow::Cow;
use crossbeam_channel::Sender;
use wry::WebView;
use crate::embedded_assets;
use crate::ipc::{JsToRust, RustToJs, WindowControlEvent};

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
}

impl Default for WebViewManager {
    fn default() -> Self {
        Self {
            webview: None,
            #[cfg(target_os = "macos")]
            ns_window: None,
            #[cfg(target_os = "windows")]
            hwnd: None,
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

    pub fn show(&self) {
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

    pub fn close(&mut self) {
        self.webview = None;
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
                    unsafe { ShowWindow(h, SW_HIDE); }
                    self.hwnd = None;
                },
            }
        }
    }

    /// Detects when the native close button was clicked (bypassing our IPC)
    /// and cleans up editor resources.
    pub fn detect_native_close(&mut self) {
        #[cfg(target_os = "macos")]
        {
            if let Some(ns_window) = &self.ns_window {
                if !ns_window.isVisible() && !ns_window.isMiniaturized() {
                    self.webview = None;
                    self.ns_window = None;
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
                    }
                }
            }
        }
    }

    pub fn cleanup(&mut self) {
        self.webview = None;
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

pub fn open_editor(
    webview_manager: &mut WebViewManager,
    ipc_sender: &Sender<JsToRust>,
) {
    if webview_manager.webview.is_some() {
        return;
    }

    #[cfg(target_os = "macos")]
    open_editor_macos(webview_manager, ipc_sender);

    #[cfg(target_os = "windows")]
    open_editor_windows(webview_manager, ipc_sender);
}

// ---------------------------------------------------------------------------
// macOS implementation
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
fn open_editor_macos(
    webview_manager: &mut WebViewManager,
    ipc_sender: &Sender<JsToRust>,
) {
    let mtm = MainThreadMarker::new().expect("must be on main thread");

    let frame = NSRect::new(
        NSPoint::new(100.0, 100.0),
        NSSize::new(1200.0, 800.0),
    );
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

    ns_window.setTitle(&NSString::from_str("DEADCODE Editor"));
    ns_window.setTitlebarAppearsTransparent(true);
    ns_window.setTitleVisibility(objc2_app_kit::NSWindowTitleVisibility::Hidden);
    ns_window.center();
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
) {
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::Win32::Foundation::{HWND, HINSTANCE};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::core::w;

    unsafe {
        let hinstance = GetModuleHandleW(None).unwrap_or_default();

        // Register window class
        let class_name = w!("DeadcodeEditor");
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(wndproc),
            hInstance: hinstance.into(),
            lpszClassName: class_name,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            hbrBackground: CreateSolidBrush(windows::Win32::Foundation::COLORREF(0x00221F1E)),
            ..Default::default()
        };
        RegisterClassExW(&wc);

        // Center the window on screen
        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let win_w = 1200;
        let win_h = 800;
        let x = (screen_w - win_w) / 2;
        let y = (screen_h - win_h) / 2;

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class_name,
            w!("DEADCODE Editor"),
            WS_OVERLAPPEDWINDOW,
            x, y, win_w, win_h,
            None,
            None,
            Some(hinstance.into()),
            None,
        ).unwrap();

        let hwnd_val = hwnd.0 as isize;
        let handle = HwndHandle {
            hwnd: NonZeroIsize::new(hwnd_val).unwrap(),
        };

        let builder = build_webview_common(ipc_sender);

        match builder.build(&handle) {
            Ok(webview) => {
                // Don't show yet — wait for EditorReady.
                webview_manager.webview = Some(webview);
                webview_manager.hwnd = Some(hwnd_val);
            }
            Err(e) => {
                eprintln!("Failed to create webview: {e}");
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
        WM_DESTROY => {
            // Don't PostQuitMessage — the winit event loop manages app lifetime.
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
