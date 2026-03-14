import styles from './ToolBtn.module.css';

interface ToolBtnProps {
  title: string;
  onClick?: () => void;
  disabled?: boolean;
  active?: boolean;
  size?: 'default' | 'small';
  variant?: 'ghost' | 'filled';
  bgColor?: string;
  hoverBgColor?: string;
  iconColor?: string;
  className?: string;
  children: React.ReactNode;
}

export function ToolBtn({
  title,
  onClick,
  disabled,
  active,
  size = 'default',
  variant = 'ghost',
  bgColor,
  hoverBgColor,
  iconColor,
  className,
  children,
}: ToolBtnProps) {
  const classes = [
    styles.btn,
    styles[size],
    active ? styles.active : '',
    variant === 'filled' ? styles.filled : '',
    className ?? '',
  ]
    .filter(Boolean)
    .join(' ');

  const inlineStyle: React.CSSProperties & Record<string, string> = {};
  if (variant === 'filled' && bgColor) {
    inlineStyle['--_btn-bg'] = bgColor;
  }
  if (variant === 'filled' && hoverBgColor) {
    inlineStyle['--_btn-hover-bg'] = hoverBgColor;
  }
  if (iconColor) {
    inlineStyle.color = iconColor;
  }

  return (
    <button
      className={classes}
      title={title}
      onClick={onClick}
      disabled={disabled}
      style={Object.keys(inlineStyle).length > 0 ? inlineStyle : undefined}
    >
      {children}
    </button>
  );
}
