use bevy::prelude::*;
use bevy::winit::WinitWindows;
use std::borrow::Cow;
use std::collections::HashMap;
use std::ops::Deref;
use wry::WebView;
use crate::embedded_assets;
use crate::ipc::{IpcChannelSender, JsToRust, RustToJs};

/// Marker component for editor windows (not the main game window).
#[derive(Component)]
pub struct EditorWindow;

/// NonSend resource holding active webviews, keyed by window entity.
#[derive(Default)]
pub struct WebViewManager {
    pub webviews: HashMap<Entity, WebView>,
}

impl WebViewManager {
    pub fn send_to_all(&self, msg: &RustToJs) {
        let json = serde_json::to_string(msg).expect("serialize IPC");
        let js = format!("window.__IPC_RECEIVE({})", json);
        for webview in self.webviews.values() {
            let _ = webview.evaluate_script(&js);
        }
    }
}

#[derive(Event)]
pub struct OpenEditorEvent;

#[derive(Event)]
pub struct CloseEditorEvent;

/// Spawns a new Bevy window with the EditorWindow marker.
pub fn create_editor_window(
    mut commands: Commands,
    mut events: EventReader<OpenEditorEvent>,
) {
    for _ in events.read() {
        commands.spawn((
            Window {
                title: "VOID//SCRIPT Editor".to_string(),
                resolution: bevy::window::WindowResolution::new(1200.0, 800.0),
                ..default()
            },
            EditorWindow,
        ));
    }
}

/// Detects editor windows without a webview and attaches one via wry.
pub fn attach_webview(
    editor_windows: Query<Entity, With<EditorWindow>>,
    winit_windows: NonSend<WinitWindows>,
    mut webview_manager: NonSendMut<WebViewManager>,
    ipc_sender: NonSend<IpcChannelSender>,
) {
    for entity in editor_windows.iter() {
        if webview_manager.webviews.contains_key(&entity) {
            continue;
        }
        let Some(winit_window) = winit_windows.get_window(entity) else {
            continue;
        };

        let tx = ipc_sender.0.clone();

        let builder = wry::WebViewBuilder::new()
            .with_custom_protocol("voidscript".into(), |_webview_id, request| {
                let path = request.uri().path().to_string();
                match embedded_assets::get_asset(&path) {
                    Some((body, mime)) => http::Response::builder()
                        .header("Content-Type", mime)
                        .body(Cow::Owned(body))
                        .unwrap(),
                    None => http::Response::builder()
                        .status(404)
                        .body(Cow::Borrowed(b"Not Found" as &[u8]))
                        .unwrap(),
                }
            })
            .with_url("voidscript://localhost/index.html")
            .with_ipc_handler(move |request| {
                let body = request.body();
                match serde_json::from_str::<JsToRust>(body) {
                    Ok(parsed) => {
                        let _ = tx.send(parsed);
                    }
                    Err(e) => eprintln!("IPC parse error: {e}"),
                }
            });

        match builder.build(winit_window.deref()) {
            Ok(webview) => {
                webview_manager.webviews.insert(entity, webview);
                info!("WebView attached to editor window {:?}", entity);
            }
            Err(e) => {
                error!("Failed to create webview: {e}");
            }
        }
    }
}

pub fn handle_close_editor(
    mut close_events: EventReader<CloseEditorEvent>,
    mut webview_manager: NonSendMut<WebViewManager>,
    editor_windows: Query<Entity, With<EditorWindow>>,
    mut commands: Commands,
) {
    for _ in close_events.read() {
        for entity in editor_windows.iter() {
            webview_manager.webviews.remove(&entity);
            commands.entity(entity).despawn();
        }
    }
}
