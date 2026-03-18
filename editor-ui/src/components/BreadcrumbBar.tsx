import { useMemo } from 'react';
import { useStore } from '../state/store';
import styles from './BreadcrumbBar.module.css';

const DEF_RE = /^\s*def\s+([a-zA-Z_]\w*)\s*\(/;

function findEnclosingFunction(content: string, cursorLine: number): string | null {
  if (!content || cursorLine < 1) return null;
  const lines = content.split('\n');
  const startIdx = Math.min(cursorLine - 1, lines.length - 1);
  for (let i = startIdx; i >= 0; i--) {
    const m = DEF_RE.exec(lines[i]);
    if (m) return m[1];
  }
  return null;
}

export function BreadcrumbBar() {
  const cursorLine = useStore((s) => s.cursorLine);
  const activeTabContent = useStore((s) => {
    const t = s.tabs.find((tab) => tab.scriptId === s.activeTabId);
    return t?.content ?? '';
  });
  const activeTabName = useStore((s) => {
    const t = s.tabs.find((tab) => tab.scriptId === s.activeTabId);
    return t?.name ?? null;
  });

  const fnName = useMemo(
    () => findEnclosingFunction(activeTabContent, cursorLine),
    [activeTabContent, cursorLine]
  );

  if (!activeTabName) return null;

  return (
    <div className={styles.bar}>
      <span className={styles.segment}>{activeTabName}.gs</span>
      {fnName !== null && (
        <>
          <span className={styles.chevron}> &rsaquo; </span>
          <span className={styles.segmentActive}>{fnName}</span>
        </>
      )}
    </div>
  );
}
