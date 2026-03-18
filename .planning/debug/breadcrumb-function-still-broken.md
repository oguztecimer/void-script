---
status: investigating
trigger: "Breadcrumb function name not showing despite cursor sync fixes"
created: 2026-03-15T22:45:00Z
updated: 2026-03-16T00:00:00Z
---

## Current Focus

hypothesis: All code is provably correct through exhaustive analysis. The issue is either (a) not yet rebuilt/tested after fixes, or (b) a runtime-only issue that only manifests during actual interaction. Need empirical verification.
test: Rebuild dist + cargo and test interactively, with diagnostic overlay if needed
expecting: If the feature works after proper rebuild, issue was operational. If still broken, the diagnostic overlay will reveal exactly which data value is wrong at runtime.
next_action: CHECKPOINT -- need user to confirm they rebuilt AND tested with cursor placed inside a def block

## Symptoms

expected: BreadcrumbBar shows `filename.vs > function_name` when cursor is inside a def block
actual: Function name never shows
errors: None reported
reproduction: Open any script, move cursor inside a def block, breadcrumb only shows filename
started: After applying cursor sync fixes (commits 233f381, 22f4d14)

## Eliminated

- hypothesis: findEnclosingFunction regex doesn't match def lines
  evidence: Tested regex against both miner_brain.vs and mothership_brain.vs content via node -- matches correctly for all def lines
  timestamp: 2026-03-15T22:50:00Z

- hypothesis: findEnclosingFunction logic returns null incorrectly
  evidence: Tested full function with mothership_brain.vs content at cursorLine 5, 7 -- correctly returns "main". With miner_brain.vs at line 1, 2 -- correctly returns "odfm".
  timestamp: 2026-03-15T22:50:00Z

- hypothesis: DEF_RE regex has g flag causing exec() to misbehave
  evidence: Regex is /^\s*def\s+([a-zA-Z_]\w*)\s*\(/ with no g flag
  timestamp: 2026-03-15T22:52:00Z

- hypothesis: Built dist doesn't include BreadcrumbBar code
  evidence: Found minified BreadcrumbBar in built JS with correct regex, cursorLine subscription, useMemo, and conditional rendering
  timestamp: 2026-03-15T22:55:00Z

- hypothesis: switchTab/openTab don't reset cursorLine
  evidence: Verified commit diff 233f381 and current code
  timestamp: 2026-03-15T22:57:00Z

- hypothesis: setCursor not called after EditorView construction
  evidence: Verified commit diff 22f4d14 and current code (lines 254-260 of Editor.tsx)
  timestamp: 2026-03-15T22:58:00Z

- hypothesis: zustand v5 store setState doesn't notify subscribers
  evidence: Read zustand/vanilla.js source. Also ran end-to-end zustand test confirming notifications fire correctly.
  timestamp: 2026-03-15T23:05:00Z

- hypothesis: useSyncExternalStore doesn't re-render when cursorLine changes
  evidence: Read zustand/react.js source -- uses Object.is comparison on selector results
  timestamp: 2026-03-15T23:07:00Z

- hypothesis: CSS hides the function name
  evidence: BreadcrumbBar.module.css uses visible colors, no overflow:hidden on the bar itself
  timestamp: 2026-03-15T23:10:00Z

- hypothesis: Rust IPC sends empty content
  evidence: Rust reads content via std::fs::read_to_string and serializes with serde_json
  timestamp: 2026-03-15T23:15:00Z

- hypothesis: React.memo prevents BreadcrumbBar re-render
  evidence: No React.memo used anywhere in codebase
  timestamp: 2026-03-15T23:17:00Z

- hypothesis: Editor useEffect re-runs on cursor click, resetting cursorLine to 1
  evidence: useEffect depends on [activeTabId, activeTab?.scriptId, handleUpdate] -- clicking doesn't change these
  timestamp: 2026-03-15T23:20:00Z

- hypothesis: updateContent called with empty string on initialization
  evidence: EditorView creation doesn't trigger docChanged
  timestamp: 2026-03-15T23:22:00Z

- hypothesis: Cargo binary needs rebuild to serve latest dist
  evidence: rust-embed v8 in debug mode reads from filesystem at absolute path. No cargo rebuild needed for dist changes.
  timestamp: 2026-03-15T23:35:00Z

- hypothesis: Known React 19 useMemo bug
  evidence: Web search found no documented useMemo recomputation bug in React 19
  timestamp: 2026-03-15T23:48:00Z

## Evidence

- timestamp: 2026-03-15T22:48:00Z
  checked: BreadcrumbBar.tsx findEnclosingFunction implementation
  found: Logic correct. Regex /^\s*def\s+([a-zA-Z_]\w*)\s*\(/ matches def lines. Searches backwards from cursorLine.
  implication: Not the problem

- timestamp: 2026-03-15T22:50:00Z
  checked: Node.js test of findEnclosingFunction with real script content
  found: Correctly returns "main" for lines 5,7 in mothership; "odfm" for lines 1,2 in miner; null for lines before any def
  implication: Function works perfectly in isolation

- timestamp: 2026-03-15T22:55:00Z
  checked: Built dist JS for BreadcrumbBar code
  found: Minified code matches source logic exactly. Regex present as /^\s*def\s+([a-zA-Z_]\w*)\s*\(/. Conditional rendering with !==null check.
  implication: Build is correct

- timestamp: 2026-03-15T23:00:00Z
  checked: Commit diffs 233f381 and 22f4d14
  found: Both fixes applied correctly. switchTab/openTab reset cursorLine. setCursor called after EditorView creation.
  implication: Fixes are in the code

- timestamp: 2026-03-15T23:03:00Z
  checked: Editor.tsx handleUpdate callback
  found: updateListener checks selectionSet, computes line from selection.main.head, calls setCursor
  implication: Should fire on every cursor move

- timestamp: 2026-03-15T23:12:00Z
  checked: Full zustand store simulation
  found: openTab -> setCursor(1,1) -> setCursor(2,5): all produce correct state. Store notifies on each change. fnName correctly computed.
  implication: Data flow is correct end-to-end when data is correct

- timestamp: 2026-03-15T23:25:00Z
  checked: rust-embed v8 debug mode behavior
  found: Reads from filesystem at absolute path resolved at compile time. Files served fresh on each request.
  implication: Latest dist files are always served

- timestamp: 2026-03-15T23:45:00Z
  checked: Complete rendering tree (App -> center panel -> BreadcrumbBar)
  found: BreadcrumbBar is rendered unconditionally in the center panel. No React.memo, no conditional wrapper that would prevent rendering.
  implication: Component renders when any subscribed store value changes

## Resolution

root_cause: PENDING -- 14 hypotheses eliminated through code analysis. All code paths verified correct. Need runtime empirical testing to identify the actual failure point.
fix:
verification:
files_changed: []
