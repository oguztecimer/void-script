import { useStore } from '../state/store';
import { sendToRust } from '../ipc/bridge';

const TYPE_LABELS: Record<string, string> = {
  ship_brain: 'Ship Brains',
  mothership_brain: 'Mothership',
  production: 'Production',
};
const TYPE_ORDER = ['ship_brain', 'mothership_brain', 'production'];

export function ScriptList() {
  const scriptList = useStore((s) => s.scriptList);
  const toggleLeftPanel = useStore((s) => s.toggleLeftPanel);
  const grouped = TYPE_ORDER.map((type) => ({
    type,
    label: TYPE_LABELS[type] || type,
    scripts: scriptList.filter((s) => s.script_type === type),
  })).filter((g) => g.scripts.length > 0);

  return (
    <div style={{
      width: '220px',
      backgroundColor: '#2b2d30',
      overflow: 'auto',
      display: 'flex',
      flexDirection: 'column',
      borderRight: '1px solid #393b40',
    }}>
      {/* Tool window header */}
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
        <span>Scripts</span>
        <button
          onClick={() => toggleLeftPanel()}
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

      <div style={{ flex: 1, overflow: 'auto', padding: '4px 0' }}>
        {grouped.map((group) => (
          <div key={group.type}>
            <div style={{
              padding: '6px 12px 4px',
              fontSize: '11px',
              textTransform: 'uppercase',
              letterSpacing: '0.5px',
              color: '#6f737a',
              fontWeight: 600,
            }}>
              {group.label}
            </div>
            {group.scripts.map((script) => (
              <div
                key={script.id}
                onClick={() => sendToRust({ type: 'script_request', script_id: script.id })}
                style={{
                  padding: '4px 12px 4px 20px',
                  cursor: 'pointer',
                  fontSize: '13px',
                  color: '#bcbec4',
                  borderRadius: '4px',
                  margin: '0 4px',
                  display: 'flex',
                  alignItems: 'center',
                  gap: '6px',
                }}
                onMouseEnter={(e) => { e.currentTarget.style.backgroundColor = '#313335'; }}
                onMouseLeave={(e) => { e.currentTarget.style.backgroundColor = 'transparent'; }}
              >
                <span style={{ color: '#6f737a', fontSize: '11px' }}>VS</span>
                <span>{script.name}</span>
              </div>
            ))}
          </div>
        ))}
        {scriptList.length === 0 && (
          <div style={{ padding: '16px 12px', color: '#5a5d63', fontSize: '12px', fontStyle: 'italic' }}>
            No scripts loaded
          </div>
        )}
      </div>
    </div>
  );
}
