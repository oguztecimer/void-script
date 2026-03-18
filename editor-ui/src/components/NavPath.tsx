import { useStore } from '../state/store';
import { TYPE_LABELS } from '../state/scriptTypes';
import styles from './NavPath.module.css';

interface Segment {
  label: string;
  kind: 'project' | 'folder' | 'file';
}

function buildSegments(
  activeTabId: string | null,
  tabs: { scriptId: string; name: string }[],
  scriptList: { id: string; script_type: string }[],
): Segment[] {
  const segments: Segment[] = [{ label: 'Grimoire', kind: 'project' }];

  if (!activeTabId) return segments;

  const activeTab = tabs.find((t) => t.scriptId === activeTabId);
  if (!activeTab) return segments;

  const info = scriptList.find((s) => s.id === activeTabId);
  if (info && info.script_type && TYPE_LABELS[info.script_type]) {
    segments.push({ label: TYPE_LABELS[info.script_type], kind: 'folder' });
  }

  segments.push({ label: activeTab.name, kind: 'file' });
  return segments;
}

export function NavPath() {
  const activeTabId = useStore((s) => s.activeTabId);
  const tabs = useStore((s) => s.tabs);
  const scriptList = useStore((s) => s.scriptList);
  const toggleLeftPanel = useStore((s) => s.toggleLeftPanel);

  const segments = buildSegments(activeTabId, tabs, scriptList);

  return (
    <div className={styles.container}>
      {segments.map((seg, i) => {
        const isLast = i === segments.length - 1;
        const chevron = !isLast ? <span className={styles.chevron}> › </span> : null;

        if (seg.kind === 'project') {
          return (
            <button
              key="project"
              className={styles.segment}
              onClick={toggleLeftPanel}
            >
              {seg.label}
              {chevron}
            </button>
          );
        }

        if (seg.kind === 'folder') {
          return (
            <span key={`folder-${seg.label}`} className={styles.segmentInert}>
              {seg.label}
              {chevron}
            </span>
          );
        }

        // file segment — last, no chevron
        return (
          <span key={`file-${seg.label}`} className={styles.segmentFile}>
            {seg.label}
          </span>
        );
      })}
    </div>
  );
}
