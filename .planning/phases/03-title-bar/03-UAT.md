---
status: complete
phase: 03-title-bar
source: [03-01-SUMMARY.md]
started: 2026-03-15T23:12:00Z
updated: 2026-03-15T23:16:00Z
---

## Current Test

[testing complete]

## Tests

### 1. Toolbar Widget Layout
expected: Title bar shows widgets in this order from left to right: traffic lights, hamburger menu icon, back/forward arrows, project name widget, VCS branch widget, spacer, run config selector, action buttons (Run/Debug/Stop), search pill, settings gear icon.
result: pass

### 2. Search Pill
expected: A 220px rounded pill is visible in the toolbar with a magnifying glass icon, "Search" text, and a muted "shift-shift" hint on the right side.
result: pass

### 3. Window Dragging
expected: Clicking and dragging on empty toolbar space (between widgets) moves the entire window. Works on the first click even if the window was not focused.
result: issue
reported: "no it doesnt move at all"
severity: blocker

### 4. Toolbar Button Hover
expected: Hovering over toolbar icon buttons (hamburger, back, forward, settings, etc.) shows a visible hover highlight. Buttons in the drag zone are still clickable.
result: pass

## Summary

total: 4
passed: 3
issues: 1
pending: 0
skipped: 0

## Gaps

- truth: "Clicking and dragging on empty toolbar space moves the entire window, works on first click even if unfocused"
  status: failed
  reason: "User reported: no it doesnt move at all"
  severity: blocker
  test: 3
  root_cause: ""
  artifacts: []
  missing: []
  debug_session: ""
