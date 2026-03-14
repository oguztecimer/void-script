---
phase: 02-css-architecture
plan: 01
subsystem: ui
tags: [react, css-modules, primitives, design-system, hover]

# Dependency graph
requires:
  - phase: 01-foundation
    provides: tokens.css with 59 CSS custom properties; CSS Modules type declaration infrastructure

provides:
  - "ToolBtn primitive: icon button with ghost/filled variants, default/small sizes, CSS :hover"
  - "PanelHeader primitive: title + right-aligned action buttons row with border-bottom"
  - "Separator primitive: line/gap variants with vertical/horizontal orientation and 3-level border hierarchy"
  - "StatusSegment primitive: icon+text pair with CSS :hover transition at 150ms ease"
  - "CSS Modules type declaration (css-modules.d.ts) for TypeScript CSS Module imports"

affects:
  - 02-css-architecture-02 (component migration will consume these primitives)
  - Phase 3-9 (all future UI work uses these building blocks)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "CSS Modules co-located with components: Component.tsx + Component.module.css"
    - "CSS custom property approach for per-instance hover colors: --_btn-hover-bg set via inline style, consumed in CSS :hover rule"
    - "No onMouseEnter/onMouseLeave for hover states — pure CSS :hover pseudo-class"
    - "Template literal class composition: [styles.btn, styles[size], ...].filter(Boolean).join(' ')"

key-files:
  created:
    - "editor-ui/src/primitives/ToolBtn.tsx"
    - "editor-ui/src/primitives/ToolBtn.module.css"
    - "editor-ui/src/primitives/PanelHeader.tsx"
    - "editor-ui/src/primitives/PanelHeader.module.css"
    - "editor-ui/src/primitives/Separator.tsx"
    - "editor-ui/src/primitives/Separator.module.css"
    - "editor-ui/src/primitives/StatusSegment.tsx"
    - "editor-ui/src/primitives/StatusSegment.module.css"
    - "editor-ui/src/css-modules.d.ts"
  modified: []

key-decisions:
  - "CSS custom property --_btn-hover-bg for filled variant hover: avoids JS hover handlers while supporting per-instance hover colors"
  - "StatusSegment renders <button> when onClick provided, <div> otherwise: semantic HTML for accessibility"
  - "Separator gap variant uses margin-based spacing rather than width/height to work naturally in flex layouts"
  - "CSS Modules type declaration added to src/css-modules.d.ts: required for TypeScript to resolve *.module.css imports"

patterns-established:
  - "Primitive component pattern: named export function, co-located CSS Module, var(--token) references"
  - "Interactive CSS hover: .class:hover:not(:disabled) with var(--transition-hover) timing"
  - "Per-instance CSS custom property injection: component sets --_private-var inline, CSS rule consumes it"

requirements-completed: [FOUN-04, PLSH-01, PLSH-02]

# Metrics
duration: ~2min
completed: 2026-03-14
---

# Phase 2 Plan 1: UI Primitives Summary

**Four shared UI primitives (ToolBtn, PanelHeader, Separator, StatusSegment) with CSS Modules and pure CSS :hover replacing all onMouseEnter/onMouseLeave patterns**

## Performance

- **Duration:** ~2 min
- **Started:** 2026-03-14T10:04:57Z
- **Completed:** 2026-03-14T10:06:49Z
- **Tasks:** 2 of 2 complete
- **Files created:** 9

## Accomplishments

- Created ToolBtn primitive with ghost/filled variants, default (36px) / small (26px) sizes, active state, and CSS :hover via custom property approach for per-instance hover colors
- Created PanelHeader primitive with title + right-aligned action buttons layout using CSS Modules
- Created Separator primitive with line/gap variants, vertical/horizontal orientation, and 3-level border color hierarchy (default, subtle, strong)
- Created StatusSegment primitive with icon+text pair, semantic button/div rendering, and CSS :hover transition at 150ms ease
- Added CSS Modules type declaration for TypeScript compatibility across the entire project

## Task Commits

Each task was committed atomically:

1. **Task 1: Create ToolBtn and Separator primitives with CSS Modules** - `5ccb7b0` (feat)
2. **Task 2: Create PanelHeader and StatusSegment primitives with CSS Modules** - `6873280` (feat)

## Files Created/Modified

- `editor-ui/src/primitives/ToolBtn.tsx` - Icon button with ghost/filled variants, size prop, active state
- `editor-ui/src/primitives/ToolBtn.module.css` - CSS :hover, :disabled, .active, .filled states with var(--transition-hover)
- `editor-ui/src/primitives/PanelHeader.tsx` - Panel header row with title and action buttons
- `editor-ui/src/primitives/PanelHeader.module.css` - Flex layout with border-bottom using var(--border-default)
- `editor-ui/src/primitives/Separator.tsx` - Line/gap separator with orientation and border hierarchy level
- `editor-ui/src/primitives/Separator.module.css` - Line and gap variants with var(--border-default/subtle/strong)
- `editor-ui/src/primitives/StatusSegment.tsx` - Status bar segment with icon+text and semantic button/div
- `editor-ui/src/primitives/StatusSegment.module.css` - CSS :hover with var(--transition-hover) and var(--bg-hover)
- `editor-ui/src/css-modules.d.ts` - TypeScript module declaration for *.module.css imports

## Decisions Made

- **CSS custom property for filled hover:** ToolBtn sets `--_btn-hover-bg` as an inline style variable; the CSS `.filled:hover` rule consumes it. This avoids onMouseEnter/onMouseLeave while supporting per-instance hover colors (e.g., run button green, debug button blue).
- **Semantic HTML for StatusSegment:** Renders `<button>` when onClick is provided, `<div>` otherwise, for proper accessibility without additional ARIA attributes.
- **Separator gap uses margin:** Gap variant applies margin-left/right (vertical) or margin-top/bottom (horizontal) at half the gap value, creating natural spacing in flex containers.
- **CSS Modules type declaration:** Added `css-modules.d.ts` to resolve TypeScript TS2307 errors on CSS Module imports. This is a standard Vite project requirement that was missing.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added CSS Modules type declaration for TypeScript**
- **Found during:** Task 1 (TypeScript verification)
- **Issue:** `npx tsc --noEmit` failed with TS2307 "Cannot find module '*.module.css'" because no type declaration existed for CSS Module imports
- **Fix:** Created `editor-ui/src/css-modules.d.ts` with a module declaration mapping `*.module.css` to `{ readonly [key: string]: string }`
- **Files modified:** `editor-ui/src/css-modules.d.ts`
- **Verification:** `npx tsc --noEmit` passes with zero errors
- **Committed in:** `5ccb7b0` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Essential infrastructure for CSS Modules to work with TypeScript. No scope creep.

## Issues Encountered

None beyond the CSS Modules type declaration (documented as deviation above).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- All four primitives are ready for Plan 02 (component migration) to consume
- ToolBtn replaces Header's ToolBtn, ActionBtn, and ScriptList/DebugPanel's ToolWindowBtn
- PanelHeader replaces inline panel header patterns in ScriptList, DebugPanel, Console
- Separator replaces Header's Separator and any toolbar spacing
- StatusSegment replaces StatusBar's inline segment pattern
- CSS Modules type declaration enables all future CSS Module files to compile

---
*Phase: 02-css-architecture*
*Completed: 2026-03-14*
