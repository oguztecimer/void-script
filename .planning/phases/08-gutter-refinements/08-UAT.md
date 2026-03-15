---
status: complete
phase: 08-gutter-refinements
source: [08-01-SUMMARY.md]
started: 2026-03-16T00:11:00Z
updated: 2026-03-16T00:14:00Z
---

## Current Test

[testing complete]

## Tests

### 1. Breakpoint Overlay on Line Numbers
expected: Clicking the gutter area next to a line number toggles a red breakpoint circle. The breakpoint replaces/overlays the line number — there is no separate breakpoint column to the left of line numbers.
result: pass

### 2. Breakpoint Hover Preview
expected: Hovering over the gutter on a line without a breakpoint shows a faint red circle (25% opacity) as a preview of where the breakpoint would be placed.
result: issue
reported: "yes but its slightly in a different position that where the breakpoint places"
severity: cosmetic

### 3. Fold Icons on Hover
expected: Code fold icons (triangles ▼/▶) are hidden by default. They only appear when hovering over a foldable line in the gutter. Clicking them folds/unfolds the code block.
result: issue
reported: "they never appear"
severity: major

## Summary

total: 3
passed: 1
issues: 2
pending: 0
skipped: 0

## Gaps

- truth: "Breakpoint hover preview circle appears at the same position as the actual breakpoint"
  status: failed
  reason: "User reported: yes but its slightly in a different position that where the breakpoint places"
  severity: cosmetic
  test: 2
  root_cause: ""
  artifacts: []
  missing: []
  debug_session: ""

- truth: "Fold icons (▼/▶) appear on hover over foldable gutter lines and clicking folds/unfolds code"
  status: failed
  reason: "User reported: they never appear"
  severity: major
  test: 3
  root_cause: ""
  artifacts: []
  missing: []
  debug_session: ""
