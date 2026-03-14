import styles from './StatusSegment.module.css';

interface StatusSegmentProps {
  icon?: React.ReactNode;
  label: React.ReactNode;
  onClick?: () => void;
}

export function StatusSegment({ icon, label, onClick }: StatusSegmentProps) {
  const Tag = onClick ? 'button' : 'div';

  return (
    <Tag className={styles.segment} onClick={onClick}>
      {icon && <span>{icon}</span>}
      <span>{label}</span>
    </Tag>
  );
}
