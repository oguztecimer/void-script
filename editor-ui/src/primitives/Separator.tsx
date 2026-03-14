import styles from './Separator.module.css';

interface SeparatorProps {
  variant?: 'line' | 'gap';
  orientation?: 'vertical' | 'horizontal';
  level?: 'default' | 'subtle' | 'strong';
  size?: number;
  gap?: number;
}

const levelClass = {
  default: styles.levelDefault,
  subtle: styles.levelSubtle,
  strong: styles.levelStrong,
} as const;

export function Separator({
  variant = 'line',
  orientation = 'vertical',
  level = 'subtle',
  size,
  gap = 8,
}: SeparatorProps) {
  if (variant === 'gap') {
    const gapStyle: React.CSSProperties =
      orientation === 'vertical'
        ? { marginLeft: gap / 2, marginRight: gap / 2 }
        : { marginTop: gap / 2, marginBottom: gap / 2 };

    return <div className={styles.gap} style={gapStyle} />;
  }

  const classes = [
    styles.line,
    orientation === 'vertical' ? styles.vertical : styles.horizontal,
    levelClass[level],
  ].join(' ');

  const lineStyle: React.CSSProperties = {};
  if (orientation === 'vertical') {
    lineStyle.height = size ?? 16;
  } else if (size !== undefined) {
    lineStyle.width = size;
  }

  return <div className={classes} style={lineStyle} />;
}
