import { useEffect } from 'react';
import { Header } from './components/Header';
import { ToolStrip, type ToolStripItem } from './components/ToolStrip';
import { TabBar } from './components/TabBar';
import { Editor } from './components/Editor';
import { ScriptList } from './components/ScriptList';
import { Console } from './components/Console';
import { DebugPanel } from './components/DebugPanel';
import { StatusBar } from './components/StatusBar';
import { BottomTabStrip } from './components/BottomTabStrip';
import { initIpcBridge } from './ipc/bridge';
import { useStore } from './state/store';
import styles from './App.module.css';

const LEFT_ITEMS: ToolStripItem[] = [
  {
    id: 'scripts',
    icon: (
      <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
        <path d="M4 2h8v12H4V2z" stroke="currentColor" strokeWidth="1.2"/>
        <path d="M6 5h4M6 7.5h4M6 10h2.5" stroke="currentColor" strokeWidth="1.1" strokeLinecap="round"/>
      </svg>
    ),
    label: 'Scripts',
    shortcut: 'Alt+1',
  },
];
const RIGHT_ITEMS: ToolStripItem[] = [
  {
    id: 'debug',
    icon: (
      <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
        <circle cx="8" cy="6.5" r="3" stroke="currentColor" strokeWidth="1.2"/>
        <path d="M5.5 4L4 2.5M10.5 4L12 2.5M4.5 6.5H2M11.5 6.5H14M5.5 9L4 11M10.5 9L12 11M8 9.5V13" stroke="currentColor" strokeWidth="1.1" strokeLinecap="round"/>
      </svg>
    ),
    label: 'Debug',
    shortcut: 'Alt+5',
  },
];

export function App() {
  const leftPanelOpen = useStore((s) => s.leftPanelOpen);
  const bottomPanelOpen = useStore((s) => s.bottomPanelOpen);
  const rightPanelOpen = useStore((s) => s.rightPanelOpen);
  const isDebugging = useStore((s) => s.isDebugging);
  const toggleLeftPanel = useStore((s) => s.toggleLeftPanel);
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
              <BottomTabStrip />
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
