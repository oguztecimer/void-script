interface ToolStripItem {
  id: string;
  icon: string;
  label: string;
  shortcut?: string;
}

interface Props {
  side: 'left' | 'right';
  items: ToolStripItem[];
  activeId: string | null;
  onToggle: (id: string) => void;
}

export function ToolStrip({ side, items, activeId, onToggle }: Props) {
  return (
    <div style={{
      display: 'flex',
      flexDirection: 'column',
      width: '36px',
      backgroundColor: 'var(--bg-panel)',
      alignItems: 'center',
      paddingTop: '6px',
      gap: '2px',
      borderLeft: side === 'right' ? '1px solid var(--border-strong)' : undefined,
      borderRight: side === 'left' ? '1px solid var(--border-strong)' : undefined,
    }}>
      {items.map((item) => {
        const isActive = activeId === item.id;
        return (
          <button
            key={item.id}
            onClick={() => onToggle(item.id)}
            title={`${item.label}${item.shortcut ? ` (${item.shortcut})` : ''}`}
            style={{
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'center',
              justifyContent: 'center',
              width: '30px',
              height: '30px',
              background: isActive ? 'var(--accent-blue)' : 'none',
              border: 'none',
              borderRadius: '6px',
              color: isActive ? 'white' : 'var(--text-secondary)',
              cursor: 'pointer',
              fontSize: '13px',
              padding: 0,
            }}
            onMouseEnter={(e) => { if (!isActive) e.currentTarget.style.backgroundColor = 'var(--bg-hover)'; }}
            onMouseLeave={(e) => { if (!isActive) e.currentTarget.style.backgroundColor = isActive ? 'var(--accent-blue)' : 'transparent'; }}
          >
            {item.icon}
          </button>
        );
      })}
    </div>
  );
}
