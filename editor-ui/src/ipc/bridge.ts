import type { RustToJsMessage, JsToRustMessage } from './types';
import { useStore } from '../state/store';

declare global {
  interface Window {
    __IPC_RECEIVE: (msg: RustToJsMessage) => void;
    ipc: { postMessage: (msg: string) => void };
  }
}

export function sendToRust(msg: JsToRustMessage): void {
  if (window.ipc) {
    window.ipc.postMessage(JSON.stringify(msg));
  } else {
    console.log('[IPC Mock] sendToRust:', msg);
  }
}

export function initIpcBridge(): void {
  window.__IPC_RECEIVE = (msg: RustToJsMessage) => {
    const store = useStore.getState();

    switch (msg.type) {
      case 'script_load':
        store.openTab(msg.script_id, msg.name, msg.content, msg.script_type);
        break;
      case 'error_update':
        store.setDiagnostics(msg.script_id, msg.diagnostics);
        break;
      case 'script_list':
        store.setScriptList(msg.scripts);
        break;
      case 'tab_close':
        store.closeTab(msg.script_id);
        break;
      case 'console_output':
        store.addTerminalOutput(msg.text, msg.level);
        break;
      case 'script_started':
        store.setRunning(true);
        store.setBottomPanelOpen(true);
        break;
      case 'script_finished':
        store.setRunning(false);
        store.setDebugging(false);
        store.setPaused(false);
        store.addTerminalOutput(
          msg.success ? '--- Script finished ---' : `--- Script failed: ${msg.error} ---`,
          msg.success ? 'info' : 'error'
        );
        break;
      case 'debug_paused':
        store.setPaused(true);
        store.setDebugging(true);
        store.setDebugLine(msg.line);
        store.setDebugVariables(msg.variables);
        store.setDebugCallStack(msg.call_stack);
        break;
      case 'debug_resumed':
        store.setPaused(false);
        break;
      case 'terminal_finished':
        store.setTerminalBusy(false);
        if (!msg.success && msg.error) {
          store.addTerminalOutput(msg.error, 'error');
        }
        break;
      case 'simulation_started':
        store.addTerminalOutput('--- Simulation started ---', 'info');
        break;
      case 'simulation_stopped':
        store.addTerminalOutput('--- Simulation stopped ---', 'info');
        break;
      case 'simulation_tick':
        // Tick updates can be used by UI components if needed.
        break;
      case 'available_commands':
        store.setAvailableCommands(msg.commands);
        break;
    }
  };

  // Notify Rust that the editor is ready
  sendToRust({ type: 'editor_ready' });

  // Request script list
  sendToRust({ type: 'script_list_request' });
}
