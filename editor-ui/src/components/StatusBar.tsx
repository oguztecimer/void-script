import { useStore } from '../state/store';
import { StatusSegment } from '../primitives/StatusSegment';
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
      {/* Left: VCS branch */}
      <StatusSegment
        icon={
          <svg width="10" height="10" viewBox="0 0 16 16" style={{ marginRight: '4px' }}>
            <circle cx="5" cy="4" r="1.5" stroke="currentColor" strokeWidth="1" fill="none"/>
            <circle cx="5" cy="12" r="1.5" stroke="currentColor" strokeWidth="1" fill="none"/>
            <path d="M5 5.5v5" stroke="currentColor" strokeWidth="1" fill="none"/>
          </svg>
        }
        label="main"
      />
      <div className={styles.spacer} />

      {/* Diagnostics indicator */}
      {errorCount > 0 && (
        <StatusSegment label={<span style={{ color: 'var(--accent-red)' }}>{errorCount} errors</span>} />
      )}
      {warningCount > 0 && (
        <StatusSegment label={<span style={{ color: 'var(--accent-yellow)' }}>{warningCount} warn</span>} />
      )}
      {errorCount === 0 && warningCount === 0 && activeTab && (
        <StatusSegment label={<span style={{ color: 'var(--accent-green)' }}>OK</span>} />
      )}

      {/* Right side segments */}
      {activeTab && (
        <>
          <StatusSegment label={`Ln ${cursorLine}, Col ${cursorCol}`} />
          <StatusSegment label="LF" />
          <StatusSegment label="UTF-8" />
          <StatusSegment label="VoidScript" />
        </>
      )}
    </div>
  );
}
