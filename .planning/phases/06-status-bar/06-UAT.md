---
status: diagnosed
phase: 06-status-bar
source: [06-01-SUMMARY.md]
started: 2026-03-15T11:00:00Z
updated: 2026-03-15T11:15:00Z
---

## Current Test
<!-- OVERWRITE each test - shows where we are -->

[testing complete]

## Tests

### 1. Navigation Path Shows Breadcrumb
expected: With a script tab open, the status bar left region shows a breadcrumb path like "VOID//SCRIPT › miner_brain.vs" (for flat scripts) or "VOID//SCRIPT › Ship Brains › miner_brain.vs" (for typed/categorized scripts). The chevron separator "›" appears between segments.
result: issue
reported: "where is the navigation path, i cant see it"
severity: major

### 2. No Tab Shows Project Only
expected: With no script tab open, the status bar left region shows just "VOID//SCRIPT" with no chevrons or file segments.
result: skipped
reason: user unsure where to look, will check later

### 3. Project Name Click Toggles Panel
expected: Clicking the "VOID//SCRIPT" text in the status bar breadcrumb toggles the ScriptList panel open/closed.
result: issue
reported: "it does nothing"
severity: major

### 4. Breadcrumb Hover Style
expected: Hovering over the "VOID//SCRIPT" segment changes only the text color (shifts to primary color). No background color change, no underline. Folder and file segments do NOT show hover effects.
result: skipped

### 5. Diagnostics Error Icon
expected: When the active script has errors, the status bar shows a red circle icon followed by the error count number. Not plain text — an actual icon shape.
result: issue
reported: "there is no status bar"
severity: blocker

### 6. Diagnostics Warning Icon
expected: When the active script has warnings, the status bar shows a yellow triangle icon followed by the warning count number. Not plain text — an actual icon shape.
result: issue
reported: "no status bar"
severity: blocker

### 7. Diagnostics OK State
expected: When the active script has no errors and no warnings, the diagnostics area shows an OK/green state instead of error/warning icons.
result: issue
reported: "no status bar"
severity: blocker

### 8. VCS Branch Widget Removed
expected: The status bar no longer shows a git branch name or VCS indicator anywhere.
result: issue
reported: "no status bar"
severity: blocker

### 9. Status Bar Dimensions
expected: The status bar is 24px tall and uses 11px Inter font for its text content.
result: issue
reported: "no status bar"
severity: blocker

## Summary

total: 9
passed: 0
issues: 7
pending: 0
skipped: 2

## Gaps

- truth: "With a script tab open, the status bar left region shows a breadcrumb path like VOID//SCRIPT › miner_brain.vs"
  status: failed
  reason: "User reported: where is the navigation path, i cant see it"
  severity: major
  test: 1
  root_cause: "StatusBar .bar class missing flex-shrink: 0 — .main flex: 1 compresses StatusBar to 0px height"
  artifacts:
    - path: "editor-ui/src/components/StatusBar.module.css"
      issue: "Missing flex-shrink: 0 on .bar rule"
  missing:
    - "Add flex-shrink: 0 to .bar in StatusBar.module.css"
  debug_session: ".planning/debug/statusbar-not-visible.md"
- truth: "Clicking the VOID//SCRIPT text in the status bar breadcrumb toggles the ScriptList panel open/closed"
  status: failed
  reason: "User reported: it does nothing"
  severity: major
  test: 3
  root_cause: "Same as test 1 — StatusBar compressed to 0px, click target invisible"
  artifacts:
    - path: "editor-ui/src/components/StatusBar.module.css"
      issue: "Missing flex-shrink: 0 on .bar rule"
  missing:
    - "Add flex-shrink: 0 to .bar in StatusBar.module.css"
  debug_session: ".planning/debug/statusbar-not-visible.md"
- truth: "When the active script has errors, the status bar shows a red circle icon followed by the error count number"
  status: failed
  reason: "User reported: there is no status bar"
  severity: blocker
  test: 5
  root_cause: "Same as test 1 — StatusBar compressed to 0px"
  artifacts:
    - path: "editor-ui/src/components/StatusBar.module.css"
      issue: "Missing flex-shrink: 0 on .bar rule"
  missing:
    - "Add flex-shrink: 0 to .bar in StatusBar.module.css"
  debug_session: ".planning/debug/statusbar-not-visible.md"
- truth: "When the active script has warnings, the status bar shows a yellow triangle icon followed by the warning count"
  status: failed
  reason: "User reported: no status bar"
  severity: blocker
  test: 6
  root_cause: "Same as test 1 — StatusBar compressed to 0px"
  artifacts:
    - path: "editor-ui/src/components/StatusBar.module.css"
      issue: "Missing flex-shrink: 0 on .bar rule"
  missing:
    - "Add flex-shrink: 0 to .bar in StatusBar.module.css"
  debug_session: ".planning/debug/statusbar-not-visible.md"
- truth: "When no errors/warnings, diagnostics area shows OK/green state"
  status: failed
  reason: "User reported: no status bar"
  severity: blocker
  test: 7
  root_cause: "Same as test 1 — StatusBar compressed to 0px"
  artifacts:
    - path: "editor-ui/src/components/StatusBar.module.css"
      issue: "Missing flex-shrink: 0 on .bar rule"
  missing:
    - "Add flex-shrink: 0 to .bar in StatusBar.module.css"
  debug_session: ".planning/debug/statusbar-not-visible.md"
- truth: "The status bar no longer shows a git branch name or VCS indicator"
  status: failed
  reason: "User reported: no status bar"
  severity: blocker
  test: 8
  root_cause: "Same as test 1 — StatusBar compressed to 0px"
  artifacts:
    - path: "editor-ui/src/components/StatusBar.module.css"
      issue: "Missing flex-shrink: 0 on .bar rule"
  missing:
    - "Add flex-shrink: 0 to .bar in StatusBar.module.css"
  debug_session: ".planning/debug/statusbar-not-visible.md"
- truth: "The status bar is 24px tall and uses 11px Inter font"
  status: failed
  reason: "User reported: no status bar"
  severity: blocker
  test: 9
  root_cause: "Same as test 1 — StatusBar compressed to 0px"
  artifacts:
    - path: "editor-ui/src/components/StatusBar.module.css"
      issue: "Missing flex-shrink: 0 on .bar rule"
  missing:
    - "Add flex-shrink: 0 to .bar in StatusBar.module.css"
  debug_session: ".planning/debug/statusbar-not-visible.md"
