import { useStore } from '../state/store';
import { sendToRust } from '../ipc/bridge';
import styles from './TabBar.module.css';

export function TabBar() {
  const tabs = useStore((s) => s.tabs);
  const activeTabId = useStore((s) => s.activeTabId);

  if (tabs.length === 0) return null;

  return (
    <div className={styles.bar}>
      {tabs.map((tab) => {
        const isActive = tab.scriptId === activeTabId;
        return (
          <div
            key={tab.scriptId}
            className={`${styles.tab} ${isActive ? styles.active : ''}`}
            onClick={() => {
              useStore.getState().switchTab(tab.scriptId);
              sendToRust({ type: 'tab_changed', script_id: tab.scriptId });
            }}
          >
            <span>{tab.name}</span>
            {tab.isModified && (
              <span className={styles.modified}>●</span>
            )}
            <button
              className={styles.closeBtn}
              onClick={(e) => {
                e.stopPropagation();
                useStore.getState().closeTab(tab.scriptId);
              }}
            >
              ×
            </button>
          </div>
        );
      })}
    </div>
  );
}
