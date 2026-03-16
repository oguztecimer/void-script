import { StreamLanguage, foldService } from '@codemirror/language';

const keywords = new Set([
  'while', 'if', 'else', 'elif', 'for', 'in', 'def', 'return',
  'and', 'or', 'not', 'break', 'continue', 'pass', 'import', 'from',
]);

const booleans = new Set(['True', 'False', 'None']);

const constants = new Set([
  'NORTH', 'SOUTH', 'EAST', 'WEST',
  'ASTEROID', 'MINER', 'FIGHTER', 'SCOUT', 'HAULER',
  'IRON', 'COPPER', 'SILICON', 'URANIUM', 'CRYSTAL',
]);

const builtinFunctions = new Set([
  'move', 'mine', 'can_mine', 'deposit', 'get_pos', 'scan',
  'get_cargo', 'cargo_full', 'nearest', 'distance', 'attack',
  'flee', 'dock', 'undock', 'transfer', 'build', 'print',
  'get_health', 'get_energy', 'get_shield', 'wait',
  'set_target', 'get_target', 'has_target',
  'get_type', 'get_name', 'get_owner',
]);

// Indentation-based folding for Python-like syntax
export const voidScriptFolding = foldService.of((state, lineStart, lineEnd) => {
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

export const voidScriptLanguage = StreamLanguage.define({
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
      if (builtinFunctions.has(word)) return 'variableName.function';
      return 'variableName';
    }

    // Skip unknown
    stream.next();
    return null;
  },
});
