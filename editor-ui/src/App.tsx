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

const LEFT_PANEL_STORAGE_KEY = 'void-left-panel-width';
const LEFT_PANEL_DEFAULT_WIDTH = 220;
const LEFT_PANEL_MIN_WIDTH = 200;

const LEFT_ITEMS: ToolStripItem[] = [
  {
    id: 'scripts',
    icon: (
      <svg width="24" height="24" viewBox="0 0 16 16" fill="none">
        <path d="M4 2.5C4 2.5 4 2 4.5 2H11.5C12 2 12 2.5 12 2.5V13C12 13 12 13.5 11.5 13.5H5C4.2 13.5 3.5 13 3.5 12.2V4C3.5 3 4 2.5 4 2.5Z" stroke="currentColor" strokeWidth="1.1" strokeLinejoin="round"/>
        <path d="M3.5 12.2C3.5 11.5 4.2 11 5 11H12" stroke="currentColor" strokeWidth="1.1" strokeLinecap="round"/>
        <path d="M6.5 5H9.5M6.5 7.5H9" stroke="currentColor" strokeWidth="1" strokeLinecap="round"/>
      </svg>
    ),
    label: 'Grimoire',
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
  const _toggleLeftPanel = useStore((s) => s.toggleLeftPanel);
  const toggleRightPanel = useStore((s) => s.toggleRightPanel);
  const toggleBottomPanel = useStore((s) => s.toggleBottomPanel);
  const setBottomPanelOpen = useStore((s) => s.setBottomPanelOpen);

  const rightPanelRef = useRef<PanelImperativeHandle | null>(null);
  const bottomPanelRef = useRef<PanelImperativeHandle | null>(null);
  const [isResizing, setIsResizing] = useState(false);

  // Left panel width — fixed pixel value, persisted to localStorage
  const [leftPanelWidth, setLeftPanelWidth] = useState(() => {
    const saved = localStorage.getItem(LEFT_PANEL_STORAGE_KEY);
    return saved ? Number(saved) : LEFT_PANEL_DEFAULT_WIDTH;
  });
  const leftPanelElRef = useRef<HTMLDivElement | null>(null);
  // "Desired" width remembers what the user set, even when the panel is clamped smaller by the window
  const leftPanelDesiredWidthRef = useRef(leftPanelWidth);
  // Track whether the panel was auto-closed due to window shrinking
  const leftPanelAutoClosedRef = useRef(false);
  const toggleLeftPanel = useCallback(() => {
    leftPanelAutoClosedRef.current = false;
    _toggleLeftPanel();
  }, [_toggleLeftPanel]);

  // Clamp left panel width when window shrinks; restore up to desired width when window grows
  useEffect(() => {
    const onResize = () => {
      const maxWidth = window.innerWidth - 100;

      if (leftPanelAutoClosedRef.current && maxWidth >= LEFT_PANEL_MIN_WIDTH) {
        // Window grew back — reopen
        leftPanelAutoClosedRef.current = false;
        const target = Math.min(leftPanelDesiredWidthRef.current, maxWidth);
        setLeftPanelWidth(target);
        useStore.setState({ leftPanelOpen: true });
        return;
      }

      if (!leftPanelOpen) return;

      if (maxWidth < LEFT_PANEL_MIN_WIDTH) {
        // Window too small — auto-close
        leftPanelAutoClosedRef.current = true;
        useStore.setState({ leftPanelOpen: false });
        return;
      }
      const target = Math.min(leftPanelDesiredWidthRef.current, maxWidth);
      setLeftPanelWidth(target);
      // Also update the DOM directly for immediate feedback
      if (leftPanelElRef.current) {
        leftPanelElRef.current.style.width = `${target}px`;
      }
    };
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  }, [leftPanelOpen]);

  // Persist layout across page loads via useDefaultLayout
  const { defaultLayout, onLayoutChanged: saveLayout } = useDefaultLayout({ id: 'void-main-layout' });
  const { defaultLayout: centerLayout, onLayoutChanged: saveCenterLayout } = useDefaultLayout({ id: 'void-center-layout' });

  useEffect(() => {
    initIpcBridge();
  }, []);

  // Left panel drag-resize handler — mutates DOM directly during drag for performance
  const handleLeftResizeMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    const panelEl = leftPanelElRef.current;
    if (!panelEl) return;
    const leftOffset = panelEl.getBoundingClientRect().left;
    let currentWidth = panelEl.offsetWidth;

    // Disable transition during drag
    panelEl.style.transition = 'none';

    const onMouseMove = (ev: MouseEvent) => {
      const maxWidth = window.innerWidth - 100;
      currentWidth = Math.min(maxWidth, Math.max(LEFT_PANEL_MIN_WIDTH, ev.clientX - leftOffset));
      panelEl.style.width = `${currentWidth}px`;
    };
    const onMouseUp = () => {
      document.removeEventListener('mousemove', onMouseMove);
      document.removeEventListener('mouseup', onMouseUp);
      // Re-enable transition and sync React state
      panelEl.style.transition = '';
      setLeftPanelWidth(currentWidth);
      leftPanelDesiredWidthRef.current = currentWidth;
      localStorage.setItem(LEFT_PANEL_STORAGE_KEY, String(currentWidth));
    };
    document.addEventListener('mousemove', onMouseMove);
    document.addEventListener('mouseup', onMouseUp);
  }, []);

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

        {/* Left panel — fixed pixel width, outside resizable Group */}
        <div
          ref={leftPanelElRef}
          className={`${styles.leftPanel} ${!leftPanelOpen ? styles.leftPanelCollapsed : ''}`}
          style={{ width: leftPanelOpen ? leftPanelWidth : 0 }}
        >
          <ScriptList />
        </div>

        {/* Left resize handle */}
        <div
          className={leftPanelOpen ? styles.leftResizeHandle : styles.leftResizeHandleHidden}
          onMouseDown={leftPanelOpen ? handleLeftResizeMouseDown : undefined}
          onDoubleClick={(e) => { e.preventDefault(); toggleLeftPanel(); }}
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
          {/* Center panel */}
          <Panel id="center">
            <div className={styles.center}>
              <TabBar />
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
                    <BreadcrumbBar />
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
                  defaultSize="35%"
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
