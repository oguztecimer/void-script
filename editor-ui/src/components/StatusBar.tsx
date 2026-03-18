import { useStore } from '../state/store';
import { StatusSegment } from '../primitives/StatusSegment';
import { NavPath } from './NavPath';
import { DiagnosticsWidget } from './DiagnosticsWidget';
import styles from './StatusBar.module.css';

export function StatusBar() {
  const cursorLine = useStore((s) => s.cursorLine);
  const cursorCol = useStore((s) => s.cursorCol);
  const activeTabId = useStore((s) => s.activeTabId);
  const tabs = useStore((s) => s.tabs);
  const activeTab = tabs.find((t) => t.scriptId === activeTabId);
  const errorCount = activeTab?.diagnostics.filter((d) => d.severity === 'error').length ?? 0;
  const warningCount = activeTab?.diagnostics.filter((d) => d.severity === 'warning').length ?? 0;

  return (
    <div className={styles.bar}>
      <NavPath />
      <div className={styles.spacer} />

      <DiagnosticsWidget
        errorCount={errorCount}
        warningCount={warningCount}
        hasActiveTab={!!activeTab}
      />

      {activeTab && (
        <>
          <StatusSegment label={`Ln ${cursorLine}, Col ${cursorCol}`} />
          <StatusSegment label="LF" />
          <StatusSegment label="UTF-8" />
          <StatusSegment label="GrimScript" />
        </>
      )}
    </div>
  );
}
