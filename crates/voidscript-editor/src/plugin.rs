use bevy::prelude::*;
use crossbeam_channel::unbounded;

use crate::ipc::*;
use crate::scripts::*;
use crate::tabs::*;
use crate::window::*;
use crate::execution;

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        let (sender, receiver) = unbounded::<JsToRust>();

        let scripts_dir = std::env::current_dir()
            .unwrap_or_default()
            .join("scripts");

        app.insert_resource(IpcChannelReceiver(receiver))
            .insert_non_send_resource(IpcChannelSender(sender))
            .insert_non_send_resource(WebViewManager::default())
            .insert_resource(EditorWindowState::default())
            .insert_resource(MaximizedState::default())
            .insert_resource(ScriptStore::new(scripts_dir))
            .add_event::<OpenEditorEvent>()
            .add_event::<CloseEditorEvent>()
            .add_event::<EditorReadyEvent>()
            .add_event::<ScriptSaveEvent>()
            .add_event::<ScriptRequestEvent>()
            .add_event::<ScriptListRequestEvent>()
            .add_event::<TabChangedEvent>()
            .add_event::<RunScriptEvent>()
            .add_event::<StopScriptEvent>()
            .add_event::<DebugStartEvent>()
            .add_event::<DebugContinueEvent>()
            .add_event::<DebugStepOverEvent>()
            .add_event::<DebugStepIntoEvent>()
            .add_event::<DebugStepOutEvent>()
            .add_event::<ToggleBreakpointEvent>()
            .add_event::<WindowControlEvent>()
            .insert_resource(execution::ScriptExecutionManager::default())
            .add_systems(
                Update,
                (
                    poll_ipc_messages,
                    create_editor_window,
                    attach_webview,
                    handle_close_editor,
                    handle_window_controls,
                    handle_editor_ready,
                    handle_script_save,
                    handle_script_request,
                    handle_script_list_request,
                ),
            )
            .add_systems(
                Update,
                (
                    execution::handle_run_script,
                    execution::handle_debug_start,
                    execution::handle_stop_script,
                    execution::handle_debug_commands,
                    execution::handle_toggle_breakpoint,
                    execution::poll_script_events,
                ),
            );
    }
}

fn handle_editor_ready(
    mut events: EventReader<EditorReadyEvent>,
    webview_manager: NonSend<WebViewManager>,
    script_store: Res<ScriptStore>,
) {
    for _event in events.read() {
        info!("Editor reported ready");
        let infos = script_store.get_script_infos();
        let msg = RustToJs::ScriptList { scripts: infos };
        webview_manager.send_to_all(&msg);
    }
}

fn handle_script_save(
    mut events: EventReader<ScriptSaveEvent>,
    mut script_store: ResMut<ScriptStore>,
    mut tab_state: ResMut<EditorWindowState>,
) {
    for event in events.read() {
        script_store.save_script(&event.script_id, event.content.clone());
        tab_state.set_modified(&event.script_id, false);
        info!("Script saved: {}", event.script_id);
    }
}

fn handle_script_request(
    mut events: EventReader<ScriptRequestEvent>,
    script_store: Res<ScriptStore>,
    webview_manager: NonSend<WebViewManager>,
    mut tab_state: ResMut<EditorWindowState>,
) {
    for event in events.read() {
        if let Some(script) = script_store.scripts.get(&event.script_id) {
            tab_state.open_tab(script.id.clone(), script.name.clone());
            let msg = RustToJs::ScriptLoad {
                script_id: script.id.clone(),
                name: script.name.clone(),
                content: script.content.clone(),
                script_type: script.script_type.as_str().to_string(),
            };
            webview_manager.send_to_all(&msg);
        }
    }
}

fn handle_script_list_request(
    mut events: EventReader<ScriptListRequestEvent>,
    script_store: Res<ScriptStore>,
    webview_manager: NonSend<WebViewManager>,
) {
    for _event in events.read() {
        let infos = script_store.get_script_infos();
        let msg = RustToJs::ScriptList { scripts: infos };
        webview_manager.send_to_all(&msg);
    }
}
