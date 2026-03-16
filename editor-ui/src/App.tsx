import { useCallback, useEffect, useRef, useState } from 'react';
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
import { BreadcrumbBar } from './components/BreadcrumbBar';
import { Editor } from './components/Editor';
import { ScriptList } from './components/ScriptList';
import { Console } from './components/Console';
import { DebugPanel } from './components/DebugPanel';
import { StatusBar } from './components/StatusBar';
import { BottomTabStrip } from './components/BottomTabStrip';
import { WindowResizeBorder } from './components/WindowResizeBorder';
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
  const toggleBottomPanel = useStore((s) => s.toggleBottomPanel);
  const setBottomPanelOpen = useStore((s) => s.setBottomPanelOpen);

  // v4 API: panelRef prop (not ref) on Panel component
  const leftPanelRef = useRef<PanelImperativeHandle | null>(null);
  const rightPanelRef = useRef<PanelImperativeHandle | null>(null);
  const bottomPanelRef = useRef<PanelImperativeHandle | null>(null);
  const [isResizing, setIsResizing] = useState(false);

  // Persist layout across page loads via useDefaultLayout
  const { defaultLayout, onLayoutChanged: saveLayout } = useDefaultLayout({ id: 'void-main-layout' });
  const { defaultLayout: centerLayout, onLayoutChanged: saveCenterLayout } = useDefaultLayout({ id: 'void-center-layout' });

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

  // Sync bottom panel imperative collapse/expand to Zustand state (mirrors left/right pattern)
  useEffect(() => {
    const panel = bottomPanelRef.current;
    if (!panel) return;
    if (bottomPanelOpen) {
      panel.expand();
    } else {
      panel.collapse();
    }
  }, [bottomPanelOpen]);

  // Double-click handlers — toggle collapse/expand for each separator
  const handleLeftSeparatorDoubleClick = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    toggleLeftPanel();
  }, [toggleLeftPanel]);

  const handleRightSeparatorDoubleClick = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    toggleRightPanel();
  }, [toggleRightPanel]);

  const handleBottomSeparatorDoubleClick = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    toggleBottomPanel();
  }, [toggleBottomPanel]);

  return (
    <div className={styles.app}>
      {/* Window resize borders (Windows only — on macOS the native frame handles resize) */}
      <WindowResizeBorder />

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
            onResize={(size) => {
              // Sync Zustand when left panel collapses/expands via drag
              const isNowCollapsed = size.asPercentage < 1;
              if (isNowCollapsed && leftPanelOpen) {
                useStore.setState({ leftPanelOpen: false });
              } else if (!isNowCollapsed && !leftPanelOpen) {
                useStore.setState({ leftPanelOpen: true });
              }
            }}
          >
            <ScriptList />
          </Panel>

          <Separator
            id="left-separator"
            className={leftPanelOpen ? styles.resizeHandle : styles.resizeHandleHidden}
            disabled={!leftPanelOpen}
            onDoubleClick={handleLeftSeparatorDoubleClick}
          />

          {/* Center panel */}
          <Panel id="center">
            <div className={styles.center}>
              <TabBar />
              <BreadcrumbBar />
              <Group
                id="void-center-layout"
                orientation="vertical"
                defaultLayout={centerLayout}
                onLayoutChange={() => setIsResizing(true)}
                onLayoutChanged={(layout) => { setIsResizing(false); saveCenterLayout(layout); }}
                className={styles.centerGroup}
              >
                <Panel id="editor-panel" minSize="50%" maxSize="90%">
                  <div className={styles.editorArea}>
                    <Editor />
                  </div>
                </Panel>

                <Separator
                  id="bottom-separator"
                  className={bottomPanelOpen ? styles.resizeHandleHorizontal : styles.resizeHandleHidden}
                  disabled={!bottomPanelOpen}
                  onDoubleClick={handleBottomSeparatorDoubleClick}
                />

                {/* Bottom panel — ALWAYS rendered, collapse/expand via imperative API */}
                <Panel
                  panelRef={bottomPanelRef}
                  id="bottom-panel"
                  defaultSize="25%"
                  minSize="10%"
                  maxSize="50%"
                  collapsible
                  collapsedSize={0}
                  className={!isResizing ? styles.panelAnimated : ''}
                  onResize={(size) => {
                    // Sync Zustand when panel collapses/expands via drag
                    // Use < 1 threshold instead of === 0 for floating point safety
                    const isNowCollapsed = size.asPercentage < 1;
                    if (isNowCollapsed && bottomPanelOpen) {
                      setBottomPanelOpen(false);
                    } else if (!isNowCollapsed && !bottomPanelOpen) {
                      setBottomPanelOpen(true);
                    }
                  }}
                >
                  <div className={styles.bottomPanel}>
                    <BottomTabStrip />
                    <Console />
                  </div>
                </Panel>
              </Group>
            </div>
          </Panel>

          <Separator
            id="right-separator"
            className={rightPanelOpen && isDebugging ? styles.resizeHandle : styles.resizeHandleHidden}
            disabled={!rightPanelOpen || !isDebugging}
            onDoubleClick={handleRightSeparatorDoubleClick}
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
            onResize={(size) => {
              // Sync Zustand when right panel collapses/expands via drag
              const isNowCollapsed = size.asPercentage < 1;
              if (isNowCollapsed && rightPanelOpen) {
                useStore.setState({ rightPanelOpen: false });
              } else if (!isNowCollapsed && !rightPanelOpen) {
                useStore.setState({ rightPanelOpen: true });
              }
            }}
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
