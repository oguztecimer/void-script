import { useStore } from '../state/store';
import { sendToRust } from '../ipc/bridge';
import { PanelHeader } from '../primitives/PanelHeader';
import { ToolBtn } from '../primitives/ToolBtn';
import type { ScriptInfo } from '../ipc/types';
import styles from './ScriptList.module.css';

function SoulIcon() {
  return (
    <svg className={styles.soulIcon} width="14" height="14" viewBox="0 0 16 16" fill="none">
      <path d="M8 14V8M8 8C8 8 6 7 5 5.5C4 4 4.5 2 6 1.5C7.5 1 8 2.5 8 2.5C8 2.5 8.5 1 10 1.5C11.5 2 12 4 11 5.5C10 7 8 8 8 8Z" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" strokeLinejoin="round"/>
      <path d="M5 5.5C4 5.5 2.5 6 2.5 8C2.5 10 4 10.5 5 10.5" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round"/>
      <path d="M11 5.5C12 5.5 13.5 6 13.5 8C13.5 10 12 10.5 11 10.5" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round"/>
      <path d="M5 10.5C4.5 11.5 5 13 6.5 13.5" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round"/>
      <path d="M11 10.5C11.5 11.5 11 13 9.5 13.5" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round"/>
    </svg>
  );
}

function sortScripts(scripts: ScriptInfo[]) {
  return [...scripts].sort((a, b) => {
    const aIsSoul = a.script_type === 'type_soul';
    const bIsSoul = b.script_type === 'type_soul';
    // grimoire always first
    if (a.name === 'grimoire') return -1;
    if (b.name === 'grimoire') return 1;
    // souls before non-souls
    if (aIsSoul && !bIsSoul) return -1;
    if (!aIsSoul && bIsSoul) return 1;
    // alphabetical within group
    return a.name.localeCompare(b.name);
  });
}

export function ScriptList() {
  const scriptList = useStore((s) => s.scriptList);
  const sorted = sortScripts(scriptList);

  return (
    <div className={styles.panel}>
      <PanelHeader
        title="Grimoire"
        actions={
          <>
            <ToolBtn size="small" onClick={() => sendToRust({ type: 'create_script' })} title="Add Page">
              <svg width="12" height="12" viewBox="0 0 16 16">
                <path d="M8 3v10M3 8h10" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"/>
              </svg>
            </ToolBtn>
          </>
        }
      />

      <div className={styles.list}>
        {sorted.map((script) => (
          <div
            key={script.id}
            className={styles.scriptItem}
            onClick={() => sendToRust({ type: 'script_request', script_id: script.id })}
          >
            {script.script_type === 'type_soul' && <SoulIcon />}
            <span>{script.name}</span>
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
