---
status: complete
phase: 09-polish-and-tooltips
source: [09-01-SUMMARY.md, 09-02-SUMMARY.md]
started: 2026-03-15T18:10:00Z
updated: 2026-03-15T18:20:00Z
---

## Current Test

[testing complete]

## Tests

### 1. Tooltip appears on hover with Rider-dark styling
expected: Hover over any toolbar button (e.g. Run, Debug, Stop). After ~800ms delay, a dark tooltip appears with #3C3F41 background, light #BBBBBB text, thin #555555 border. No arrow, no shadow. Tooltip fades in smoothly.
result: pass

### 2. Keyboard shortcut hints in tooltips
expected: Hover over Run button — tooltip shows "Run (Shift+F10)". Hover Debug — "Debug (Shift+F9)". Hover Stop — "Stop (Ctrl+F2)". Step buttons show their shortcuts too (F8, F7, Shift+F8, F9).
result: pass

### 3. No native browser tooltips remain
expected: Hover slowly over all interactive elements (toolbar buttons, traffic lights, search pill, debug variable values). You should never see a native browser tooltip (the plain yellow/system-styled one). Only the custom dark tooltips appear.
result: pass

### 4. Tooltip viewport flip near bottom edge
expected: If a button is near the bottom of the viewport, the tooltip should appear above the element instead of below, preventing it from being clipped off-screen.
result: issue
reported: "yeah but when its near right or left side, it clips off the sides"
severity: minor

### 5. Breadcrumb bar visible below tab strip
expected: A 24px-tall breadcrumb bar appears between the tab strip and the editor area. It has a dark panel background consistent with the Rider theme and a bottom border separator.
result: pass

### 6. Breadcrumb shows filename and function context
expected: Open a .vs file and place the cursor inside a `def` block. The breadcrumb should show `filename.vs › function_name`. Move the cursor outside any def block — it should show just `filename.vs`.
result: issue
reported: "nope it doesnt show the function_name"
severity: major

### 7. Breadcrumb updates on cursor movement
expected: Move the cursor between different `def` blocks in the same file. The breadcrumb function name updates to reflect the enclosing function. Switching tabs updates the filename.
result: issue
reported: "fail"
severity: major

## Summary

total: 7
passed: 4
issues: 3
pending: 0
skipped: 0

## Gaps

- truth: "Tooltip should not clip off left/right edges of the viewport"
  status: failed
  reason: "User reported: yeah but when its near right or left side, it clips off the sides"
  severity: minor
  test: 4
  root_cause: ""
  artifacts: []
  missing: []
  debug_session: ""

- truth: "Breadcrumb shows filename.vs › function_name when cursor is inside a def block"
  status: failed
  reason: "User reported: nope it doesnt show the function_name"
  severity: major
  test: 6
  root_cause: ""
  artifacts: []
  missing: []
  debug_session: ""

- truth: "Breadcrumb function name updates when cursor moves between def blocks"
  status: failed
  reason: "User reported: fail"
  severity: major
  test: 7
  root_cause: ""
  artifacts: []
  missing: []
  debug_session: ""
