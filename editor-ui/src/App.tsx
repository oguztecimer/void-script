import { useEffect } from 'react';
import { Header } from './components/Header';
import { ToolStrip } from './components/ToolStrip';
import { TabBar } from './components/TabBar';
import { Editor } from './components/Editor';
import { ScriptList } from './components/ScriptList';
import { Console } from './components/Console';
import { DebugPanel } from './components/DebugPanel';
import { StatusBar } from './components/StatusBar';
import { initIpcBridge } from './ipc/bridge';
import { useStore } from './state/store';

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
    <div style={{ display: 'flex', flexDirection: 'column', height: '100vh', background: '#1e1f22' }}>
      {/* Header */}
      <Header />

      {/* Main area: left strip + left panel + center + right panel + right strip */}
      <div style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
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
        <div style={{ display: 'flex', flexDirection: 'column', flex: 1, overflow: 'hidden' }}>
          <TabBar />
          <div style={{ flex: 1, overflow: 'hidden' }}>
            <Editor />
          </div>

          {/* Bottom panel with its own tab bar */}
          {bottomPanelOpen && (
            <div style={{
              height: '200px',
              display: 'flex',
              flexDirection: 'column',
              borderTop: '1px solid #393b40',
            }}>
              {/* Bottom panel header with tabs */}
              <div style={{
                display: 'flex',
                alignItems: 'center',
                backgroundColor: '#2b2d30',
                borderBottom: '1px solid #393b40',
                minHeight: '28px',
                padding: '0 4px',
                justifyContent: 'space-between',
              }}>
                <div style={{ display: 'flex', gap: '0' }}>
                  <BottomTab label="Run" active />
                </div>
                <div style={{ display: 'flex', gap: '2px', padding: '0 4px' }}>
                  <PanelHeaderBtn label="Clear" onClick={() => useStore.getState().clearConsole()} />
                  <PanelHeaderBtn label="x" onClick={() => toggleBottomPanel()} fontSize="14px" />
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
    <div style={{
      padding: '4px 12px',
      fontSize: '12px',
      color: active ? '#bcbec4' : '#6f737a',
      borderBottom: active ? '2px solid #3574f0' : '2px solid transparent',
      cursor: 'pointer',
      userSelect: 'none',
    }}>
      {label}
    </div>
  );
}

function PanelHeaderBtn({ label, onClick, fontSize }: { label: string; onClick: () => void; fontSize?: string }) {
  return (
    <button
      onClick={onClick}
      style={{
        background: 'none', border: 'none', color: '#6f737a',
        cursor: 'pointer', fontSize: fontSize || '11px',
        fontFamily: 'inherit', padding: '2px 6px', borderRadius: '4px',
        display: 'flex', alignItems: 'center',
      }}
      onMouseEnter={(e) => { e.currentTarget.style.backgroundColor = '#393b40'; e.currentTarget.style.color = '#bcbec4'; }}
      onMouseLeave={(e) => { e.currentTarget.style.backgroundColor = 'transparent'; e.currentTarget.style.color = '#6f737a'; }}
    >
      {label}
    </button>
  );
}
