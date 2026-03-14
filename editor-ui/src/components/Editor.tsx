import { useEffect, useRef, useCallback } from 'react';
import { EditorView, keymap, lineNumbers, highlightActiveLine, drawSelection, gutter, GutterMarker, Decoration, type DecorationSet } from '@codemirror/view';
import { EditorState, StateField, StateEffect, RangeSet } from '@codemirror/state';
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

// --- Breakpoint gutter ---

class BreakpointMarker extends GutterMarker {
  toDOM() {
    const el = document.createElement('div');
    el.style.color = '#f85149';
    el.style.fontSize = '14px';
    el.style.lineHeight = '1';
    el.style.cursor = 'pointer';
    el.textContent = '\u25CF';
    return el;
  }
}

const breakpointMarker = new BreakpointMarker();

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

function createBreakpointGutter(scriptId: string) {
  return gutter({
    class: 'cm-breakpoint-gutter',
    markers: (v) => v.state.field(breakpointState),
    initialSpacer: () => breakpointMarker,
    domEventHandlers: {
      mousedown(view, line) {
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
      },
    },
  });
}

// --- Debug line highlighting ---

const setDebugLineEffect = StateEffect.define<number | null>();

const debugLineDecoration = Decoration.line({ attributes: { style: 'background-color: #264d00' } });

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

export function Editor() {
  const containerRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const activeTabId = useStore((s) => s.activeTabId);
  const tabs = useStore((s) => s.tabs);
  const activeTab = tabs.find((t) => t.scriptId === activeTabId);
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const debugLine = useStore((s) => s.debugLine);
  const isDebugging = useStore((s) => s.isDebugging);

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

    // Destroy previous editor
    if (viewRef.current) {
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

    const state = EditorState.create({
      doc: activeTab.content,
      extensions: [
        breakpointState,
        createBreakpointGutter(activeTab.scriptId),
        debugLineField,
        lineNumbers(),
        highlightActiveLine(),
        drawSelection(),
        history(),
        indentOnInput(),
        bracketMatching(),
        closeBrackets(),
        foldGutter(),
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
              sendToRust({ type: 'script_save', script_id: activeTab.scriptId, content });
              useStore.getState().updateContent(activeTab.scriptId, content);
              return true;
            },
          },
          ...defaultKeymap,
          ...historyKeymap,
          ...closeBracketsKeymap,
          ...completionKeymap,
        ]),
        handleUpdate(activeTab.scriptId),
      ],
    });

    viewRef.current = new EditorView({
      state,
      parent: containerRef.current,
    });

    return () => {
      if (viewRef.current) {
        viewRef.current.destroy();
        viewRef.current = null;
      }
    };
  }, [activeTabId, activeTab?.scriptId, handleUpdate]);

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
        color: '#5a5d63',
        fontSize: '16px',
        fontStyle: 'italic',
      }}>
        Select a script to begin editing
      </div>
    );
  }

  return <div ref={containerRef} style={{ flex: 1, overflow: 'hidden' }} />;
}
