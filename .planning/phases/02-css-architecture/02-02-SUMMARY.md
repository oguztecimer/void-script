---
phase: 02-css-architecture
plan: 02
subsystem: ui
tags: [react, css-modules, hover, primitives, migration]

# Dependency graph
requires:
  - phase: 02-css-architecture
    provides: "ToolBtn, PanelHeader, Separator, StatusSegment primitives with CSS Modules; css-modules.d.ts type declaration"
  - phase: 01-foundation
    provides: "tokens.css with 59 CSS custom properties; inline :root in index.html for wry compatibility"

provides:
  - "All non-titlebar components migrated from inline styles to CSS Modules"
  - "Zero onMouseEnter/onMouseLeave hover handlers outside Header.tsx"
  - "CSS :hover pseudo-classes with var(--transition-hover) on all interactive elements"
  - "3-level border hierarchy (strong/default/subtle) correctly applied across all panels"
  - "Primitives consumed: ToolBtn (5 components), PanelHeader (2 components), StatusSegment (1 component)"

affects:
  - Phase 3-9 (all future UI work builds on CSS Module patterns)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "CSS Module per component: ComponentName.module.css co-located with ComponentName.tsx"
    - "CSS :hover replaces all onMouseEnter/onMouseLeave JS patterns"
    - "Class composition via template literals: `${styles.tab} ${isActive ? styles.active : ''}`"
    - "CSS composes for variant inheritance: .frameActive { composes: frame; }"
    - "Level-based CSS classes for entry types: levelClass[entry.level] maps to styles.error/warn/info"

key-files:
  created:
    - "editor-ui/src/components/ToolStrip.module.css"
    - "editor-ui/src/components/ScriptList.module.css"
    - "editor-ui/src/components/DebugPanel.module.css"
    - "editor-ui/src/components/StatusBar.module.css"
    - "editor-ui/src/components/TabBar.module.css"
    - "editor-ui/src/components/Console.module.css"
    - "editor-ui/src/App.module.css"
  modified:
    - "editor-ui/src/components/ToolStrip.tsx"
    - "editor-ui/src/components/ScriptList.tsx"
    - "editor-ui/src/components/DebugPanel.tsx"
    - "editor-ui/src/components/StatusBar.tsx"
    - "editor-ui/src/components/TabBar.tsx"
    - "editor-ui/src/components/Console.tsx"
    - "editor-ui/src/App.tsx"

key-decisions:
  - "Bottom panel header uses CSS Module div instead of PanelHeader primitive: PanelHeader takes string title but bottom panel needs BottomTab component on left side"
  - "DebugPanel frame variants use CSS composes for DRY base-class inheritance"
  - "Console entry levels mapped via runtime object (levelClass record) rather than ternary chains"

patterns-established:
  - "Component migration pattern: create .module.css, replace style props with className, remove onMouseEnter/onMouseLeave"
  - "Primitive consumption pattern: import from ../primitives/*, replace inline helper components"
  - "Bottom panel header pattern: CSS Module classes mirroring PanelHeader visual design for layouts requiring ReactNode in title position"

requirements-completed: [FOUN-04, PLSH-01, PLSH-02]

# Metrics
duration: ~4min
completed: 2026-03-14
---

# Phase 2 Plan 2: Component CSS Module Migration Summary

**All 7 non-titlebar components migrated from inline React styles to CSS Modules with pure CSS :hover, 3-level border hierarchy, and primitive consumption eliminating 16 onMouseEnter/onMouseLeave handlers**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-03-14T10:10:08Z
- **Completed:** 2026-03-14T10:14:40Z
- **Tasks:** 2 of 2 complete
- **Files created:** 7 CSS Modules
- **Files modified:** 7 TSX components

## Accomplishments

- Migrated ToolStrip, ScriptList, DebugPanel, StatusBar, TabBar, Console, and App from inline React styles to CSS Modules
- Eliminated all 16 onMouseEnter/onMouseLeave handlers outside Header.tsx, replaced with CSS :hover pseudo-classes using var(--transition-hover)
- Consumed primitives across 5 components: ToolBtn (ToolStrip, ScriptList, DebugPanel, App), PanelHeader (ScriptList, DebugPanel), StatusSegment (StatusBar)
- Applied 3-level border hierarchy correctly: --border-strong for panel outer edges (ToolStrip, ScriptList, DebugPanel, StatusBar, bottom panel), --border-default for separators (tab bar, panel headers, section dividers)
- Removed inline helper components: ToolWindowBtn (ScriptList, DebugPanel), StatusSegment (StatusBar), PanelHeaderBtn (App) -- replaced with primitives

## Task Commits

Each task was committed atomically:

1. **Task 1: Migrate ToolStrip, ScriptList, DebugPanel, StatusBar to CSS Modules** - `191fb86` (feat)
2. **Task 2: Migrate TabBar, Console, App to CSS Modules; final hover elimination** - `3182458` (feat)

## Files Created/Modified

**CSS Modules created:**
- `editor-ui/src/components/ToolStrip.module.css` - Strip layout with left/right border variants
- `editor-ui/src/components/ScriptList.module.css` - Panel layout, group labels, script items with :hover
- `editor-ui/src/components/DebugPanel.module.css` - Panel layout, frame/variable rows with composes
- `editor-ui/src/components/StatusBar.module.css` - Bar layout with spacer
- `editor-ui/src/components/TabBar.module.css` - Tab bar with tab :hover, active state, close button :hover
- `editor-ui/src/components/Console.module.css` - Console output with error/warn/info level classes
- `editor-ui/src/App.module.css` - App layout shell, bottom panel, bottom tab styles

**Components modified:**
- `editor-ui/src/components/ToolStrip.tsx` - Uses ToolBtn primitive, CSS Module classes
- `editor-ui/src/components/ScriptList.tsx` - Uses PanelHeader + ToolBtn primitives, CSS Module classes
- `editor-ui/src/components/DebugPanel.tsx` - Uses PanelHeader + ToolBtn primitives, CSS Module classes
- `editor-ui/src/components/StatusBar.tsx` - Uses StatusSegment primitive, CSS Module classes
- `editor-ui/src/components/TabBar.tsx` - CSS Module classes with :hover, button element for close
- `editor-ui/src/components/Console.tsx` - CSS Module classes with level-based styling
- `editor-ui/src/App.tsx` - CSS Module layout classes, ToolBtn primitive for bottom panel actions

## Decisions Made

- **Bottom panel header layout:** Used CSS Module styled div instead of PanelHeader primitive because PanelHeader's API (string title) doesn't support the bottom panel's layout requirement of BottomTab component on the left side. The CSS Module classes mirror PanelHeader's visual design.
- **DebugPanel frame variants with CSS composes:** Used CSS `composes: frame` for frameActive/frameInactive classes to share base styling (padding, font-size, font-family) without duplication.
- **Console level mapping:** Used a runtime `Record<string, string>` object mapping entry levels to CSS Module class names, avoiding nested ternary expressions.
- **TabBar close button as `<button>` element:** Changed from `<span>` to `<button>` for semantic HTML correctness (interactive element should be a button).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Bottom panel header layout fix**
- **Found during:** Task 2 (App.tsx migration)
- **Issue:** PanelHeader primitive takes `title: string` but bottom panel header needs BottomTab React component on left side. Using PanelHeader with empty title put all content inside the actions div, breaking the left-right layout.
- **Fix:** Used CSS Module styled div (`.bottomPanelHeader`) that mirrors PanelHeader's visual design while supporting ReactNode content on both sides.
- **Files modified:** `editor-ui/src/App.tsx`, `editor-ui/src/App.module.css`
- **Verification:** Build passes, layout matches original design with tabs left, actions right
- **Committed in:** `3182458` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Minor API mismatch between PanelHeader primitive and bottom panel use case. CSS Module approach maintains visual consistency. No scope creep.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 2 (CSS Architecture) is fully complete
- All non-titlebar components use CSS Modules with real CSS :hover
- 3-level border hierarchy consistently applied
- All primitives (ToolBtn, PanelHeader, StatusSegment) actively consumed
- Header.tsx remains the only component with onMouseEnter/onMouseLeave (title bar exception per user decision)
- Ready for Phase 3+ UI development using established CSS Module patterns

## Self-Check: PASSED

All 7 CSS Module files confirmed present. All 2 task commits verified. SUMMARY.md created.

---
*Phase: 02-css-architecture*
*Completed: 2026-03-14*
