---
phase: 09-polish-and-tooltips
plan: "04"
subsystem: ui
tags: [react, tooltip, viewport-clamping, css-transform]

# Dependency graph
requires:
  - phase: 09-polish-and-tooltips
    provides: Tooltip primitive with vertical flip behavior

provides:
  - Tooltip horizontal edge clamping — prevents clips at left/right viewport edges via offsetX corrective delta

affects: [09-polish-and-tooltips]

# Tech tracking
tech-stack:
  added: []
  patterns: [inline-style-override for CSS transform correction delta]

key-files:
  created: []
  modified:
    - editor-ui/src/primitives/Tooltip.tsx

key-decisions:
  - "offsetX correction applied as inline style override (transform: translateX(calc(-50% + Xpx))) only when non-zero; CSS default untouched for centered tooltips"
  - "MARGIN=4px gap maintained from viewport edge matches existing vertical-flip pattern"

patterns-established:
  - "Viewport edge clamping pattern: getBoundingClientRect rect.left/rect.right vs MARGIN, compute dx, apply as inline style delta atop existing CSS centering"

requirements-completed: [PLSH-03]

# Metrics
duration: 2min
completed: 2026-03-15
---

# Phase 9 Plan 04: Tooltip Horizontal Clamping Summary

**Tooltip horizontal edge clamping via offsetX state and getBoundingClientRect delta, preventing left/right viewport clips reported in UAT test 4**

## Performance

- **Duration:** 2 min
- **Started:** 2026-03-15T18:14:10Z
- **Completed:** 2026-03-15T18:14:50Z
- **Tasks:** 2 (1 code change + 1 build verification)
- **Files modified:** 1

## Accomplishments

- Added `offsetX` state to Tooltip.tsx (default 0) alongside existing `flipped` state
- Extended useEffect to compute corrective dx when tooltip overhangs left or right viewport edge by 4px margin
- Applied correction as `transform: translateX(calc(-50% + ${offsetX}px))` inline style — only set when offsetX !== 0, leaving CSS-centered tooltips completely unaffected
- Vertical flip behavior (bottom edge) preserved exactly; offsetX resets to 0 on hide alongside flipped
- TypeScript compiles clean; production build exits 0

## Task Commits

Each task was committed atomically:

1. **Task 1: Add horizontal clamping to Tooltip.tsx** - `22a95b8` (feat)

*Task 2 (build verification) produced no source changes — no additional commit.*

**Plan metadata:** committed with docs commit below

## Files Created/Modified

- `editor-ui/src/primitives/Tooltip.tsx` — Added offsetX state, horizontal clamping in useEffect, conditional inline style on tooltip div

## Decisions Made

- Inline style override for correction delta: CSS `transform: translateX(-50%)` is the base; when offsetX != 0 the inline style replaces it with `calc(-50% + Xpx)`. When offsetX === 0, `style={undefined}` so CSS applies unmodified — avoids any unnecessary style override.
- Only `Tooltip.tsx` modified — `Tooltip.module.css` required no changes as correction is entirely runtime-computed.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- UAT test 4 "clips off the sides" concern resolved — tooltip now clamps horizontally with 4px margin from viewport edges
- Tooltip primitive is feature-complete: vertical flip, horizontal clamping, fade-in, 800ms delay, disabled prop

---
*Phase: 09-polish-and-tooltips*
*Completed: 2026-03-15*

## Self-Check: PASSED

- Tooltip.tsx: FOUND
- 09-04-SUMMARY.md: FOUND
- Commit 22a95b8: FOUND
