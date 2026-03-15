use bevy::prelude::*;
use std::borrow::Cow;
use wry::WebView;
use crate::embedded_assets;
use crate::ipc::{IpcChannelSender, JsToRust, RustToJs, WindowControlEvent};

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

/// Marker component for editor windows.
#[derive(Component)]
pub struct EditorWindow;

/// NonSend resource holding the editor webview and native window.
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
}

/// Tracks whether the editor window is currently maximized.
#[derive(Resource, Default)]
pub struct MaximizedState {
    pub maximized: bool,
}

#[derive(Event)]
pub struct OpenEditorEvent;

#[derive(Event)]
pub struct CloseEditorEvent;

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
    mut events: EventReader<OpenEditorEvent>,
    mut webview_manager: NonSendMut<WebViewManager>,
    ipc_sender: NonSend<IpcChannelSender>,
) {
    for _ in events.read() {
        if webview_manager.webview.is_some() {
            continue;
        }

        #[cfg(target_os = "macos")]
        {
            let mtm = MainThreadMarker::new().expect("must be on main thread");

            // Create native NSWindow with transparent titlebar (decorations: false equivalent)
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

            ns_window.setTitle(&NSString::from_str("VOID//SCRIPT Editor"));
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

            let tx = ipc_sender.0.clone();
            let ns_window_for_ipc = ns_window.clone();

            #[cfg(target_os = "macos")]
            use wry::WebViewBuilderExtDarwin;

            let builder = wry::WebViewBuilder::new()
                .with_traffic_light_inset(wry::dpi::LogicalPosition::new(12.0, 22.0))
                .with_custom_protocol("voidscript".into(), |_webview_id, request| {
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
                .with_url("voidscript://localhost/index.html")
                .with_accept_first_mouse(true)
                .with_ipc_handler(move |request| {
                    let body = request.body();
                    match serde_json::from_str::<JsToRust>(body) {
                        Ok(JsToRust::WindowDrag { delta_x, delta_y }) => {
                            // Handle drag directly in IPC handler for low latency
                            let frame = ns_window_for_ipc.frame();
                            let origin = NSPoint::new(
                                frame.origin.x + delta_x,
                                frame.origin.y - delta_y,
                            );
                            ns_window_for_ipc.setFrameOrigin(origin);
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
                    info!("Editor window created with native NSWindow + WebView");
                }
                Err(e) => {
                    error!("Failed to create webview: {e}");
                }
            }
        }
    }
}

pub fn handle_close_editor(
    mut close_events: EventReader<CloseEditorEvent>,
    mut webview_manager: NonSendMut<WebViewManager>,
) {
    for _ in close_events.read() {
        webview_manager.webview = None;
        #[cfg(target_os = "macos")]
        {
            if let Some(ns_window) = webview_manager.ns_window.take() {
                ns_window.orderOut(None);
            }
        }
    }
}

pub fn handle_window_controls(
    mut events: EventReader<WindowControlEvent>,
    mut webview_manager: NonSendMut<WebViewManager>,
    mut maximized_state: ResMut<MaximizedState>,
) {
    for event in events.read() {
        #[cfg(target_os = "macos")]
        {
            let Some(ns_window) = &webview_manager.ns_window else { continue };
            match event {
                WindowControlEvent::Minimize => {
                    ns_window.miniaturize(None);
                }
                WindowControlEvent::Maximize => {
                    maximized_state.maximized = !maximized_state.maximized;
                    ns_window.zoom(None);
                }
                WindowControlEvent::Close => {
                    webview_manager.webview = None;
                    if let Some(w) = webview_manager.ns_window.take() {
                        w.orderOut(None);
                    }
                }
            }
        }
    }
}

/// Detects when the native macOS close button was clicked (bypassing our IPC)
/// and cleans up editor resources.
pub fn detect_native_close(
    mut webview_manager: NonSendMut<WebViewManager>,
) {
    #[cfg(target_os = "macos")]
    {
        if let Some(ns_window) = &webview_manager.ns_window {
            // Window is closed (not visible) but not minimized → native close was used
            if !ns_window.isVisible() && !ns_window.isMiniaturized() {
                webview_manager.webview = None;
                webview_manager.ns_window = None;
            }
        }
    }
}

/// Clean up native window resources before Bevy exits to prevent crash on shutdown.
pub fn cleanup_on_exit(
    exit_events: EventReader<AppExit>,
    mut webview_manager: NonSendMut<WebViewManager>,
) {
    if !exit_events.is_empty() {
        // Drop webview before window to ensure clean teardown
        webview_manager.webview = None;
        #[cfg(target_os = "macos")]
        {
            webview_manager.ns_window = None;
        }
        // Force clean exit to avoid crash during resource teardown
        std::process::exit(0);
    }
}
