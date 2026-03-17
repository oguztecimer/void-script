import { useRef, useEffect, useState, useCallback } from 'react';
import { useStore } from '../state/store';
import { sendToRust } from '../ipc/bridge';
import styles from './Console.module.css';

/** Highlight function calls like `foo()` in console output */
function highlightCode(text: string, glow?: boolean): React.ReactNode {
  const parts = text.split(/(\w+\(\))/g);
  if (parts.length === 1) return text;
  return parts.map((part, i) =>
    /^\w+\(\)$/.test(part)
      ? <span key={i} className={glow ? styles.codeGlow : styles.code}>{part}</span>
      : part
  );
}

const levelClass: Record<string, string> = {
  error: styles.error,
  warn: styles.warn,
  info: styles.info,
};

export function Console() {
  const consoleOutput = useStore((s) => s.consoleOutput);
  const addConsoleOutput = useStore((s) => s.addConsoleOutput);
  const tier = useStore((s) => s.tier);
  const bottomRef = useRef<HTMLDivElement>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const [command, setCommand] = useState('');

  useEffect(() => {
    const el = scrollRef.current;
    if (el) {
      el.scrollTop = el.scrollHeight;
    } else {
      bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
    }
  }, [consoleOutput.length]);

  // Auto-focus the input in T0
  useEffect(() => {
    if (tier === 0) {
      inputRef.current?.focus();
    }
  }, [tier]);

  const handleSubmit = useCallback((e: React.FormEvent) => {
    e.preventDefault();
    const trimmed = command.trim();
    if (!trimmed) return;
    addConsoleOutput(`> ${trimmed}`, 'info');
    sendToRust({ type: 'console_command', command: trimmed });
    setCommand('');
  }, [command, addConsoleOutput]);

  // Keep focus on input when clicking anywhere in T0 console
  const handleConsoleClick = useCallback(() => {
    if (tier === 0) {
      inputRef.current?.focus();
    }
  }, [tier]);

  if (tier === 0) {
    return (
      <div className={styles.consoleTier0} onClick={handleConsoleClick}>
        <div ref={scrollRef} className={styles.tier0Scroll}>
          <div className={styles.tier0Spacer} />
          {consoleOutput.map((entry, i) => (
            <div key={i} className={`${styles.entry} ${levelClass[entry.level] || styles.info}`}>
              {highlightCode(entry.text, i === 1)}
            </div>
          ))}
          <div ref={bottomRef} />
        </div>
        <form className={styles.prompt} onSubmit={handleSubmit}>
          <span className={styles.promptChar}>{'>'}</span>
          <input
            ref={inputRef}
            className={styles.promptInput}
            value={command}
            onChange={(e) => setCommand(e.target.value)}
            spellCheck={false}
            autoComplete="off"
          />
        </form>
      </div>
    );
  }

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
