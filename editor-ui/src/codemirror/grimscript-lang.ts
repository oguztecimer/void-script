import { StreamLanguage, foldService } from '@codemirror/language';
import { useStore } from '../state/store';

const keywords = new Set([
  'while', 'if', 'else', 'elif', 'for', 'in', 'def', 'return',
  'and', 'or', 'not', 'break', 'continue', 'pass', 'import', 'from',
]);

const booleans = new Set(['True', 'False', 'None']);

const constants = new Set<string>([]);

const stdlibFunctions = new Set([
  'print', 'len', 'range', 'abs', 'min', 'max', 'int', 'str', 'type',
]);

const allGameFunctions = new Set([
  'move', 'get_pos', 'scan', 'nearest', 'distance', 'attack',
  'flee', 'wait', 'set_target', 'get_target', 'has_target',
  'get_type', 'get_name', 'get_owner',
  'consult', 'raise', 'harvest', 'pact',
]);

// Indentation-based folding for Python-like syntax
export const grimScriptFolding = foldService.of((state, lineStart, lineEnd) => {
  const line = state.doc.lineAt(lineStart);
  const lineText = line.text;
  const trimmed = lineText.trimEnd();
  if (!trimmed.endsWith(':')) return null;

  const baseIndent = lineText.match(/^(\s*)/)![1].length;
  let lastFoldLine = line.number;

  for (let i = line.number + 1; i <= state.doc.lines; i++) {
    const next = state.doc.line(i);
    const nextText = next.text;
    if (nextText.trim().length === 0) continue; // skip blank lines
    const nextIndent = nextText.match(/^(\s*)/)![1].length;
    if (nextIndent <= baseIndent) break;
    lastFoldLine = i;
  }

  if (lastFoldLine === line.number) return null;
  return { from: lineEnd, to: state.doc.line(lastFoldLine).to };
});

export const grimScriptLanguage = StreamLanguage.define({
  token(stream) {
    // Skip whitespace
    if (stream.eatSpace()) return null;

    // Comments
    if (stream.match('#')) {
      stream.skipToEnd();
      return 'comment';
    }

    // Strings (double-quoted)
    if (stream.match('"')) {
      while (!stream.eol()) {
        if (stream.next() === '"') break;
      }
      return 'string';
    }

    // Strings (single-quoted)
    if (stream.match("'")) {
      while (!stream.eol()) {
        if (stream.next() === "'") break;
      }
      return 'string';
    }

    // Numbers
    if (stream.match(/^-?\d+(\.\d+)?/)) {
      return 'number';
    }

    // Operators
    if (stream.match(/^[+\-*/%=<>!&|^~]+/)) {
      return 'operator';
    }

    // Brackets
    if (stream.match(/^[(){}\[\]]/)) {
      return 'bracket';
    }

    // Punctuation
    if (stream.match(/^[,:;.]/)) {
      return 'punctuation';
    }

    // Words
    if (stream.match(/^[a-zA-Z_]\w*/)) {
      const word = stream.current();
      if (keywords.has(word)) return 'keyword';
      if (booleans.has(word)) return 'bool';
      if (constants.has(word)) return 'variableName.constant';
      if (stdlibFunctions.has(word)) return 'variableName.function';
      if (allGameFunctions.has(word)) {
        const available = useStore.getState().availableCommands;
        if (available.includes(word)) return 'variableName.function';
      }
      // Check custom commands from mods.
      const available = useStore.getState().availableCommands;
      if (available.includes(word) && !allGameFunctions.has(word) && !stdlibFunctions.has(word)) {
        return 'variableName.function';
      }
      return 'variableName';
    }

    // Skip unknown
    stream.next();
    return null;
  },
});
