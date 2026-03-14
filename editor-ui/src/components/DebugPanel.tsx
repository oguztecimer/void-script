import { useStore } from '../state/store';

export function DebugPanel() {
  const debugVariables = useStore((s) => s.debugVariables);
  const debugCallStack = useStore((s) => s.debugCallStack);
  const toggleRightPanel = useStore((s) => s.toggleRightPanel);

  return (
    <div style={{
      width: '250px',
      backgroundColor: '#2b2d30',
      overflow: 'auto',
      display: 'flex',
      flexDirection: 'column',
      borderLeft: '1px solid #393b40',
    }}>
      {/* Call Stack section */}
      <div style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '6px 12px',
        fontSize: '12px',
        fontWeight: 600,
        color: '#bcbec4',
        borderBottom: '1px solid #393b40',
        minHeight: '30px',
      }}>
        <span>Frames</span>
        <button
          onClick={() => toggleRightPanel()}
          title="Hide"
          style={{
            background: 'none', border: 'none', color: '#6f737a',
            cursor: 'pointer', fontSize: '14px', padding: '2px 4px',
            borderRadius: '4px', display: 'flex', alignItems: 'center',
          }}
          onMouseEnter={(e) => { e.currentTarget.style.backgroundColor = '#393b40'; e.currentTarget.style.color = '#bcbec4'; }}
          onMouseLeave={(e) => { e.currentTarget.style.backgroundColor = 'transparent'; e.currentTarget.style.color = '#6f737a'; }}
        >
          x
        </button>
      </div>
      <div style={{ padding: '4px 0', borderBottom: '1px solid #393b40' }}>
        {debugCallStack.map((frame, i) => (
          <div key={i} style={{
            padding: '3px 12px',
            fontSize: '12px',
            color: i === 0 ? '#bcbec4' : '#6f737a',
            backgroundColor: i === 0 ? '#214283' : 'transparent',
            fontFamily: "'JetBrains Mono', monospace",
          }}>
            {frame}
          </div>
        ))}
      </div>

      {/* Variables section */}
      <div style={{
        display: 'flex',
        alignItems: 'center',
        padding: '6px 12px',
        fontSize: '12px',
        fontWeight: 600,
        color: '#bcbec4',
        borderBottom: '1px solid #393b40',
        minHeight: '30px',
      }}>
        Variables
      </div>
      <div style={{ flex: 1, overflow: 'auto', padding: '4px 0' }}>
        {debugVariables.length === 0 && (
          <div style={{ padding: '8px 12px', color: '#5a5d63', fontSize: '12px', fontStyle: 'italic' }}>
            No variables in scope
          </div>
        )}
        {debugVariables.map((v, i) => (
          <div key={i} style={{
            display: 'flex',
            justifyContent: 'space-between',
            padding: '2px 12px',
            fontSize: '12px',
            fontFamily: "'JetBrains Mono', monospace",
          }}>
            <span style={{ color: '#bcbec4' }}>{v.name}</span>
            <span style={{
              color: '#6897bb',
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
