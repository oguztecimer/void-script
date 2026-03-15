import { useEffect, useRef, useCallback } from 'react';
import { EditorView, keymap, lineNumbers, lineNumberMarkers, highlightActiveLine, drawSelection, GutterMarker, Decoration, type BlockInfo, type DecorationSet } from '@codemirror/view';
import { EditorState, StateField, StateEffect, RangeSet, type Extension } from '@codemirror/state';
import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
import { bracketMatching, indentOnInput, foldGutter } from '@codemirror/language';
import { closeBrackets, closeBracketsKeymap } from '@codemirror/autocomplete';
import { autocompletion, completionKeymap } from '@codemirror/autocomplete';
import { linter, type Diagnostic as CmDiagnostic } from '@codemirror/lint';
import { voidScriptLanguage } from '../codemirror/voidscript-lang';
import { voidScriptTheme, voidScriptHighlightStyle } from '../codemirror/voidscript-theme';
import { voidScriptCompletion } from '../codemirror/voidscript-completion';
import { useStore } from '../state/store';
import { sendToRust } from '../ipc/bridge';

// --- EditorState cache ---
// Module-level map so it survives re-renders. Keyed by scriptId.
// Stores the last EditorState and scroll snapshot for each inactive tab.
interface CachedTabState {
  state: EditorState;
  // scrollSnapshot() returns StateEffect<ScrollTarget> — dispatched after view creation
  scrollSnapshot: ReturnType<EditorView['scrollSnapshot']> | null;
}
const editorStates = new Map<string, CachedTabState>();

// --- Breakpoint overlay ---

class BreakpointOverlayMarker extends GutterMarker {
  eq(other: GutterMarker): boolean {
    return other instanceof BreakpointOverlayMarker;
  }
  toDOM(): HTMLElement {
    const el = document.createElement('div');
    el.className = 'cm-bp-circle';
    return el;
  }
}

const breakpointMarker = new BreakpointOverlayMarker();

const toggleBreakpointEffect = StateEffect.define<{ pos: number; on: boolean }>();

const breakpointState = StateField.define<RangeSet<GutterMarker>>({
  create() {
    return RangeSet.empty;
  },
  update(set, transaction) {
    set = set.map(transaction.changes);
    for (const e of transaction.effects) {
      if (e.is(toggleBreakpointEffect)) {
        if (e.value.on) {
          set = set.update({ add: [breakpointMarker.range(e.value.pos)] });
        } else {
          set = set.update({ filter: (from) => from !== e.value.pos });
        }
      }
    }
    return set;
  },
});

// Feed breakpointState markers into the line-number gutter via lineNumberMarkers facet.
// BreakpointOverlayMarker.toDOM() causes lineNumberGutter.lineMarker to return null for
// those rows (suppressing the line number), per @codemirror/view source line 11602.
// computeN derives a RangeSet<GutterMarker> from breakpointState each time it changes.
const breakpointLineNumberMarkers = lineNumberMarkers.computeN(
  [breakpointState],
  (state) => [state.field(breakpointState)]
);

// --- Debug line highlighting ---

const setDebugLineEffect = StateEffect.define<number | null>();

const debugLineDecoration = Decoration.line({ attributes: { style: 'background-color: var(--bg-selection)' } });

const debugLineField = StateField.define<DecorationSet>({
  create() {
    return Decoration.none;
  },
  update(decorations, transaction) {
    decorations = decorations.map(transaction.changes);
    for (const e of transaction.effects) {
      if (e.is(setDebugLineEffect)) {
        if (e.value !== null && e.value >= 1 && e.value <= transaction.state.doc.lines) {
          const line = transaction.state.doc.line(e.value);
          decorations = Decoration.set([debugLineDecoration.range(line.from)]);
        } else {
          decorations = Decoration.none;
        }
      }
    }
    return decorations;
  },
  provide: (f) => EditorView.decorations.from(f),
});

function buildExtensions(
  scriptId: string,
  voidScriptLinter: Extension,
  saveTimerRef: React.MutableRefObject<ReturnType<typeof setTimeout> | null>,
  handleUpdate: (id: string) => Extension,
): Extension[] {
  const handleBreakpointClick = (view: EditorView, line: BlockInfo): boolean => {
    const lineNo = view.state.doc.lineAt(line.from).number;
    const store = useStore.getState();
    const bps = store.breakpoints[scriptId] || [];
    const isSet = bps.includes(lineNo);
    view.dispatch({
      effects: toggleBreakpointEffect.of({ pos: line.from, on: !isSet }),
    });
    store.toggleBreakpoint(scriptId, lineNo);
    sendToRust({ type: 'toggle_breakpoint', script_id: scriptId, line: lineNo });
    return true;
  };

  return [
    breakpointState,
    breakpointLineNumberMarkers,
    debugLineField,
    lineNumbers({ domEventHandlers: { mousedown: handleBreakpointClick } }),
    highlightActiveLine(),
    drawSelection(),
    history(),
    indentOnInput(),
    bracketMatching(),
    closeBrackets(),
    foldGutter({
      markerDOM(open: boolean): HTMLElement {
        const span = document.createElement('span');
        span.className = 'cm-fold-marker';
        span.textContent = open ? '\u25BC' : '\u25B6';
        return span;
      },
    }),
    autocompletion({ override: [voidScriptCompletion] }),
    voidScriptLanguage,
    voidScriptTheme,
    voidScriptHighlightStyle,
    voidScriptLinter,
    keymap.of([
      {
        key: 'Mod-s',
        run: (view) => {
          const content = view.state.doc.toString();
          if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
          sendToRust({ type: 'script_save', script_id: scriptId, content });
          useStore.getState().updateContent(scriptId, content);
          return true;
        },
      },
      ...defaultKeymap,
      ...historyKeymap,
      ...closeBracketsKeymap,
      ...completionKeymap,
    ]),
    handleUpdate(scriptId),
  ];
}

export function Editor() {
  const containerRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const activeTabId = useStore((s) => s.activeTabId);
  const tabs = useStore((s) => s.tabs);
  const activeTab = tabs.find((t) => t.scriptId === activeTabId);
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const debugLine = useStore((s) => s.debugLine);
  const isDebugging = useStore((s) => s.isDebugging);

  // Track the previous active tab ID so we save its state before destroying
  const prevTabIdRef = useRef<string | null>(null);

  const handleUpdate = useCallback((scriptId: string) => {
    return EditorView.updateListener.of((update) => {
      if (update.docChanged) {
        const content = update.state.doc.toString();
        useStore.getState().updateContent(scriptId, content);

        // Debounced auto-save
        if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
        saveTimerRef.current = setTimeout(() => {
          sendToRust({ type: 'script_save', script_id: scriptId, content });
        }, 1000);
      }
      if (update.selectionSet) {
        const pos = update.state.selection.main.head;
        const line = update.state.doc.lineAt(pos);
        useStore.getState().setCursor(line.number, pos - line.from + 1);
      }
    });
  }, []);

  useEffect(() => {
    if (!containerRef.current || !activeTab) return;

    // Save the current view's state before destroying it
    if (viewRef.current) {
      const prevId = prevTabIdRef.current;
      if (prevId) {
        editorStates.set(prevId, {
          state: viewRef.current.state,
          scrollSnapshot: viewRef.current.scrollSnapshot(),
        });
      }
      viewRef.current.destroy();
      viewRef.current = null;
    }

    const diagnostics = activeTab.diagnostics;
    const voidScriptLinter = linter(() => {
      return diagnostics.map((d): CmDiagnostic => ({
        from: 0, // Will be computed properly with line info
        to: 0,
        severity: d.severity === 'error' ? 'error' : d.severity === 'warning' ? 'warning' : 'info',
        message: d.message,
      }));
    });

    // Check if we have a cached state for this tab
    const cached = editorStates.get(activeTab.scriptId);
    let editorState: EditorState;

    if (cached) {
      // Check if content was changed externally (e.g. via IPC) while this tab was inactive
      const cachedContent = cached.state.doc.toString();
      if (cachedContent !== activeTab.content) {
        // Content changed externally — create fresh state to avoid stale doc
        editorState = EditorState.create({
          doc: activeTab.content,
          extensions: buildExtensions(activeTab.scriptId, voidScriptLinter, saveTimerRef, handleUpdate),
        });
      } else {
        // Restore cached state — preserves undo history, selection, and all StateField values.
        // Apply StateEffect.reconfigure so the linter and handleUpdate closures are refreshed.
        editorState = cached.state.update({
          effects: StateEffect.reconfigure.of(
            buildExtensions(activeTab.scriptId, voidScriptLinter, saveTimerRef, handleUpdate)
          ),
        }).state;
      }
    } else {
      // First time opening this tab — create fresh state
      editorState = EditorState.create({
        doc: activeTab.content,
        extensions: buildExtensions(activeTab.scriptId, voidScriptLinter, saveTimerRef, handleUpdate),
      });
    }

    viewRef.current = new EditorView({
      state: editorState,
      parent: containerRef.current,
    });

    // Restore scroll position after view is mounted
    if (cached?.scrollSnapshot) {
      viewRef.current.dispatch({ effects: cached.scrollSnapshot });
    }

    // Update the previous tab ref to the current one
    prevTabIdRef.current = activeTab.scriptId;

    return () => {
      if (viewRef.current) {
        viewRef.current.destroy();
        viewRef.current = null;
      }
    };
  }, [activeTabId, activeTab?.scriptId, handleUpdate]);

  // Clean up cached states for tabs that have been closed
  useEffect(() => {
    const tabIds = new Set(tabs.map((t) => t.scriptId));
    for (const key of editorStates.keys()) {
      if (!tabIds.has(key)) {
        editorStates.delete(key);
      }
    }
  }, [tabs]);

  // Sync debug line highlight
  useEffect(() => {
    if (!viewRef.current) return;
    const line = isDebugging ? debugLine : null;
    viewRef.current.dispatch({
      effects: setDebugLineEffect.of(line),
    });
  }, [debugLine, isDebugging]);

  if (!activeTab) {
    return (
      <div style={{
        flex: 1,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        color: 'var(--text-tertiary)',
        fontSize: '16px',
        fontStyle: 'italic',
      }}>
        Select a script to begin editing
      </div>
    );
  }

  return <div ref={containerRef} style={{ flex: 1, overflow: 'hidden' }} />;
}
