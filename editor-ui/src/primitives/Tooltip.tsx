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
  const [offsetX, setOffsetX] = useState(0);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const tooltipRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (visible && tooltipRef.current) {
      const rect = tooltipRef.current.getBoundingClientRect();

      // Vertical flip (existing behavior — preserve exactly)
      if (rect.bottom > window.innerHeight) {
        setFlipped(true);
      } else {
        setFlipped(false);
      }

      // Horizontal clamping — compute corrective delta to keep tooltip in viewport
      const MARGIN = 4; // px gap from viewport edge
      let dx = 0;
      if (rect.left < MARGIN) {
        dx = MARGIN - rect.left;          // shift right
      } else if (rect.right > window.innerWidth - MARGIN) {
        dx = (window.innerWidth - MARGIN) - rect.right;  // shift left (negative)
      }
      setOffsetX(dx);
    }
    if (!visible) {
      setFlipped(false);
      setOffsetX(0);
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
          style={offsetX !== 0 ? { transform: `translateX(calc(-50% + ${offsetX}px))` } : undefined}
        >
          {content}
        </div>
      )}
    </div>
  );
}
