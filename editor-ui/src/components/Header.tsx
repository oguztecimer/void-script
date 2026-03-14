import { useStore } from '../state/store';
import { sendToRust } from '../ipc/bridge';

export function Header() {
  const activeTabId = useStore((s) => s.activeTabId);
  const tabs = useStore((s) => s.tabs);
  const activeTab = tabs.find((t) => t.scriptId === activeTabId);
  const isRunning = useStore((s) => s.isRunning);
  const isDebugging = useStore((s) => s.isDebugging);
  const isPaused = useStore((s) => s.isPaused);
  const active = isRunning || isDebugging;

  return (
    <div style={{
      display: 'flex',
      alignItems: 'center',
      height: '36px',
      backgroundColor: '#2b2d30',
      padding: '0 12px',
      userSelect: 'none',
      borderBottom: '1px solid #393b40',
      fontSize: '12px',
    }}>
      {/* Left: project name + branch */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
        <span style={{ color: '#bcbec4', fontWeight: 600, fontSize: '13px' }}>VOID//SCRIPT</span>
        <span style={{ color: '#6f737a' }}>|</span>
        <span style={{ color: '#6f737a' }}>main</span>
      </div>

      {/* Center spacer */}
      <div style={{ flex: 1 }} />

      {/* Right: run config + run/debug */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
        {/* Run configuration selector */}
        <div style={{
          display: 'flex',
          alignItems: 'center',
          gap: '4px',
          padding: '2px 10px',
          backgroundColor: '#393b40',
          borderRadius: '4px',
          color: '#bcbec4',
          height: '24px',
          cursor: 'pointer',
          fontSize: '12px',
        }}>
          {activeTab ? `${activeTab.name}.vs` : 'No configuration'}
          <span style={{ color: '#6f737a', fontSize: '8px', marginLeft: '2px' }}>v</span>
        </div>

        <span style={{ color: '#393b40' }}>|</span>

        {/* Run/Debug/Stop buttons */}
        {!active ? (
          <>
            <ToolBtn title="Run" color="#57a64a" onClick={() => activeTabId && sendToRust({ type: 'run_script', script_id: activeTabId })} disabled={!activeTabId}>
              <svg width="12" height="12" viewBox="0 0 16 16"><path d="M4 2l10 6-10 6V2z" fill="currentColor"/></svg>
            </ToolBtn>
            <ToolBtn title="Debug" color="#3574f0" onClick={() => activeTabId && sendToRust({ type: 'debug_start', script_id: activeTabId })} disabled={!activeTabId}>
              <svg width="12" height="12" viewBox="0 0 16 16">
                <circle cx="8" cy="9" r="5" stroke="currentColor" strokeWidth="1.5" fill="none"/>
                <line x1="8" y1="4" x2="6" y2="1" stroke="currentColor" strokeWidth="1.5"/>
                <line x1="8" y1="4" x2="10" y2="1" stroke="currentColor" strokeWidth="1.5"/>
                <line x1="3" y1="7" x2="13" y2="7" stroke="currentColor" strokeWidth="1"/>
                <line x1="3" y1="10" x2="13" y2="10" stroke="currentColor" strokeWidth="1"/>
              </svg>
            </ToolBtn>
          </>
        ) : (
          <>
            <ToolBtn title="Stop" color="#ef5350" onClick={() => activeTabId && sendToRust({ type: 'stop_script', script_id: activeTabId })}>
              <svg width="10" height="10" viewBox="0 0 10 10"><rect width="10" height="10" rx="1.5" fill="currentColor"/></svg>
            </ToolBtn>
            {isDebugging && isPaused && (
              <>
                <ToolBtn title="Resume" color="#57a64a" onClick={() => activeTabId && sendToRust({ type: 'debug_continue', script_id: activeTabId })}>
                  <svg width="12" height="12" viewBox="0 0 16 16"><path d="M4 2l10 6-10 6V2z" fill="currentColor"/></svg>
                </ToolBtn>
                <ToolBtn title="Step Over" color="#bcbec4" onClick={() => activeTabId && sendToRust({ type: 'debug_step_over', script_id: activeTabId })}>
                  <svg width="14" height="14" viewBox="0 0 16 16"><path d="M2 12h4V8h4v4h4L8 4z" fill="currentColor" transform="rotate(90 8 8)"/></svg>
                </ToolBtn>
                <ToolBtn title="Step Into" color="#bcbec4" onClick={() => activeTabId && sendToRust({ type: 'debug_step_into', script_id: activeTabId })}>
                  <svg width="14" height="14" viewBox="0 0 16 16"><path d="M8 2v8m-3-3l3 3 3-3M5 14h6" stroke="currentColor" strokeWidth="1.5" fill="none"/></svg>
                </ToolBtn>
                <ToolBtn title="Step Out" color="#bcbec4" onClick={() => activeTabId && sendToRust({ type: 'debug_step_out', script_id: activeTabId })}>
                  <svg width="14" height="14" viewBox="0 0 16 16"><path d="M8 14V6m-3 3l3-3 3 3M5 2h6" stroke="currentColor" strokeWidth="1.5" fill="none"/></svg>
                </ToolBtn>
              </>
            )}
            <span style={{ color: '#393b40' }}>|</span>
            <span style={{ color: isDebugging ? '#3574f0' : '#57a64a', fontSize: '11px' }}>
              {isDebugging ? (isPaused ? 'Paused' : 'Debugging...') : 'Running...'}
            </span>
          </>
        )}
      </div>
    </div>
  );
}

function ToolBtn({ title, color, onClick, disabled, children }: {
  title: string; color: string; onClick?: () => void; disabled?: boolean; children: React.ReactNode;
}) {
  return (
    <button onClick={onClick} disabled={disabled} title={title}
      style={{
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        width: '24px', height: '24px', background: 'none', border: 'none',
        borderRadius: '4px', color: disabled ? '#5a5d63' : color,
        cursor: disabled ? 'default' : 'pointer', padding: 0,
        opacity: disabled ? 0.5 : 1,
      }}
      onMouseEnter={(e) => { if (!disabled) e.currentTarget.style.backgroundColor = '#393b40'; }}
      onMouseLeave={(e) => { e.currentTarget.style.backgroundColor = 'transparent'; }}
    >
      {children}
    </button>
  );
}
