import { useRef, useEffect } from 'react';
import { useStore } from '../state/store';

export function Console() {
  const consoleOutput = useStore((s) => s.consoleOutput);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [consoleOutput.length]);

  return (
    <div style={{
      flex: 1,
      overflow: 'auto',
      padding: '6px 12px',
      fontSize: '12px',
      lineHeight: '1.6',
      fontFamily: 'var(--font-mono)',
      backgroundColor: 'var(--bg-editor)',
    }}>
      {consoleOutput.map((entry, i) => (
        <div key={i} style={{
          color: entry.level === 'error' ? 'var(--accent-red)' : entry.level === 'warn' ? 'var(--accent-yellow)' : 'var(--text-primary)',
          whiteSpace: 'pre-wrap',
          wordBreak: 'break-all',
        }}>
          {entry.text}
        </div>
      ))}
      {consoleOutput.length === 0 && (
        <div style={{ color: 'var(--text-disabled)', fontStyle: 'italic', padding: '8px 0' }}>
          Run a script to see output here
        </div>
      )}
      <div ref={bottomRef} />
    </div>
  );
}
