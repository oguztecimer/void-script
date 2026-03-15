---
status: complete
phase: 09-polish-and-tooltips
source: [09-01-SUMMARY.md, 09-02-SUMMARY.md]
started: 2026-03-15T18:10:00Z
updated: 2026-03-15T22:55:00Z
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

### 4. Tooltip viewport flip near edges
expected: If a button is near the left/right/bottom edge of the viewport, the tooltip should stay fully visible — clamped with a small margin from the edge, not clipped off-screen.
result: pass

### 5. Breadcrumb bar visible below tab strip
expected: A 24px-tall breadcrumb bar appears between the tab strip and the editor area. It has a dark panel background consistent with the Rider theme and a bottom border separator.
result: pass

### 6. Breadcrumb shows filename and function context
expected: Open a .vs file and place the cursor inside a `def` block. The breadcrumb should show `filename.vs › function_name`. Move the cursor outside any def block — it should show just `filename.vs`.
result: pass

### 7. Breadcrumb updates on cursor movement
expected: Move the cursor between different `def` blocks in the same file. The breadcrumb function name updates to reflect the enclosing function. Switching tabs updates the filename.
result: pass

## Summary

total: 7
passed: 7
issues: 0
pending: 0
skipped: 0

## Gaps

[none]
