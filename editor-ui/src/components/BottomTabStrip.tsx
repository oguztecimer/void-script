import { useStore } from '../state/store';
import { ToolBtn } from '../primitives/ToolBtn';
import styles from './BottomTabStrip.module.css';

const BOTTOM_TABS = [
  { id: 'terminal', label: 'Terminal' },
  { id: 'console', label: 'Console' },
];

export function BottomTabStrip() {
  const activeTab = useStore((s) => s.bottomPanelTab);
  const setTab = useStore((s) => s.setBottomPanelTab);
  const clearConsole = useStore((s) => s.clearConsole);
  const clearTerminal = useStore((s) => s.clearTerminal);
  const toggleBottomPanel = useStore((s) => s.toggleBottomPanel);
  const handleClear = activeTab === 'terminal' ? clearTerminal : clearConsole;

  return (
    <div className={styles.strip}>
      <div className={styles.tabs}>
        {BOTTOM_TABS.map((tab) => (
          <button
            key={tab.id}
            className={`${styles.tab} ${activeTab === tab.id ? styles.active : ''}`}
            onClick={() => setTab(tab.id)}
          >
            {tab.label}
          </button>
        ))}
      </div>
      <div className={styles.actions}>
        <ToolBtn size="small" onClick={handleClear} title="Clear">
          <svg width="12" height="12" viewBox="0 0 16 16">
            <path d="M2 2h12M4 6h8M6 10h4M8 14" stroke="currentColor" strokeWidth="1.5" fill="none"/>
          </svg>
        </ToolBtn>
        <ToolBtn size="small" onClick={toggleBottomPanel} title="Close">
          <svg width="12" height="12" viewBox="0 0 16 16">
            <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" strokeWidth="1.5" fill="none"/>
          </svg>
        </ToolBtn>
      </div>
    </div>
  );
}
