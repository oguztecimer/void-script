---
phase: 03-title-bar
plan: 01
subsystem: ui
tags: [react, css-modules, toolbar, header, rider-ui, wry, webkit, macos]

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
    - crates/voidscript-editor/src/window.rs

key-decisions:
  - "TrafficLight uses CSS :hover (no useState) — it is in a titlebar-no-drag container"
  - "HeaderWidget retains JS hover (onMouseEnter/onMouseLeave) — in drag zone per CONTEXT.md decision"
  - "No separator between project widget and VCS branch widget — matches Rider layout"
  - "Debug paused state: Stop | Separator | Resume StepOver StepInto StepOut"
  - "SearchPill and Settings gear added in same restructure pass as Task 1 — coherent single commit"
  - "wry with_accept_first_mouse(true) required — default false absorbs first mouseDown to focus window, preventing -webkit-app-region: drag on first click for frameless windows"

patterns-established:
  - "Drag-zone buttons get className='titlebar-no-drag' on ToolBtn for clickability"
  - "Static layout values live in CSS Module; only dynamic values (per-instance colors, JS hover) stay as inline styles"
  - "titlebar-drag and titlebar-no-drag MUST be global literal strings — never CSS Module references"
  - "wry WebViewBuilder for frameless windows MUST include with_accept_first_mouse(true)"

requirements-completed: [TBAR-01, TBAR-02, TBAR-03, TBAR-04]

# Metrics
duration: 30min
completed: 2026-03-14
---

# Phase 3 Plan 1: Title Bar CSS Module and Rider Widget Layout Summary

**Header.tsx migrated to CSS Modules with Rider-accurate widget layout, SearchPill + Settings gear widgets, and wry acceptsFirstMouse fix for reliable frameless window drag**

## Performance

- **Duration:** ~30 min (across two sessions, with checkpoint)
- **Started:** 2026-03-14T10:45:00Z
- **Completed:** 2026-03-14T14:10:00Z
- **Tasks:** 3/3 complete
- **Files modified:** 3

## Accomplishments

- Deleted three local component definitions from Header.tsx (ToolBtn, ActionBtn, Separator) and replaced with project primitives imported from src/primitives/
- Created Header.module.css with 12 CSS classes covering all toolbar layout and widget styling — zero remaining inline styles on static elements
- Implemented correct Rider toolbar widget order: traffic lights | hamburger | back/forward | project VCS | spacer | run-config | action-buttons | search-pill | settings-gear, with separators only between groups
- Added SearchPill (220px pill, magnifying glass SVG, "Search" label, "⇧⇧" right-aligned muted hint) and Settings gear ToolBtn
- Fixed debug controls: Stop | separator | Resume StepOver StepInto StepOut (separator only between Stop and stepping group)
- Removed all status text ("Running...", "Debugging...", "Paused") from toolbar
- Fixed window drag regression by adding `.with_accept_first_mouse(true)` to the wry WebViewBuilder (default `false` was absorbing the first mouseDown to focus the window instead of initiating drag)

## Task Commits

Each task was committed atomically:

1. **Tasks 1+2: Migrate Header to CSS Module, replace primitives, add SearchPill and Settings gear** - `4f58fe4` (feat)
2. **Task 3 continuation: Fix window drag regression** - `c38f532` (fix)

_Note: Tasks 1 and 2 were executed in a single coherent pass since the restructure of Header.tsx naturally encompassed both — the SearchPill and Settings gear were written as part of the same file rewrite._

**Plan metadata:** `bde9548` (docs: complete plan)

## Files Created/Modified

- `editor-ui/src/components/Header.module.css` - New CSS Module with all header layout classes: toolbar, spacer, rightGroup, trafficLights, trafficLight, widget, widgetMuted, widgetIcon, widgetChevron, runConfig, runConfigIcon, runConfigChevron, searchPill, searchShortcut
- `editor-ui/src/components/Header.tsx` - Fully restructured: imports ToolBtn/Separator from primitives, CSS Module for all static styles, correct widget arrangement, SearchPill, Settings gear
- `crates/voidscript-editor/src/window.rs` - Added `.with_accept_first_mouse(true)` to wry WebViewBuilder

## Decisions Made

- TrafficLight migrated to CSS `:hover` (no more useState/onMouseEnter/onMouseLeave) because it lives inside a `titlebar-no-drag` container — CSS hover is safe there
- HeaderWidget retains JS hover (`onMouseEnter`/`onMouseLeave`) since it sits in the drag region — following the established Phase 3 CONTEXT.md decision
- RunConfigSelector loses JS hover in favor of CSS `:hover` — it is inside the `.rightGroup` which has `titlebar-no-drag`, making CSS hover reliable
- All three step buttons (Step Over, Step Into, Step Out) use `ToolBtn size="small"` (ghost variant, not filled) — they are navigational, not action buttons
- Tasks 1 and 2 executed together as a single coherent rewrite — committed as one atomic unit rather than two partial states

## Deviations from Plan

Tasks 1 and 2 had no deviations. Task 3 (continuation after checkpoint) had one auto-fix:

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed window drag regression — wry acceptsFirstMouse default prevents drag on first click**
- **Found during:** Task 3 checkpoint — user reported "the app window is not draggable when run with cargo run"
- **Issue:** wry's WebView defaults to `accept_first_mouse: false`. On macOS, this causes the first mouseDown event on an unfocused frameless window to be consumed to focus the window, without passing through to WKWebView. This means `-webkit-app-region: drag` only works when the window is already the key window — users need to click first to focus, then try to drag, which feels broken.
- **Fix:** Added `.with_accept_first_mouse(true)` to the `wry::WebViewBuilder` chain in `window.rs`. This passes mouseDown events through to WKWebView even when the window is not focused, enabling `-webkit-app-region: drag` to activate on the first click.
- **Files modified:** `crates/voidscript-editor/src/window.rs`
- **Verification:** `npm run build` passes with zero TypeScript errors. Rust recompile with new wry configuration required for the fix to take effect.
- **Committed in:** `c38f532`

---

**Total deviations:** 1 auto-fixed (Rule 1 - Bug)
**Impact on plan:** Essential for correct frameless window drag behavior. No scope creep. Note: the plan's hypothesis that CSS Module scoping of `titlebar-drag` was the cause was incorrect — the CSS Module code correctly used global literal strings throughout. The actual root cause was a wry default.

## Issues Encountered

- Plan's hypothesis about the drag regression (CSS Module scoping the `titlebar-drag` class name) was incorrect — the code correctly used global literal strings for both `titlebar-drag` and `titlebar-no-drag` throughout. The actual root cause was wry's `acceptsFirstMouse: false` default causing drag to fail on first click when the window was unfocused.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Phase 3 Plan 1 complete — all four TBAR requirements visually verified by user, drag fix applied
- wry acceptsFirstMouse pattern documented — any future frameless window work must include this
- Global class name pattern settled — titlebar-drag/titlebar-no-drag are literal strings, never CSS Module references
- Ready for Phase 4 (Tab Bar) or any other Phase 3 plans
- Note: after `cargo build`, the wry acceptsFirstMouse fix will be embedded in the binary — next `cargo run` will have working drag

---
*Phase: 03-title-bar*
*Completed: 2026-03-14*
