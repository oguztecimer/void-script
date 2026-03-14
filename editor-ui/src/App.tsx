import { useEffect } from 'react';
import { Header } from './components/Header';
import { ToolStrip } from './components/ToolStrip';
import { TabBar } from './components/TabBar';
import { Editor } from './components/Editor';
import { ScriptList } from './components/ScriptList';
import { Console } from './components/Console';
import { DebugPanel } from './components/DebugPanel';
import { StatusBar } from './components/StatusBar';
import { ToolBtn } from './primitives/ToolBtn';
import { initIpcBridge } from './ipc/bridge';
import { useStore } from './state/store';
import styles from './App.module.css';

const LEFT_ITEMS = [
  { id: 'scripts', icon: 'S', label: 'Scripts', shortcut: 'Alt+1' },
];
const RIGHT_ITEMS = [
  { id: 'debug', icon: 'D', label: 'Debug', shortcut: 'Alt+5' },
];

export function App() {
  const leftPanelOpen = useStore((s) => s.leftPanelOpen);
  const bottomPanelOpen = useStore((s) => s.bottomPanelOpen);
  const rightPanelOpen = useStore((s) => s.rightPanelOpen);
  const isDebugging = useStore((s) => s.isDebugging);
  const toggleLeftPanel = useStore((s) => s.toggleLeftPanel);
  const toggleBottomPanel = useStore((s) => s.toggleBottomPanel);
  const toggleRightPanel = useStore((s) => s.toggleRightPanel);

  useEffect(() => {
    initIpcBridge();
  }, []);

  return (
    <div className={styles.app}>
      {/* Unified title bar / toolbar */}
      <Header />

      {/* Main area: left strip + left panel + center + right panel + right strip */}
      <div className={styles.main}>
        {/* Left tool strip */}
        <ToolStrip
          side="left"
          items={LEFT_ITEMS}
          activeId={leftPanelOpen ? 'scripts' : null}
          onToggle={() => toggleLeftPanel()}
        />

        {/* Left panel (Scripts) */}
        {leftPanelOpen && <ScriptList />}

        {/* Center: tabs + editor + bottom panel */}
        <div className={styles.center}>
          <TabBar />
          <div className={styles.editorArea}>
            <Editor />
          </div>

          {/* Bottom panel with its own tab bar */}
          {bottomPanelOpen && (
            <div className={styles.bottomPanel}>
              {/* Bottom panel header */}
              <div className={styles.bottomPanelHeader}>
                <div className={styles.bottomTabs}>
                  <BottomTab label="Run" active />
                </div>
                <div className={styles.bottomActions}>
                  <ToolBtn
                    size="small"
                    onClick={() => useStore.getState().clearConsole()}
                    title="Clear"
                  >
                    <svg width="12" height="12" viewBox="0 0 16 16"><path d="M2 2h12M4 6h8M6 10h4M8 14" stroke="currentColor" strokeWidth="1.5"/></svg>
                  </ToolBtn>
                  <ToolBtn
                    size="small"
                    onClick={() => toggleBottomPanel()}
                    title="Close"
                  >
                    <svg width="12" height="12" viewBox="0 0 16 16"><path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" strokeWidth="1.5"/></svg>
                  </ToolBtn>
                </div>
              </div>
              <Console />
            </div>
          )}
        </div>

        {/* Right panel (Debug) */}
        {rightPanelOpen && isDebugging && <DebugPanel />}

        {/* Right tool strip */}
        <ToolStrip
          side="right"
          items={RIGHT_ITEMS}
          activeId={rightPanelOpen && isDebugging ? 'debug' : null}
          onToggle={() => toggleRightPanel()}
        />
      </div>

      {/* Status bar */}
      <StatusBar />
    </div>
  );
}

function BottomTab({ label, active }: { label: string; active?: boolean }) {
  return (
    <div className={`${styles.bottomTab} ${active ? styles.bottomTabActive : ''}`}>
      {label}
    </div>
  );
}
