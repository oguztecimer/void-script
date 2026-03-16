import { EditorView } from '@codemirror/view';
import { tags } from '@lezer/highlight';
import { HighlightStyle, syntaxHighlighting } from '@codemirror/language';

export const voidScriptTheme = EditorView.theme({
  '&': {
    color: 'var(--text-primary)',
    backgroundColor: 'var(--bg-editor)',
    height: '100%',
  },
  '.cm-scroller': {
    overscrollBehavior: 'none',
  },
  '.cm-content': {
    fontFamily: 'var(--font-mono)',
    fontSize: '14px',
    lineHeight: '1.6',
    caretColor: 'var(--text-primary)',
    fontVariantLigatures: 'none',
  },
  '.cm-cursor, .cm-dropCursor': {
    borderLeftColor: 'var(--text-primary)',
    borderLeftWidth: '2px',
  },
  '&.cm-focused .cm-selectionBackground, .cm-selectionBackground': {
    backgroundColor: 'var(--bg-selection)',
  },
  '&.cm-has-selection .cm-activeLine': {
    backgroundColor: 'transparent',
  },
  '&.cm-has-selection .cm-activeLineGutter': {
    backgroundColor: 'transparent',
    color: 'var(--text-tertiary)',
  },
  '.cm-gutters': {
    backgroundColor: 'var(--bg-editor) !important',
    color: 'var(--text-tertiary)',
    borderRight: 'none',
    marginTop: '1px',
    zIndex: 201,
  },
  '.cm-activeLineGutter': {
    backgroundColor: 'var(--bg-hover)',
    color: 'var(--text-secondary)',
  },
  '.cm-activeLine': {
    backgroundColor: 'var(--bg-panel)',
  },
  '.cm-foldPlaceholder': {
    backgroundColor: 'var(--bg-hover)',
    color: 'var(--text-primary)',
    border: 'none',
    borderRadius: '0',
  },
  '.cm-tooltip': {
    backgroundColor: 'var(--bg-tooltip)',
    border: '1px solid var(--border-subtle)',
    color: 'var(--text-primary)',
    borderRadius: '0',
  },
  '.cm-tooltip-autocomplete': {
    '& > ul > li[aria-selected]': {
      backgroundColor: 'var(--bg-selection)',
    },
  },
  '.cm-panels': {
    backgroundColor: 'var(--bg-panel)',
    color: 'var(--text-primary)',
  },
  '.cm-searchMatch': {
    backgroundColor: 'var(--bg-selection)',
    outline: '1px solid var(--accent-blue)',
  },
  '.cm-searchMatch.cm-searchMatch-selected': {
    backgroundColor: 'var(--bg-selection)',
  },

  // ── Fold gutter (EDIT-02) ──────────────────────────────────────────────────
  '.cm-foldGutter': {
    width: '14px',
    minWidth: '14px',
  },
  '.cm-foldGutter .cm-gutterElement': {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    padding: '0',
  },
  '.cm-foldGutter .cm-fold-marker': {
    opacity: '0',
    color: 'var(--text-tertiary)',
    fontSize: '10px',
    lineHeight: '1',
    cursor: 'pointer',
    transition: 'opacity var(--transition-hover), color var(--transition-hover)',
    userSelect: 'none',
  },
  '.cm-foldGutter .cm-gutterElement:hover .cm-fold-marker': {
    opacity: '1',
  },
  '.cm-foldGutter .cm-gutterElement:hover .cm-fold-marker:hover': {
    color: 'var(--text-secondary)',
  },

  // ── Breakpoint circle (EDIT-03) ────────────────────────────────────────────
  '.cm-lineNumbers .cm-gutterElement': {
    position: 'relative',
    cursor: 'pointer',
  },
  '.cm-bp-circle': {
    width: '12px',
    height: '12px',
    borderRadius: '50%',
    backgroundColor: 'var(--accent-breakpoint)',
    margin: '0 auto',
  },

  // ── Breakpoint hover preview (faint circle on hover) ──────────────────────
  '.cm-lineNumbers .cm-gutterElement::after': {
    content: '""',
    position: 'absolute',
    top: '50%',
    left: '50%',
    transform: 'translate(-50%, -50%)',
    width: '12px',
    height: '12px',
    borderRadius: '50%',
    backgroundColor: 'var(--accent-breakpoint)',
    opacity: '0',
    transition: 'opacity var(--transition-hover)',
    pointerEvents: 'none',
  },
  '.cm-lineNumbers .cm-gutterElement:hover::after': {
    opacity: '0.25',
  },
}, { dark: true });

export const voidScriptHighlightStyle = syntaxHighlighting(HighlightStyle.define([
  { tag: tags.keyword, color: 'var(--syntax-keyword)' },              // cyan
  { tag: tags.variableName, color: 'var(--syntax-variable)' },         // foreground
  { tag: [tags.function(tags.variableName)], color: 'var(--syntax-function)' },  // amber
  { tag: [tags.definition(tags.variableName)], color: 'var(--syntax-function)' },
  { tag: tags.comment, color: 'var(--syntax-comment)', fontStyle: 'italic' },
  { tag: tags.string, color: 'var(--syntax-string)' },                // green
  { tag: tags.number, color: 'var(--syntax-number)' },                // orange
  { tag: tags.bool, color: 'var(--syntax-number)' },
  { tag: tags.operator, color: 'var(--syntax-operator)' },              // teal
  { tag: [tags.constant(tags.variableName)], color: 'var(--syntax-constant)' },  // purple
  { tag: tags.bracket, color: 'var(--syntax-bracket)' },
  { tag: tags.punctuation, color: 'var(--syntax-bracket)' },
]));
