import { useStore } from '../state/store';
import { sendToRust } from '../ipc/bridge';
import { PanelHeader } from '../primitives/PanelHeader';
import { ToolBtn } from '../primitives/ToolBtn';
import styles from './ScriptList.module.css';

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
    <div className={styles.panel}>
      <PanelHeader
        title="Scripts"
        actions={
          <>
            <ToolBtn size="small" onClick={() => sendToRust({ type: 'create_script' })} title="Add Script">
              <svg width="12" height="12" viewBox="0 0 16 16">
                <path d="M8 3v10M3 8h10" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"/>
              </svg>
            </ToolBtn>
            <ToolBtn size="small" onClick={() => toggleLeftPanel()} title="Hide">
              <svg width="12" height="12" viewBox="0 0 16 16">
                <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" strokeWidth="1.5"/>
              </svg>
            </ToolBtn>
          </>
        }
      />

      <div className={styles.list}>
        {grouped.map((group) => (
          <div key={group.type}>
            <div className={styles.groupLabel}>
              {group.label}
            </div>
            {group.scripts.map((script) => (
              <div
                key={script.id}
                className={styles.scriptItem}
                onClick={() => sendToRust({ type: 'script_request', script_id: script.id })}
              >
                <span>{script.name}</span>
              </div>
            ))}
          </div>
        ))}
        {scriptList.length === 0 && (
          <div className={styles.emptyState}>
            No scripts loaded
          </div>
        )}
      </div>
    </div>
  );
}
