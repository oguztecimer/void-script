---
phase: 06-status-bar
plan: 01
subsystem: ui
tags: [react, css-modules, zustand, status-bar, breadcrumb]

# Dependency graph
requires:
  - phase: 05-tool-strips-and-panels
    provides: Zustand store with scriptList, toggleLeftPanel, tabs, activeTabId selectors
  - phase: 02-css-architecture
    provides: StatusSegment primitive used by DiagnosticsWidget
provides:
  - NavPath breadcrumb component showing VOID//SCRIPT > [folder] > file.vs
  - DiagnosticsWidget with inline SVG error/warning icons and OK state
  - Shared scriptTypes.ts constants (TYPE_LABELS, TYPE_ORDER)
  - Updated StatusBar wiring NavPath + DiagnosticsWidget
affects: [07-editor-gutter, 08-debug-ui, 09-breadcrumb]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Breadcrumb segments: button for clickable (project), span for inert (folder/file)"
    - "CSS text-only hover: color transition only, no background change (Rider breadcrumb style)"
    - "Component-owned Zustand selectors: NavPath reads its own store slices internally"
    - "Shared constants module (scriptTypes.ts) prevents duplication across components"

key-files:
  created:
    - editor-ui/src/state/scriptTypes.ts
    - editor-ui/src/components/NavPath.tsx
    - editor-ui/src/components/NavPath.module.css
    - editor-ui/src/components/DiagnosticsWidget.tsx
  modified:
    - editor-ui/src/components/StatusBar.tsx
    - editor-ui/src/components/ScriptList.tsx

key-decisions:
  - "NavPath project segment renders as <button> (clickable toggleLeftPanel); folder/file as <span> (no implied action)"
  - "Chevron › character lives inside preceding segment element for larger click target; wrapped in .chevron span to stay --text-secondary on parent hover"
  - "DiagnosticsWidget returns null when !hasActiveTab, separate StatusSegment per severity type for composability"
  - "TYPE_LABELS/TYPE_ORDER extracted to state/scriptTypes.ts — both NavPath and ScriptList need the same mapping"

patterns-established:
  - "Text-only hover: .segment:hover { color: var(--text-primary) } with no background-color change — Rider breadcrumb contract"
  - "Inert span variant (.segmentInert) prevents cursor:pointer and hover color shift for non-interactive path segments"

requirements-completed: [STAT-01, STAT-02]

# Metrics
duration: 1min
completed: 2026-03-15
---

# Phase 6 Plan 1: Status Bar Nav Path and Diagnostics Widget Summary

**Dynamic breadcrumb nav path (VOID//SCRIPT > Ship Brains > miner_brain.vs) and inline SVG icon diagnostics replacing plain-text labels and the VCS branch widget**

## Performance

- **Duration:** 1 min
- **Started:** 2026-03-15T10:31:21Z
- **Completed:** 2026-03-15T10:32:29Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments
- Created NavPath component deriving breadcrumb path from active tab and scriptList (flat scripts show 2 segments; typed scripts show 3 segments with folder label)
- Created DiagnosticsWidget with inline SVG ErrorIcon/WarningIcon and OK state — replaces plain-text "X errors" / "X warn" labels
- Extracted TYPE_LABELS/TYPE_ORDER to shared scriptTypes.ts — NavPath and ScriptList now import from single source
- Updated StatusBar to use NavPath in left region and DiagnosticsWidget in diagnostics area; VCS branch widget completely removed

## Task Commits

Each task was committed atomically:

1. **Task 1: Create NavPath, DiagnosticsWidget, and shared scriptTypes** - `3e82b4b` (feat)
2. **Task 2: Wire NavPath and DiagnosticsWidget into StatusBar** - `449f68c` (feat)

## Files Created/Modified
- `editor-ui/src/state/scriptTypes.ts` - Shared TYPE_LABELS and TYPE_ORDER constants
- `editor-ui/src/components/NavPath.tsx` - Breadcrumb path component using Zustand store
- `editor-ui/src/components/NavPath.module.css` - Text-only hover styles (no background change)
- `editor-ui/src/components/DiagnosticsWidget.tsx` - Icon+count diagnostic segments with inline SVGs
- `editor-ui/src/components/StatusBar.tsx` - Wired NavPath and DiagnosticsWidget, removed VCS widget
- `editor-ui/src/components/ScriptList.tsx` - Updated to import TYPE_LABELS/TYPE_ORDER from scriptTypes.ts

## Decisions Made
- NavPath project segment renders as `<button>` (calls toggleLeftPanel on click); folder and file segments render as `<span>` with no onClick to avoid implying non-existent functionality
- Chevron `›` is placed inside the preceding segment button as a `<span className={styles.chevron}>` — keeps the full button as the click target while the chevron stays `--text-secondary` even when the parent button is hovered
- DiagnosticsWidget accepts `hasActiveTab` as a prop rather than calling useStore internally — StatusBar already has the activeTab reference, avoids duplicate selector
- TYPE_LABELS/TYPE_ORDER extracted to `state/scriptTypes.ts` because both NavPath and ScriptList need the same script type mapping

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Status bar now matches Rider New UI layout: dynamic nav path on left, icon diagnostics on right
- NavPath clicking "VOID//SCRIPT" toggles the left panel (ScriptList)
- Ready for Phase 7 (editor gutter / breakpoints)

---
*Phase: 06-status-bar*
*Completed: 2026-03-15*
