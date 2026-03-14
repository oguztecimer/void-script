import { ToolBtn } from '../primitives/ToolBtn';
import styles from './ToolStrip.module.css';

export interface ToolStripItem {
  id: string;
  icon: React.ReactNode;
  label: string;
  shortcut?: string;
}

interface Props {
  side: 'left' | 'right';
  items: ToolStripItem[];
  activeId: string | null;
  onToggle: (id: string) => void;
}

export function ToolStrip({ side, items, activeId, onToggle }: Props) {
  return (
    <div className={`${styles.strip} ${styles[side]}`}>
      {items.map((item) => {
        const isActive = activeId === item.id;
        return (
          <ToolBtn
            key={item.id}
            size="default"
            onClick={() => onToggle(item.id)}
            title={`${item.label}${item.shortcut ? ` (${item.shortcut})` : ''}`}
            className={isActive ? styles.activeBtn : styles.stripBtn}
          >
            {item.icon}
          </ToolBtn>
        );
      })}
    </div>
  );
}
