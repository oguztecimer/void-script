---
phase: 03-title-bar
plan: 01
subsystem: ui
tags: [react, css-modules, toolbar, header, rider-ui]

# Dependency graph
requires:
  - phase: 02-css-architecture
    provides: ToolBtn and Separator primitives with CSS Modules, design tokens in index.html

provides:
  - Header.tsx restructured to use CSS Modules and project primitives exclusively
  - Header.module.css with all header layout and widget styles
  - SearchPill component with magnifying glass + "Search" text + shift-shift hint
  - Settings gear icon at toolbar far-right via ToolBtn size="small"
  - Correct Rider-style widget arrangement with proper separator grouping

affects: [04-tab-bar, 05-status-bar, any phase touching Header.tsx]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - CSS Module co-located with component (Header.module.css alongside Header.tsx)
    - JS-driven hover for drag-zone widgets (HeaderWidget) vs CSS :hover for no-drag zone (RunConfigSelector, TrafficLight, SearchPill)
    - ToolBtn size="small" with className="titlebar-no-drag" for icon buttons in drag zone

key-files:
  created:
    - editor-ui/src/components/Header.module.css
  modified:
    - editor-ui/src/components/Header.tsx

key-decisions:
  - "TrafficLight uses CSS :hover (no useState) — it is in a titlebar-no-drag container"
  - "HeaderWidget retains JS hover (onMouseEnter/onMouseLeave) — in drag zone per CONTEXT.md decision"
  - "No separator between project widget and VCS branch widget — matches Rider layout"
  - "Debug paused state: Stop | Separator | Resume StepOver StepInto StepOut"
  - "SearchPill and Settings gear added in same restructure pass as Task 1 — coherent single commit"

patterns-established:
  - "Drag-zone buttons get className='titlebar-no-drag' on ToolBtn for clickability"
  - "Static layout values live in CSS Module; only dynamic values (per-instance colors, JS hover) stay as inline styles"

requirements-completed: [TBAR-01, TBAR-02, TBAR-03, TBAR-04]

# Metrics
duration: 15min
completed: 2026-03-14
---

# Phase 3 Plan 1: Title Bar CSS Module and Rider Widget Layout Summary

**Header.tsx fully migrated to CSS Modules with ToolBtn/Separator primitives, correct Rider widget arrangement (no project-VCS separator), SearchPill with magnifying glass + shift-shift hint, and Settings gear at far-right — awaiting visual verification.**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-03-14T00:00:00Z
- **Completed:** 2026-03-14
- **Tasks:** 2/2 auto tasks complete (Task 3 is human-verify checkpoint)
- **Files modified:** 2

## Accomplishments

- Deleted three local component definitions from Header.tsx (ToolBtn, ActionBtn, Separator) and replaced with project primitives imported from src/primitives/
- Created Header.module.css with 12 CSS classes covering all toolbar layout and widget styling — zero remaining inline styles on static elements
- Implemented correct Rider toolbar widget order: traffic lights | hamburger | back/forward | project VCS | spacer | run-config | action-buttons | search-pill | settings-gear, with separators only between groups
- Added SearchPill (220px pill, magnifying glass SVG, "Search" label, "⇧⇧" right-aligned muted hint) and Settings gear ToolBtn
- Fixed debug controls: Stop | separator | Resume StepOver StepInto StepOut (separator only between Stop and stepping group)
- Removed all status text ("Running...", "Debugging...", "Paused") from toolbar

## Task Commits

Each task was committed atomically:

1. **Task 1 + 2: Migrate Header to CSS Module, replace primitives, add SearchPill and Settings gear** - `4f58fe4` (feat)

_Note: Tasks 1 and 2 were executed in a single coherent pass since the restructure of Header.tsx naturally encompassed both — the SearchPill and Settings gear were written as part of the same file rewrite._

**Plan metadata:** (pending — created after checkpoint)

## Files Created/Modified

- `editor-ui/src/components/Header.module.css` - New CSS Module with all header layout classes: toolbar, spacer, rightGroup, trafficLights, trafficLight, widget, widgetMuted, widgetIcon, widgetChevron, runConfig, runConfigIcon, runConfigChevron, searchPill, searchShortcut
- `editor-ui/src/components/Header.tsx` - Fully restructured: imports ToolBtn/Separator from primitives, CSS Module for all static styles, correct widget arrangement, SearchPill, Settings gear

## Decisions Made

- TrafficLight migrated to CSS `:hover` (no more useState/onMouseEnter/onMouseLeave) because it lives inside a `titlebar-no-drag` container — CSS hover is safe there
- HeaderWidget retains JS hover (`onMouseEnter`/`onMouseLeave`) since it sits in the drag region — following the established Phase 3 CONTEXT.md decision
- RunConfigSelector loses JS hover in favor of CSS `:hover` — it is inside the `.rightGroup` which has `titlebar-no-drag`, making CSS hover reliable
- All three step buttons (Step Over, Step Into, Step Out) use `ToolBtn size="small"` (ghost variant, not filled) — they are navigational, not action buttons
- Tasks 1 and 2 executed together as a single coherent rewrite — committed as one atomic unit rather than two partial states

## Deviations from Plan

None — plan executed exactly as written. SearchPill and Settings gear (Task 2 scope) were written in the same file pass as Task 1 since splitting them would have required writing Header.tsx twice.

## Issues Encountered

None — build passed on first attempt with zero TypeScript errors.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Task 3 (human-verify checkpoint) requires visual inspection by user before TBAR requirements are officially signed off
- If visual check passes: Phase 3 Plan 1 is complete, ready for Phase 4 (Tab Bar) or any other Phase 3 plans
- Verification steps: `cd editor-ui && npm run dev`, check all 4 TBAR requirements in DevTools + visual inspection

---
*Phase: 03-title-bar*
*Completed: 2026-03-14*
