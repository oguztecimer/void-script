import { useStore } from '../state/store';
import { PanelHeader } from '../primitives/PanelHeader';
import { ToolBtn } from '../primitives/ToolBtn';
import styles from './DebugPanel.module.css';

export function DebugPanel() {
  const debugVariables = useStore((s) => s.debugVariables);
  const debugCallStack = useStore((s) => s.debugCallStack);
  const toggleRightPanel = useStore((s) => s.toggleRightPanel);

  return (
    <div className={styles.panel}>
      {/* Top-level Debug header with close action */}
      <PanelHeader
        title="Debug"
        actions={
          <ToolBtn size="small" onClick={() => toggleRightPanel()} title="Hide">
            <svg width="12" height="12" viewBox="0 0 16 16">
              <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" strokeWidth="1.5"/>
            </svg>
          </ToolBtn>
        }
      />

      {/* Call Stack section */}
      <PanelHeader title="Frames" />
      <div className={styles.section}>
        {debugCallStack.map((frame, i) => (
          <div key={i} className={i === 0 ? styles.frameActive : styles.frameInactive}>
            {frame}
          </div>
        ))}
      </div>

      {/* Variables section */}
      <PanelHeader title="Variables" />
      <div className={styles.variables}>
        {debugVariables.length === 0 && (
          <div className={styles.emptyState}>
            No variables in scope
          </div>
        )}
        {debugVariables.map((v, i) => (
          <div key={i} className={styles.variable}>
            <span className={styles.variableName}>{v.name}</span>
            <span className={styles.variableValue} title={v.value}>
              {v.value}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}
