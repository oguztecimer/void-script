---
status: diagnosed
trigger: "BreadcrumbBar doesn't show function_name when cursor is inside a def block; doesn't update when moving between def blocks"
created: 2026-03-15T00:00:00Z
updated: 2026-03-15T00:00:00Z
---

## Current Focus

hypothesis: Two independent bugs: (1) cursorLine is never reset to 1 when switching tabs, so stale line number is used; (2) the regex is correct for VoidScript syntax but cursorLine is not updated on initial view creation (no selectionSet event fires until user moves the cursor)
test: Confirmed by reading all relevant source files
expecting: See Resolution
next_action: diagnosis reported

## Symptoms

expected: BreadcrumbBar shows `filename.vs › function_name` when cursor is inside a def block
actual: BreadcrumbBar only shows `filename.vs`; does not update when moving between def blocks
errors: none (silent logic failure)
reproduction: Open a .vs file with a def block, place cursor inside it
started: unknown / feature not working

## Eliminated

- hypothesis: DEF_RE regex does not match VoidScript def syntax
  evidence: VoidScript uses `def odfm():` and `def main():` — the regex `/^\s*def\s+([a-zA-Z_]\w*)\s*\(/` requires a `(` after the name which IS present in both sample files. The regex correctly captures `odfm` and `main`.
  timestamp: 2026-03-15T00:00:00Z

## Evidence

- timestamp: 2026-03-15T00:00:00Z
  checked: editor-ui/src/components/Editor.tsx — handleUpdate callback, lines 173-191
  found: |
    setCursor is only called inside `if (update.selectionSet)`. This fires when the
    CodeMirror selection changes. It does NOT fire on initial view creation. When a new
    EditorView is constructed (line 249) there is no dispatch that triggers selectionSet,
    so the cursor position reported to the store at that moment is whatever was left in
    the store from the previous view.
  implication: |
    When the user first opens a file or switches tabs, cursorLine in the store retains
    the value from the previous session/tab. The breadcrumb reads this stale value and
    may show the wrong function (or no function) until the user physically moves the cursor.

- timestamp: 2026-03-15T00:00:00Z
  checked: editor-ui/src/state/store.ts — initial state, line 65
  found: cursorLine initialises to 1, cursorCol to 1
  implication: |
    On app start the stored cursor is (1,1). This is fine for the very first file opened.
    But on subsequent tab switches the value is NOT reset — it keeps the line from the
    last tab. If the previous tab had the cursor on line 20 and the new tab's def starts
    on line 1, findEnclosingFunction will scan backwards from line 20 on the new content,
    which may accidentally land inside a def or produce wrong output.

- timestamp: 2026-03-15T00:00:00Z
  checked: editor-ui/src/state/store.ts — switchTab / openTab, lines 80-105
  found: |
    Neither switchTab (line 105) nor openTab (lines 80-90) calls setCursor or resets
    cursorLine/cursorCol. The cursor position is fully decoupled from tab switching.
  implication: |
    Every tab switch leaves a stale cursorLine in the store until the user moves the
    cursor inside the new tab. findEnclosingFunction receives the wrong line number.

- timestamp: 2026-03-15T00:00:00Z
  checked: editor-ui/src/components/Editor.tsx — useEffect that builds the view, lines 193-268
  found: |
    After the new EditorView is created (line 249), the code does NOT call
    `useStore.getState().setCursor(...)` with the actual cursor position from the
    restored or fresh EditorState. It also does not dispatch any effect that would
    trigger selectionSet on the update listener.
  implication: |
    The store's cursorLine is never synchronised at view-creation time. The first
    selectionSet event only arrives when the user clicks or moves the cursor.

- timestamp: 2026-03-15T00:00:00Z
  checked: DEF_RE regex vs VoidScript sample files
  found: |
    miner_brain.vs line 1:  `def odfm():`
    mothership_brain.vs line 4: `def main():`
    Regex: /^\s*def\s+([a-zA-Z_]\w*)\s*\(/
    Both lines start with `def `, then a valid identifier, then `(` — the regex matches.
  implication: The regex is NOT the problem. It correctly handles VoidScript def syntax.

- timestamp: 2026-03-15T00:00:00Z
  checked: editor-ui/src/components/BreadcrumbBar.tsx — findEnclosingFunction, lines 7-16
  found: |
    The function scans backwards from `cursorLine - 1` (0-indexed) to 0 looking for
    any line matching DEF_RE. If cursorLine is stale (e.g. 1 from initial state), the
    scan only checks line 0 of the content, missing all def blocks below line 1.
    If cursorLine is stale from a previous tab (e.g. 20) but the current file only has
    5 lines, `Math.min(cursorLine - 1, lines.length - 1)` clamps correctly but the
    wrong content is still scanned with the wrong conceptual line.
  implication: |
    When cursorLine is 1 (the default / not-yet-updated value), the backward scan
    starts at index 0. Only line 1 of the file is checked. For both sample files the
    def is ON line 1 (miner_brain.vs) or line 4 (mothership_brain.vs). For miner_brain
    the scan will find `def odfm():` on line 1 immediately. For mothership_brain (def
    on line 4) the scan from index 0 finds nothing → fnName is null → no breadcrumb segment.

## Resolution

root_cause: |
  TWO compounding bugs, both in the same flow:

  BUG 1 — cursorLine not reset on tab switch (primary cause of "doesn't update between defs"):
    Location: editor-ui/src/state/store.ts, switchTab (line 105) and openTab (lines 80-90)
    Neither action resets cursorLine/cursorCol to 1. The store keeps the cursor from the
    previous tab. findEnclosingFunction in BreadcrumbBar.tsx then searches the new file's
    content at the old tab's line number, finding the wrong function or none at all.

  BUG 2 — cursorLine not synchronised on initial view creation (primary cause of "doesn't show on first open"):
    Location: editor-ui/src/components/Editor.tsx, inside the useEffect at line 193,
    specifically after `viewRef.current = new EditorView(...)` at line 249.
    After creating (or restoring) the EditorView, no setCursor call is made with the
    actual cursor position from the EditorState. setCursor is only called inside the
    updateListener's `if (update.selectionSet)` branch (line 185), which fires only
    on user-driven selection changes — not on view construction.
    Result: the store's cursorLine stays at whatever value it had (initial 1 or stale
    previous-tab value) until the user moves the cursor.

  REGEX STATUS — not a bug:
    DEF_RE = /^\s*def\s+([a-zA-Z_]\w*)\s*\(/ correctly matches VoidScript def syntax.
    Both sample scripts use `def name():` which satisfies the regex. The regex is fine.

fix: not applied (diagnose-only mode)
verification: n/a
files_changed: []

## Suggested Fix Directions

  Fix for BUG 1 (tab switch — store):
    In switchTab and openTab in store.ts, reset cursorLine and cursorCol to 1
    whenever the active tab changes:
      switchTab: (scriptId) => set({ activeTabId: scriptId, cursorLine: 1, cursorCol: 1 }),
    And in openTab, include cursorLine: 1, cursorCol: 1 in the returned state.

  Fix for BUG 2 (view creation — Editor.tsx):
    After constructing the new EditorView (after line 249), read the cursor from the
    restored EditorState and push it to the store:
      const sel = editorState.selection.main;
      const ln = editorState.doc.lineAt(sel.head);
      useStore.getState().setCursor(ln.number, sel.head - ln.from + 1);
    This ensures the store always reflects the real cursor when a tab is activated,
    even before the user moves the cursor.
