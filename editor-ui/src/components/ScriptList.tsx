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
      backgroundColor: 'var(--bg-panel)',
      overflow: 'auto',
      display: 'flex',
      flexDirection: 'column',
      borderRight: '1px solid var(--border-strong)',
    }}>
      {/* Tool window header */}
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
        <span>Scripts</span>
        <ToolWindowBtn onClick={() => toggleLeftPanel()} title="Hide" />
      </div>

      <div style={{ flex: 1, overflow: 'auto', padding: '4px 0' }}>
        {grouped.map((group) => (
          <div key={group.type}>
            <div style={{
              padding: '6px 12px 4px',
              fontSize: '11px',
              textTransform: 'uppercase',
              letterSpacing: '0.5px',
              color: 'var(--text-tertiary)',
              fontWeight: 600,
            }}>
              {group.label}
            </div>
            {group.scripts.map((script) => (
              <div
                key={script.id}
                onClick={() => sendToRust({ type: 'script_request', script_id: script.id })}
                style={{
                  padding: '4px 12px 4px 24px',
                  cursor: 'pointer',
                  fontSize: '13px',
                  color: 'var(--text-primary)',
                  borderRadius: '4px',
                  margin: '0 4px',
                  display: 'flex',
                  alignItems: 'center',
                  gap: '6px',
                }}
                onMouseEnter={(e) => { e.currentTarget.style.backgroundColor = 'var(--bg-hover)'; }}
                onMouseLeave={(e) => { e.currentTarget.style.backgroundColor = 'transparent'; }}
              >
                <span>{script.name}</span>
              </div>
            ))}
          </div>
        ))}
        {scriptList.length === 0 && (
          <div style={{ padding: '16px 12px', color: 'var(--text-disabled)', fontSize: '12px', fontStyle: 'italic' }}>
            No scripts loaded
          </div>
        )}
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
