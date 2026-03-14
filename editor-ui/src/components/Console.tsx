import { useRef, useEffect } from 'react';
import { useStore } from '../state/store';
import styles from './Console.module.css';

const levelClass: Record<string, string> = {
  error: styles.error,
  warn: styles.warn,
  info: styles.info,
};

export function Console() {
  const consoleOutput = useStore((s) => s.consoleOutput);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [consoleOutput.length]);

  return (
    <div className={styles.console}>
      {consoleOutput.map((entry, i) => (
        <div key={i} className={`${styles.entry} ${levelClass[entry.level] || styles.info}`}>
          {entry.text}
        </div>
      ))}
      {consoleOutput.length === 0 && (
        <div className={styles.emptyState}>
          Run a script to see output here
        </div>
      )}
      <div ref={bottomRef} />
    </div>
  );
}
