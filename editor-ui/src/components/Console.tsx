import { useRef, useEffect, useState, useCallback } from 'react';
import { useStore } from '../state/store';
import { sendToRust } from '../ipc/bridge';
import styles from './Console.module.css';

/** Render text with <hl> tags as highlighted spans */
function highlightCode(text: string, glow?: boolean): React.ReactNode {
  const parts = text.split(/(<hl>.*?<\/hl>)/g);
  if (parts.length === 1) return text;
  return parts.map((part, i) => {
    const match = part.match(/^<hl>(.*)<\/hl>$/);
    if (match) {
      return <span key={i} className={glow ? styles.codeGlow : styles.code}>{match[1]}</span>;
    }
    return part;
  });
}

const levelClass: Record<string, string> = {
  error: styles.error,
  warn: styles.warn,
  info: styles.info,
};

export function Console({ variant = 'console' }: { variant?: 'console' | 'terminal' }) {
  const consoleOutput = useStore((s) => s.consoleOutput);
  const terminalOutput = useStore((s) => s.terminalOutput);
  const addTerminalOutput = useStore((s) => s.addTerminalOutput);
  const terminalBusy = useStore((s) => s.terminalBusy);
  const setTerminalBusy = useStore((s) => s.setTerminalBusy);
  const tier = useStore((s) => s.tier);
  const bottomRef = useRef<HTMLDivElement>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const [command, setCommand] = useState('');
  const [history, setHistory] = useState<string[]>([]);
  const [historyIdx, setHistoryIdx] = useState(-1);

  const isTerminal = variant === 'terminal' || tier === 0;
  const output = isTerminal ? terminalOutput : consoleOutput;

  useEffect(() => {
    const el = scrollRef.current;
    if (el) {
      el.scrollTop = el.scrollHeight;
    } else {
      bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
    }
  }, [output.length]);

  // Auto-focus the input in terminal mode and when command finishes
  useEffect(() => {
    if (isTerminal && !terminalBusy) {
      inputRef.current?.focus();
    }
  }, [isTerminal, terminalBusy]);

  const handleSubmit = useCallback((e: React.FormEvent) => {
    e.preventDefault();
    if (terminalBusy) return;
    const trimmed = command.trim();
    if (!trimmed) return;
    addTerminalOutput(`> ${trimmed}`, 'info');
    setTerminalBusy(true);
    sendToRust({ type: 'console_command', command: trimmed });
    setHistory((h) => [...h, trimmed]);
    setHistoryIdx(-1);
    setCommand('');
  }, [command, terminalBusy, addTerminalOutput, setTerminalBusy]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (command.length > 0) return;
    if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (history.length === 0) return;
      const newIdx = historyIdx === -1 ? history.length - 1 : Math.max(0, historyIdx - 1);
      setHistoryIdx(newIdx);
      setCommand(history[newIdx]);
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (historyIdx === -1) return;
      const newIdx = historyIdx + 1;
      if (newIdx >= history.length) {
        setHistoryIdx(-1);
        setCommand('');
      } else {
        setHistoryIdx(newIdx);
        setCommand(history[newIdx]);
      }
    }
  }, [command, history, historyIdx]);

  const handleConsoleClick = useCallback(() => {
    if (isTerminal) {
      inputRef.current?.focus();
    }
  }, [isTerminal]);

  if (isTerminal) {
    return (
      <div className={`${styles.consoleTier0} ${variant === 'terminal' ? styles.editorTerminal : ''}`} onClick={handleConsoleClick}>
        <div ref={scrollRef} className={styles.tier0Scroll}>
          <div className={styles.tier0Spacer} />
          {output.map((entry, i) => (
            <div key={i} className={`${styles.entry} ${levelClass[entry.level] || styles.info}`}>
              {highlightCode(entry.text, tier === 0 && i === 1)}
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
            onKeyDown={handleKeyDown}
            disabled={terminalBusy}
            spellCheck={false}
            autoComplete="off"
          />
        </form>
      </div>
    );
  }

  return (
    <div className={styles.console}>
      {output.map((entry, i) => (
        <div key={i} className={`${styles.entry} ${levelClass[entry.level] || styles.info}`}>
          {entry.text}
        </div>
      ))}
      {output.length === 0 && (
        <div className={styles.emptyState}>
          Run a script to see output here
        </div>
      )}
      <div ref={bottomRef} />
    </div>
  );
}
