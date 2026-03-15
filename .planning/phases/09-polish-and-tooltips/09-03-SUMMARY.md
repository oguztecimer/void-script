---
phase: 09-polish-and-tooltips
plan: "03"
subsystem: ui
tags: [codemirror, zustand, breadcrumb, cursor, editor]

# Dependency graph
requires:
  - phase: 09-polish-and-tooltips
    provides: BreadcrumbBar component with findEnclosingFunction; setCursor action in store

provides:
  - cursorLine/cursorCol reset in switchTab and openTab actions (prevents cross-tab cursor bleed)
  - Initial setCursor call after EditorView construction (breadcrumb correct on tab open without cursor move)

affects: [breadcrumb, BreadcrumbBar, Editor, store]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Reset transient UI state (cursor position) eagerly on tab switch; let Editor.tsx overwrite with real value moments later"
    - "Read CodeMirror selection.main.head immediately after new EditorView() to sync store before first render"

key-files:
  created: []
  modified:
    - editor-ui/src/state/store.ts
    - editor-ui/src/components/Editor.tsx

key-decisions:
  - "Reset to line 1, col 1 in switchTab/openTab — Editor.tsx overwrites with real position before any render sees the stale value, avoiding cross-tab bleed"
  - "Block scope { } around head/line locals in EditorView construction block — keeps useEffect scope clean"

patterns-established:
  - "Eager stale-state reset + async real-value sync pattern: reset to sentinel first, overwrite with truth immediately after"

requirements-completed: [PLSH-03, PLSH-04]

# Metrics
duration: 1min
completed: 2026-03-15
---

# Phase 9 Plan 03: Breadcrumb Cursor Sync Summary

**Breadcrumb bug fixed: cursorLine reset on tab switch/open and synced from real CodeMirror selection on EditorView creation**

## Performance

- **Duration:** 1 min
- **Started:** 2026-03-15T18:54:04Z
- **Completed:** 2026-03-15T18:55:15Z
- **Tasks:** 3
- **Files modified:** 2

## Accomplishments
- switchTab and openTab both reset cursorLine/cursorCol to 1,1 eliminating cross-tab cursor bleed
- Editor.tsx reads editorState.selection.main.head immediately after new EditorView() and calls setCursor so BreadcrumbBar sees the correct line on first render
- Production build confirmed clean (exit 0, no TypeScript errors)

## Task Commits

Each task was committed atomically:

1. **Task 1: Reset cursor in switchTab and openTab** - `233f381` (fix)
2. **Task 2: Sync initial cursor after EditorView creation** - `22f4d14` (fix)
3. **Task 3: Build and verify** - no commit (build verification only, dist is gitignored)

## Files Created/Modified
- `editor-ui/src/state/store.ts` - Added `cursorLine: 1, cursorCol: 1` to switchTab and both branches of openTab
- `editor-ui/src/components/Editor.tsx` - Added setCursor call using editorState.selection.main.head after EditorView construction

## Decisions Made
- Reset to sentinel (1,1) eagerly in store actions; Editor.tsx overwrites with real CodeMirror value before React renders — guarantees no stale cross-tab cursor visible
- Block scope `{ }` for head/line locals in EditorView construction block keeps useEffect scope uncluttered

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None. Both TypeScript checks and production build exited 0 on first attempt.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- UAT tests 6 and 7 (breadcrumb function name on tab open/switch) should now pass
- BreadcrumbBar.tsx and findEnclosingFunction were already correct; the data they receive is now correct too
- No blockers for remaining phase 9 work

---
*Phase: 09-polish-and-tooltips*
*Completed: 2026-03-15*
