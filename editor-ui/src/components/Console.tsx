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
      padding: '4px 12px',
      fontSize: '12px',
      lineHeight: '1.7',
      fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
      backgroundColor: '#1e1f22',
    }}>
      {consoleOutput.map((entry, i) => (
        <div key={i} style={{
          color: entry.level === 'error' ? '#ef5350' : entry.level === 'warn' ? '#e2a42b' : '#bcbec4',
          whiteSpace: 'pre-wrap',
          wordBreak: 'break-all',
        }}>
          {entry.text}
        </div>
      ))}
      {consoleOutput.length === 0 && (
        <div style={{ color: '#5a5d63', fontStyle: 'italic', padding: '8px 0' }}>
          Run a script to see output here
        </div>
      )}
      <div ref={bottomRef} />
    </div>
  );
}
