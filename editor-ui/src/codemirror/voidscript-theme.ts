import { EditorView } from '@codemirror/view';
import { tags } from '@lezer/highlight';
import { HighlightStyle, syntaxHighlighting } from '@codemirror/language';

const background = '#1e1f22';
const foreground = '#bcbec4';
const selection = '#214283';
const cursor = '#bcbec4';
const gutterBg = '#2b2d30';
const gutterFg = '#6f737a';
const lineHighlight = '#26282e';

export const voidScriptTheme = EditorView.theme({
  '&': {
    color: foreground,
    backgroundColor: background,
    height: '100%',
  },
  '.cm-content': {
    fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
    fontSize: '14px',
    lineHeight: '1.6',
    caretColor: cursor,
  },
  '.cm-cursor, .cm-dropCursor': {
    borderLeftColor: cursor,
    borderLeftWidth: '2px',
  },
  '&.cm-focused .cm-selectionBackground, .cm-selectionBackground': {
    backgroundColor: selection,
  },
  '.cm-gutters': {
    backgroundColor: gutterBg,
    color: gutterFg,
    borderRight: '1px solid #393b40',
  },
  '.cm-activeLineGutter': {
    backgroundColor: lineHighlight,
    color: '#bcbec4',
  },
  '.cm-activeLine': {
    backgroundColor: lineHighlight,
  },
  '.cm-foldPlaceholder': {
    backgroundColor: '#393b40',
    color: '#bcbec4',
    border: 'none',
  },
  '.cm-tooltip': {
    backgroundColor: '#2b2d30',
    border: '1px solid #393b40',
    color: foreground,
  },
  '.cm-tooltip-autocomplete': {
    '& > ul > li[aria-selected]': {
      backgroundColor: '#214283',
    },
  },
  '.cm-panels': {
    backgroundColor: gutterBg,
    color: foreground,
  },
  '.cm-searchMatch': {
    backgroundColor: '#32593d',
    outline: '1px solid #3d6b48',
  },
  '.cm-searchMatch.cm-searchMatch-selected': {
    backgroundColor: '#214283',
  },
}, { dark: true });

export const voidScriptHighlightStyle = syntaxHighlighting(HighlightStyle.define([
  { tag: tags.keyword, color: '#00e5ff' },              // cyan
  { tag: tags.variableName, color: '#bcbec4' },         // foreground
  { tag: [tags.function(tags.variableName)], color: '#ffb347' },  // amber
  { tag: [tags.definition(tags.variableName)], color: '#ffb347' },
  { tag: tags.comment, color: '#7a7e85', fontStyle: 'italic' },
  { tag: tags.string, color: '#98c379' },                // green
  { tag: tags.number, color: '#d19a66' },                // orange
  { tag: tags.bool, color: '#d19a66' },
  { tag: tags.operator, color: '#56b6c2' },              // teal
  { tag: [tags.constant(tags.variableName)], color: '#c678dd' },  // purple
  { tag: tags.bracket, color: '#8b949e' },
  { tag: tags.punctuation, color: '#8b949e' },
]));
