---
phase: 09-polish-and-tooltips
plan: 01
subsystem: ui
tags: [react, tooltip, custom-primitives, keyboard-shortcuts, rider-dark]

# Dependency graph
requires:
  - phase: 02-css-architecture
    provides: CSS tokens (--font-ui, color tokens) used by Tooltip styling
  - phase: 05-tool-strips-and-panels
    provides: ToolBtn primitive that was extended with Tooltip wrapper

provides:
  - Tooltip.tsx: reusable tooltip primitive with 800ms delay, viewport flip, Rider-dark styling
  - ToolBtn with Tooltip wrapper and optional shortcut prop (no native title attribute)
  - All native title= DOM attributes removed from interactive elements across entire app
  - Keyboard shortcut hints in Run/Debug/Stop and debug step button tooltips

affects: [any future interactive elements — use Tooltip, not native title=]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Tooltip wrapper pattern: wrap interactive elements in <Tooltip content="..."> instead of native title= attribute
    - shortcut prop pattern: ToolBtn accepts shortcut prop; tooltip content auto-formatted as "Label (Shortcut)"

key-files:
  created:
    - editor-ui/src/primitives/Tooltip.tsx
    - editor-ui/src/primitives/Tooltip.module.css
  modified:
    - editor-ui/src/primitives/ToolBtn.tsx
    - editor-ui/src/components/Header.tsx
    - editor-ui/src/components/DebugPanel.tsx

key-decisions:
  - "Tooltip wrapper uses display:inline-flex (not display:contents) — contents breaks absolute positioning of tooltip div"
  - "No exit animation on Tooltip — fade-in only, instant hide on mouseleave matches Rider behavior"
  - "Tooltip disabled prop skips wrapper entirely and renders children directly — avoids unnecessary DOM nodes"
  - "ToolStrip.tsx was not modified — it already concatenates shortcut into title prop before passing to ToolBtn"

patterns-established:
  - "Tooltip primitive: always use <Tooltip content='...'> wrapper instead of native title= on interactive elements"
  - "ToolBtn shortcut prop: pass shortcut='Modifier+Key' to auto-format tooltip as 'Label (Modifier+Key)'"

requirements-completed: [PLSH-03, PLSH-04]

# Metrics
duration: 3min
completed: 2026-03-15
---

# Phase 9 Plan 01: Polish and Tooltips Summary

**Rider-dark custom Tooltip primitive with 800ms delay and viewport flip, replacing all native title= attributes; keyboard shortcut hints added to Run/Debug/Stop/Resume/Step buttons**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-15T17:55:54Z
- **Completed:** 2026-03-15T17:58:12Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments

- Created Tooltip.tsx primitive: 800ms delay, viewport flip when clipping bottom, fade-in animation, Rider-dark styling (#3C3F41 bg, #BBBBBB text, #555555 border, no arrow)
- Extended ToolBtn with Tooltip wrapper and optional shortcut prop — native title= attribute removed from button element
- Migrated all native title= DOM attributes in Header.tsx (TrafficLight x3, SearchPill) and DebugPanel.tsx (variable value span)
- Added keyboard shortcut hints to 7 run/debug buttons: Run (Shift+F10), Debug (Shift+F9), Stop (Ctrl+F2), Resume (F9), Step Over (F8), Step Into (F7), Step Out (Shift+F8)

## Task Commits

Each task was committed atomically:

1. **Task 1: Create Tooltip primitive and integrate into ToolBtn** - `4faf9e0` (feat)
2. **Task 2: Migrate all remaining title attributes to Tooltip** - `fadf47f` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified

- `editor-ui/src/primitives/Tooltip.tsx` - Tooltip wrapper component with 800ms delay, viewport flip, fade-in animation
- `editor-ui/src/primitives/Tooltip.module.css` - Rider-dark tooltip visual styling (#3C3F41 background)
- `editor-ui/src/primitives/ToolBtn.tsx` - Wraps button in Tooltip, adds shortcut prop, removes native title attribute
- `editor-ui/src/components/Header.tsx` - TrafficLight and SearchPill migrated; shortcut hints on 7 run/debug buttons
- `editor-ui/src/components/DebugPanel.tsx` - Variable value span migrated from native title to Tooltip wrapper

## Decisions Made

- Tooltip wrapper uses `display: inline-flex` (not `display: contents`) — contents breaks absolute positioning of the tooltip div
- No exit animation — fade-in only, instant hide on mouseleave matches JetBrains Rider behavior
- `disabled` prop on Tooltip skips wrapper entirely, rendering children directly (avoids unnecessary DOM nodes on disabled buttons)
- ToolStrip.tsx was intentionally not modified — it already concatenates shortcut into the title string before passing to ToolBtn, which now wraps in Tooltip correctly

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- PLSH-03 and PLSH-04 requirements satisfied
- Tooltip primitive is ready for use by any future interactive elements
- Convention established: never use native title= on interactive elements; always use Tooltip wrapper

---
*Phase: 09-polish-and-tooltips*
*Completed: 2026-03-15*
