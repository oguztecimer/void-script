import { useStore } from '../state/store';

export function StatusBar() {
  const cursorLine = useStore((s) => s.cursorLine);
  const cursorCol = useStore((s) => s.cursorCol);
  const activeTabId = useStore((s) => s.activeTabId);
  const tabs = useStore((s) => s.tabs);
  const activeTab = tabs.find((t) => t.scriptId === activeTabId);
  const errorCount = activeTab?.diagnostics.filter((d) => d.severity === 'error').length ?? 0;
  const warningCount = activeTab?.diagnostics.filter((d) => d.severity === 'warning').length ?? 0;

  return (
    <div style={{
      display: 'flex',
      alignItems: 'center',
      height: '24px',
      backgroundColor: 'var(--bg-panel)',
      fontSize: '11px',
      color: 'var(--text-secondary)',
      padding: '0 0',
      borderTop: '1px solid var(--border-strong)',
      gap: '0',
    }}>
      {/* Left: VCS branch */}
      <StatusSegment>
        <svg width="10" height="10" viewBox="0 0 16 16" style={{ marginRight: '4px' }}>
          <circle cx="5" cy="4" r="1.5" stroke="currentColor" strokeWidth="1" fill="none"/>
          <circle cx="5" cy="12" r="1.5" stroke="currentColor" strokeWidth="1" fill="none"/>
          <path d="M5 5.5v5" stroke="currentColor" strokeWidth="1" fill="none"/>
        </svg>
        main
      </StatusSegment>
      <div style={{ flex: 1 }} />

      {/* Diagnostics indicator */}
      {errorCount > 0 && (
        <StatusSegment>
          <span style={{ color: 'var(--accent-red)' }}>{errorCount} errors</span>
        </StatusSegment>
      )}
      {warningCount > 0 && (
        <StatusSegment>
          <span style={{ color: 'var(--accent-yellow)' }}>{warningCount} warn</span>
        </StatusSegment>
      )}
      {errorCount === 0 && warningCount === 0 && activeTab && (
        <StatusSegment>
          <span style={{ color: 'var(--accent-green)' }}>OK</span>
        </StatusSegment>
      )}

      {/* Right side segments */}
      {activeTab && (
        <>
          <StatusSegment>Ln {cursorLine}, Col {cursorCol}</StatusSegment>
          <StatusSegment>LF</StatusSegment>
          <StatusSegment>UTF-8</StatusSegment>
          <StatusSegment>VoidScript</StatusSegment>
        </>
      )}
    </div>
  );
}

function StatusSegment({ children }: { children: React.ReactNode }) {
  return (
    <div style={{
      padding: '0 8px',
      height: '100%',
      display: 'flex',
      alignItems: 'center',
      cursor: 'pointer',
    }}
      onMouseEnter={(e) => { e.currentTarget.style.backgroundColor = 'var(--bg-hover)'; e.currentTarget.style.color = 'var(--text-primary)'; }}
      onMouseLeave={(e) => { e.currentTarget.style.backgroundColor = 'transparent'; e.currentTarget.style.color = 'var(--text-secondary)'; }}
    >
      {children}
    </div>
  );
}
