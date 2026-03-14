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
      width: '28px',
      backgroundColor: '#2b2d30',
      alignItems: 'center',
      paddingTop: '4px',
      gap: '2px',
      borderLeft: side === 'right' ? '1px solid #1e1f22' : undefined,
      borderRight: side === 'left' ? '1px solid #1e1f22' : undefined,
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
              width: '24px',
              height: '24px',
              background: isActive ? '#393b40' : 'none',
              border: 'none',
              borderRadius: '6px',
              color: isActive ? '#bcbec4' : '#6f737a',
              cursor: 'pointer',
              fontSize: '13px',
              padding: 0,
            }}
            onMouseEnter={(e) => { if (!isActive) e.currentTarget.style.backgroundColor = '#313335'; }}
            onMouseLeave={(e) => { if (!isActive) e.currentTarget.style.backgroundColor = isActive ? '#393b40' : 'transparent'; }}
          >
            {item.icon}
          </button>
        );
      })}
    </div>
  );
}
