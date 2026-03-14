import { useStore } from '../state/store';

export function DebugPanel() {
  const debugVariables = useStore((s) => s.debugVariables);
  const debugCallStack = useStore((s) => s.debugCallStack);
  const toggleRightPanel = useStore((s) => s.toggleRightPanel);

  return (
    <div style={{
      width: '250px',
      backgroundColor: 'var(--bg-panel)',
      overflow: 'auto',
      display: 'flex',
      flexDirection: 'column',
      borderLeft: '1px solid var(--border-strong)',
    }}>
      {/* Call Stack section */}
      <div style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '0 12px',
        fontSize: '12px',
        fontWeight: 600,
        color: 'var(--text-primary)',
        borderBottom: '1px solid var(--border-default)',
        minHeight: '30px',
      }}>
        <span>Frames</span>
        <ToolWindowBtn onClick={() => toggleRightPanel()} title="Hide" />
      </div>
      <div style={{ padding: '4px 0', borderBottom: '1px solid var(--border-default)' }}>
        {debugCallStack.map((frame, i) => (
          <div key={i} style={{
            padding: '3px 12px',
            fontSize: '12px',
            color: i === 0 ? 'var(--text-primary)' : 'var(--text-tertiary)',
            backgroundColor: i === 0 ? 'var(--bg-selection)' : 'transparent',
            fontFamily: 'var(--font-mono)',
          }}>
            {frame}
          </div>
        ))}
      </div>

      {/* Variables section */}
      <div style={{
        display: 'flex',
        alignItems: 'center',
        padding: '0 12px',
        fontSize: '12px',
        fontWeight: 600,
        color: 'var(--text-primary)',
        borderBottom: '1px solid var(--border-default)',
        minHeight: '30px',
      }}>
        Variables
      </div>
      <div style={{ flex: 1, overflow: 'auto', padding: '4px 0' }}>
        {debugVariables.length === 0 && (
          <div style={{ padding: '8px 12px', color: 'var(--text-disabled)', fontSize: '12px', fontStyle: 'italic' }}>
            No variables in scope
          </div>
        )}
        {debugVariables.map((v, i) => (
          <div key={i} style={{
            display: 'flex',
            justifyContent: 'space-between',
            padding: '2px 12px',
            fontSize: '12px',
            fontFamily: 'var(--font-mono)',
          }}>
            <span style={{ color: 'var(--text-primary)' }}>{v.name}</span>
            <span style={{
              color: 'var(--text-secondary)',
              marginLeft: '8px',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
              maxWidth: '140px',
            }} title={v.value}>
              {v.value}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

function ToolWindowBtn({ onClick, title }: { onClick: () => void; title: string }) {
  return (
    <button
      onClick={onClick}
      title={title}
      style={{
        background: 'none',
        border: 'none',
        color: 'var(--text-tertiary)',
        cursor: 'pointer',
        fontSize: '12px',
        padding: '2px 6px',
        borderRadius: '4px',
        display: 'flex',
        alignItems: 'center',
      }}
      onMouseEnter={(e) => { e.currentTarget.style.backgroundColor = 'var(--bg-hover)'; e.currentTarget.style.color = 'var(--text-primary)'; }}
      onMouseLeave={(e) => { e.currentTarget.style.backgroundColor = 'transparent'; e.currentTarget.style.color = 'var(--text-tertiary)'; }}
    >
      <svg width="12" height="12" viewBox="0 0 16 16">
        <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" strokeWidth="1.5"/>
      </svg>
    </button>
  );
}
