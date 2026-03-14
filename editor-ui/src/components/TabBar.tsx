import { useStore } from '../state/store';
import { sendToRust } from '../ipc/bridge';

export function TabBar() {
  const tabs = useStore((s) => s.tabs);
  const activeTabId = useStore((s) => s.activeTabId);

  if (tabs.length === 0) return null;

  return (
    <div style={{
      display: 'flex',
      backgroundColor: 'var(--bg-tab-inactive)',
      overflow: 'hidden',
      minHeight: '36px',
      alignItems: 'stretch',
      borderBottom: '1px solid var(--border-default)',
    }}>
      {tabs.map((tab) => {
        const isActive = tab.scriptId === activeTabId;
        return (
          <div
            key={tab.scriptId}
            onClick={() => {
              useStore.getState().switchTab(tab.scriptId);
              sendToRust({ type: 'tab_changed', script_id: tab.scriptId });
            }}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '6px',
              padding: '0 14px',
              cursor: 'pointer',
              backgroundColor: isActive ? 'var(--bg-tab-active)' : 'transparent',
              borderBottom: isActive ? '2px solid var(--accent-blue)' : '2px solid transparent',
              color: isActive ? 'var(--text-primary)' : 'var(--text-secondary)',
              fontSize: '13px',
              whiteSpace: 'nowrap',
              userSelect: 'none',
              transition: 'background-color 0.1s',
            }}
            onMouseEnter={(e) => { if (!isActive) e.currentTarget.style.backgroundColor = 'var(--bg-tab-active)'; }}
            onMouseLeave={(e) => { if (!isActive) e.currentTarget.style.backgroundColor = 'transparent'; }}
          >
            <span>{tab.name}</span>
            {tab.isModified && (
              <span style={{ color: 'var(--text-primary)', fontSize: '8px' }}>●</span>
            )}
            <span
              onClick={(e) => {
                e.stopPropagation();
                useStore.getState().closeTab(tab.scriptId);
              }}
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                width: '18px',
                height: '18px',
                borderRadius: '4px',
                color: 'var(--text-tertiary)',
                cursor: 'pointer',
                fontSize: '14px',
                lineHeight: '1',
                marginLeft: '2px',
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.backgroundColor = 'var(--bg-hover)';
                e.currentTarget.style.color = 'var(--text-primary)';
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.backgroundColor = 'transparent';
                e.currentTarget.style.color = 'var(--text-tertiary)';
              }}
            >
              ×
            </span>
          </div>
        );
      })}
    </div>
  );
}
