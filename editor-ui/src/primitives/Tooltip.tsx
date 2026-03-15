import { useState, useRef, useEffect } from 'react';
import styles from './Tooltip.module.css';

interface TooltipProps {
  content: string;
  children: React.ReactNode;
  disabled?: boolean;
}

export function Tooltip({ content, children, disabled }: TooltipProps) {
  const [visible, setVisible] = useState(false);
  const [flipped, setFlipped] = useState(false);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const tooltipRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (visible && tooltipRef.current) {
      const rect = tooltipRef.current.getBoundingClientRect();
      if (rect.bottom > window.innerHeight) {
        setFlipped(true);
      } else {
        setFlipped(false);
      }
    }
    if (!visible) {
      setFlipped(false);
    }
  }, [visible]);

  if (!content || disabled) {
    return <>{children}</>;
  }

  function handleMouseEnter() {
    timerRef.current = setTimeout(() => {
      setVisible(true);
    }, 800);
  }

  function handleMouseLeave() {
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    setVisible(false);
  }

  return (
    <div
      className={styles.wrapper}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      {children}
      {visible && (
        <div
          ref={tooltipRef}
          className={`${styles.tooltip}${flipped ? ` ${styles.flipped}` : ''}`}
        >
          {content}
        </div>
      )}
    </div>
  );
}
