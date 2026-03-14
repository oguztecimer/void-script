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
      backgroundColor: '#2b2d30',
      fontSize: '11px',
      color: '#6f737a',
      padding: '0 8px',
      borderTop: '1px solid #1e1f22',
      gap: '0',
    }}>
      {/* Left: VCS branch */}
      <StatusSegment>git: main</StatusSegment>
      <div style={{ flex: 1 }} />

      {/* Right side segments */}
      {activeTab && (
        <>
          <StatusSegment>LF</StatusSegment>
          <StatusSegment>UTF-8</StatusSegment>
          <StatusSegment>Ln {cursorLine}, Col {cursorCol}</StatusSegment>
          <StatusSegment>VoidScript</StatusSegment>
        </>
      )}

      {/* Diagnostics indicator */}
      {errorCount > 0 && (
        <StatusSegment>
          <span style={{ color: '#ef5350' }}>{errorCount} errors</span>
        </StatusSegment>
      )}
      {warningCount > 0 && (
        <StatusSegment>
          <span style={{ color: '#e2a42b' }}>{warningCount} warn</span>
        </StatusSegment>
      )}
      {errorCount === 0 && warningCount === 0 && activeTab && (
        <StatusSegment>
          <span style={{ color: '#57a64a' }}>OK</span>
        </StatusSegment>
      )}
    </div>
  );
}

function StatusSegment({ children }: { children: React.ReactNode }) {
  return (
    <div style={{
      padding: '0 8px',
      borderLeft: '1px solid #393b40',
      height: '100%',
      display: 'flex',
      alignItems: 'center',
      cursor: 'pointer',
    }}
      onMouseEnter={(e) => { e.currentTarget.style.backgroundColor = '#313335'; }}
      onMouseLeave={(e) => { e.currentTarget.style.backgroundColor = 'transparent'; }}
    >
      {children}
    </div>
  );
}
