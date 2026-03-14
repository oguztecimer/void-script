import styles from './PanelHeader.module.css';

interface PanelHeaderProps {
  title: string;
  actions?: React.ReactNode;
}

export function PanelHeader({ title, actions }: PanelHeaderProps) {
  return (
    <div className={styles.header}>
      <span>{title}</span>
      {actions && <div className={styles.actions}>{actions}</div>}
    </div>
  );
}
