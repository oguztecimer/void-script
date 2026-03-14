import { useStore } from '../state/store';
import { sendToRust } from '../ipc/bridge';

export function TabBar() {
  const tabs = useStore((s) => s.tabs);
  const activeTabId = useStore((s) => s.activeTabId);

  if (tabs.length === 0) return null;

  return (
    <div style={{
      display: 'flex',
      backgroundColor: '#2b2d30',
      overflow: 'hidden',
      minHeight: '34px',
      alignItems: 'stretch',
      borderBottom: '1px solid #393b40',
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
              padding: '0 16px',
              cursor: 'pointer',
              backgroundColor: isActive ? '#1e1f22' : 'transparent',
              borderBottom: isActive ? '2px solid #3574f0' : '2px solid transparent',
              color: isActive ? '#bcbec4' : '#6f737a',
              fontSize: '13px',
              whiteSpace: 'nowrap',
              userSelect: 'none',
              transition: 'background-color 0.1s',
            }}
            onMouseEnter={(e) => { if (!isActive) e.currentTarget.style.backgroundColor = '#313335'; }}
            onMouseLeave={(e) => { if (!isActive) e.currentTarget.style.backgroundColor = 'transparent'; }}
          >
            <span style={{ color: '#6f737a', fontSize: '12px', marginRight: '2px' }}>VS</span>
            <span>{tab.name}</span>
            {tab.isModified && (
              <span style={{ color: '#bcbec4', fontSize: '8px' }}>●</span>
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
                color: '#6f737a',
                cursor: 'pointer',
                fontSize: '14px',
                lineHeight: '1',
                marginLeft: '2px',
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.backgroundColor = '#393b40';
                e.currentTarget.style.color = '#bcbec4';
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.backgroundColor = 'transparent';
                e.currentTarget.style.color = '#6f737a';
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
