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

/// Holds the editor webview and native window.
pub struct WebViewManager {
    pub webview: Option<WebView>,
    #[cfg(target_os = "macos")]
    pub ns_window: Option<Retained<NSWindow>>,
}

impl Default for WebViewManager {
    fn default() -> Self {
        Self {
            webview: None,
            #[cfg(target_os = "macos")]
            ns_window: None,
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
    }

    /// Detects when the native macOS close button was clicked (bypassing our IPC)
    /// and cleans up editor resources.
    pub fn detect_native_close(&mut self) {
        #[cfg(target_os = "macos")]
        {
            if let Some(ns_window) = &self.ns_window {
                // Window is closed (not visible) but not minimized → native close was used
                if !ns_window.isVisible() && !ns_window.isMiniaturized() {
                    self.webview = None;
                    self.ns_window = None;
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
    }
}

#[derive(Default)]
pub struct MaximizedState {
    pub maximized: bool,
}

/// Wrapper to pass a native NSView as a HasWindowHandle to wry.
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

/// Creates a native NSWindow and attaches a wry WebView to it.
pub fn open_editor(
    webview_manager: &mut WebViewManager,
    ipc_sender: &Sender<JsToRust>,
) {
    if webview_manager.webview.is_some() {
        return;
    }

    #[cfg(target_os = "macos")]
    {
        let mtm = MainThreadMarker::new().expect("must be on main thread");

        // Create native NSWindow with transparent titlebar
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

        // Prevent macOS from releasing the window when the native close button
        // is clicked — our Retained handle manages the lifetime instead.
        unsafe { ns_window.setReleasedWhenClosed(false) };

        ns_window.setTitle(&NSString::from_str("DEADCODE Editor"));
        ns_window.setTitlebarAppearsTransparent(true);
        ns_window.setTitleVisibility(objc2_app_kit::NSWindowTitleVisibility::Hidden);
        ns_window.center();
        ns_window.setMovableByWindowBackground(true);

        // Get content view handle for wry
        let content_view = ns_window.contentView().expect("window has content view");
        let view_ptr = Retained::as_ptr(&content_view) as *mut std::ffi::c_void;
        let handle = NativeViewHandle {
            ns_view: NonNull::new(view_ptr).unwrap(),
        };

        let tx = ipc_sender.clone();
        let ns_window_for_ipc = ns_window.clone();

        #[cfg(target_os = "macos")]
        use wry::WebViewBuilderExtDarwin;

        let builder = wry::WebViewBuilder::new()
            .with_traffic_light_inset(wry::dpi::LogicalPosition::new(12.0, 22.0))
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
                        // Start native window drag — macOS handles the rest
                        let mtm = MainThreadMarker::new().unwrap();
                        let app = objc2_app_kit::NSApplication::sharedApplication(mtm);
                        if let Some(event) = app.currentEvent() {
                            ns_window_for_ipc.performWindowDragWithEvent(&event);
                        }
                    }
                    Ok(parsed) => {
                        let _ = tx.send(parsed);
                    }
                    Err(e) => eprintln!("IPC parse error: {e}"),
                }
            });

        match builder.build(&handle) {
            Ok(webview) => {
                // Show window after webview is ready
                ns_window.makeKeyAndOrderFront(None);
                webview_manager.webview = Some(webview);
                webview_manager.ns_window = Some(ns_window);
                eprintln!("Editor window created with native NSWindow + WebView");
            }
            Err(e) => {
                eprintln!("Failed to create webview: {e}");
            }
        }
    }
}
