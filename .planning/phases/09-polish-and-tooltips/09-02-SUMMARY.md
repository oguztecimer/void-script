---
phase: 09-polish-and-tooltips
plan: 02
subsystem: ui
tags: [react, zustand, codemirror, css-modules, breadcrumb, voidscript]

# Dependency graph
requires:
  - phase: 07-resizable-panels
    provides: flex column layout for center panel (.center div with flex-direction column)
  - phase: 06-status-bar
    provides: flex-shrink: 0 pattern for fixed-height siblings in column flex layouts
  - phase: 04-tab-bar-and-editor-state
    provides: Zustand store with cursorLine, activeTabId, tabs; setCursor action

provides:
  - BreadcrumbBar component showing cursor-aware filename > function_name breadcrumb
  - findEnclosingFunction heuristic scanning backward through VoidScript code for def blocks
  - 24px Rider-styled breadcrumb bar slotted between TabBar and vertical Group in App layout

affects: [09-03-tooltips, future-navigation-features]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "BreadcrumbBar uses primitive Zustand selectors returning strings (not objects) to minimize re-renders"
    - "useMemo wraps expensive per-line scan to avoid recomputing on unrelated store changes"
    - "flex-shrink: 0 on fixed-height bar siblings of flex:1 children (established in Phase 6)"

key-files:
  created:
    - editor-ui/src/components/BreadcrumbBar.tsx
    - editor-ui/src/components/BreadcrumbBar.module.css
  modified:
    - editor-ui/src/App.tsx

key-decisions:
  - "BreadcrumbBar selectors return primitives (string, not Tab object) so Zustand only triggers re-render when string value changes"
  - "useMemo on findEnclosingFunction prevents O(n) backward scan on every unrelated store update"
  - "Using &rsaquo; HTML entity (›) as separator, consistent with NavPath chevron in status bar"
  - "Display-only breadcrumb — no click handlers, no navigation (click-to-navigate deferred per CONTEXT.md)"

patterns-established:
  - "Primitive selector pattern: useStore((s) => s.tabs.find(...).name ?? null) returns string|null for fine-grained reactivity"
  - "Heuristic def-block detection: scan backward from cursorLine - 1 with DEF_RE regex"

requirements-completed: [EDIT-01]

# Metrics
duration: 1min
completed: 2026-03-15
---

# Phase 9 Plan 02: Breadcrumb Bar Summary

**Cursor-aware breadcrumb bar below the tab strip using backward-scan heuristic to show `filename.vs > function_name` when inside a VoidScript `def` block**

## Performance

- **Duration:** 1 min
- **Started:** 2026-03-15T17:55:59Z
- **Completed:** 2026-03-15T17:56:59Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- BreadcrumbBar.tsx component with `findEnclosingFunction` backward-scan heuristic for VoidScript `def` blocks
- Zustand integration using primitive selectors for minimal re-renders, `useMemo` wrapping the scan
- BreadcrumbBar.module.css with 24px height, `flex-shrink: 0`, Rider dark panel styling
- App.tsx slotted `<BreadcrumbBar />` between `<TabBar />` and vertical `<Group>` in the center panel

## Task Commits

Each task was committed atomically:

1. **Task 1: Create BreadcrumbBar component with def-block heuristic** - `e30858a` (feat)
2. **Task 2: Insert BreadcrumbBar into App layout** - `2540564` (feat)

## Files Created/Modified
- `editor-ui/src/components/BreadcrumbBar.tsx` - Display-only breadcrumb component; reads cursorLine, activeTabContent, activeTabName from Zustand; derives enclosing function name via backward scan
- `editor-ui/src/components/BreadcrumbBar.module.css` - 24px fixed-height bar, flex-shrink: 0, Rider bg-panel background, border-bottom separator, segment/segmentActive/chevron rules
- `editor-ui/src/App.tsx` - Added BreadcrumbBar import; inserted `<BreadcrumbBar />` between `<TabBar />` and vertical `<Group>`

## Decisions Made
- Zustand selectors return primitive strings rather than Tab objects so Zustand's shallow equality only triggers re-renders when the actual string values change
- `useMemo` wraps `findEnclosingFunction` to avoid O(n) backward line scan on every store update not related to cursor or content
- Used `&rsaquo;` (›) HTML entity for the chevron separator, matching the NavPath component in the status bar
- Breadcrumb is display-only per CONTEXT.md decision; click-to-navigate deferred to a future enhancement

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Breadcrumb bar complete; Phase 9 Plan 01 (Tooltip primitive) and Plan 03 (tooltip integration) can proceed
- The `findEnclosingFunction` heuristic could be extended for nested scopes in a future polish pass

---
*Phase: 09-polish-and-tooltips*
*Completed: 2026-03-15*
