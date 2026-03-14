import { useEffect, useRef, useState } from 'react';
import {
  Group,
  Panel,
  Separator,
  type PanelImperativeHandle,
  useDefaultLayout,
} from 'react-resizable-panels';
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

  // v4 API: panelRef prop (not ref) on Panel component
  const leftPanelRef = useRef<PanelImperativeHandle | null>(null);
  const rightPanelRef = useRef<PanelImperativeHandle | null>(null);
  const [isResizing, setIsResizing] = useState(false);

  // Persist layout across page loads via useDefaultLayout
  const { defaultLayout, onLayoutChanged: saveLayout } = useDefaultLayout({ id: 'void-main-layout' });

  useEffect(() => {
    initIpcBridge();
  }, []);

  // Imperative collapse/expand synced to Zustand state
  useEffect(() => {
    const panel = leftPanelRef.current;
    if (!panel) return;
    if (leftPanelOpen) {
      panel.expand();
    } else {
      panel.collapse();
    }
  }, [leftPanelOpen]);

  useEffect(() => {
    const panel = rightPanelRef.current;
    if (!panel) return;
    if (rightPanelOpen && isDebugging) {
      panel.expand();
    } else {
      panel.collapse();
    }
  }, [rightPanelOpen, isDebugging]);

  return (
    <div className={styles.app}>
      {/* Unified title bar / toolbar */}
      <Header />

      {/* Main area: left strip + resizable panels + right strip */}
      <div className={styles.main}>
        {/* Left tool strip — fixed, OUTSIDE Group */}
        <ToolStrip
          side="left"
          items={LEFT_ITEMS}
          activeId={leftPanelOpen ? 'scripts' : null}
          onToggle={() => toggleLeftPanel()}
        />

        {/* Resizable panel layout */}
        <Group
          id="void-main-layout"
          orientation="horizontal"
          defaultLayout={defaultLayout}
          onLayoutChange={() => setIsResizing(true)}
          onLayoutChanged={(layout) => { setIsResizing(false); saveLayout(layout); }}
          className={styles.panelGroup}
        >
          {/* Left panel — ALWAYS rendered, collapse/expand via imperative API */}
          <Panel
            panelRef={leftPanelRef}
            id="left-panel"
            defaultSize="18%"
            minSize="10%"
            maxSize="40%"
            collapsible
            collapsedSize={0}
            className={!isResizing ? styles.panelAnimated : ''}
          >
            <ScriptList />
          </Panel>

          <Separator
            id="left-separator"
            className={styles.resizeHandle}
          />

          {/* Center panel */}
          <Panel id="center">
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
          </Panel>

          <Separator
            id="right-separator"
            className={styles.resizeHandle}
          />

          {/* Right panel — ALWAYS rendered, collapse/expand via imperative API */}
          <Panel
            panelRef={rightPanelRef}
            id="right-panel"
            defaultSize="18%"
            minSize="10%"
            maxSize="40%"
            collapsible
            collapsedSize={0}
            className={!isResizing ? styles.panelAnimated : ''}
          >
            {isDebugging && <DebugPanel />}
          </Panel>
        </Group>

        {/* Right tool strip — fixed, OUTSIDE Group */}
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
