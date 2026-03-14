# Phase 2: CSS Architecture - Context

**Gathered:** 2026-03-14
**Status:** Ready for planning

<domain>
## Phase Boundary

Migrate all component styles from inline React style props to CSS Modules with real CSS `:hover` pseudo-classes. Extract shared UI atoms (`ToolBtn`, `PanelHeader`, `Separator`, `StatusSegment`) into a `src/primitives/` directory. Establish consistent 150ms ease hover transitions and apply the 3-level border color hierarchy across all components. No new UI features — this is architectural cleanup that gates Phases 3-9.

</domain>

<decisions>
## Implementation Decisions

### Primitive component design
- `ToolBtn`: Composable API — size prop (default 36px, small 26px for toolbar widgets), optional label, optional badge/indicator. Icon-only is the common case.
- `PanelHeader`: Title text on the left, 1-3 right-aligned action icon buttons. No collapse toggle or flexible slots — keep it simple.
- `Separator`: Two variants — `line` (visible 1px border, color set by hierarchy level) and `gap` (invisible spacer with fixed width/height). Orientation prop for horizontal/vertical.
- `StatusSegment`: Icon + text pattern. Props: `icon?`, `label`, `onClick?`. Enforces consistent alignment across all status bar sections.

### Hover feedback style
- Rider-exact subtle fill: semi-transparent white overlay (`rgba(255,255,255,0.06)` or similar) on hover. No border change, no scale effect.
- Timing: 150ms ease on all interactive elements (from requirements PLSH-01)
- No visible focus ring — match Rider's dark UI approach
- Active/pressed states: Claude's discretion
- Disabled element treatment: Claude's discretion

### Border hierarchy
- 3-level system: `#1E1F22` (outer/darkest), `#393B40` (structural separators), `#43454A` (subtle dividers)
- All specific mapping decisions (panel edges, tool strip boundaries, tab bar borders, item dividers): Claude's discretion — match Rider as closely as possible

### Title bar exception scope
- Header.tsx hover migration strategy: Claude's discretion based on wry/WKWebView drag region behavior
- Whether primitives (ToolBtn) are reused inside Header or Header gets its own buttons: Claude's discretion
- Third-party widget hover scoping (CodeMirror gutter, etc.): Claude's discretion — only migrate React-owned components
- Final Header hover elimination: Claude's discretion, evaluate during Phase 3

### Claude's Discretion
- Active/pressed state visual treatment
- Disabled element treatment (opacity vs muted color)
- All border hierarchy placement — match Rider reference
- Title bar hover approach (CSS vs JS, permanent exception vs Phase 3 migration)
- Primitive reuse in Header vs separate header buttons
- Whether CodeMirror/third-party hover is in scope

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `tokens.css`: 59 CSS custom properties (colors, dimensions, typography) — all primitives and CSS Modules reference these via `var(--token)`
- `index.html` `<style>` block: inlined `:root` token definitions for wry compatibility

### Established Patterns
- All styling currently inline via React `style` props with hardcoded hex values (Phase 1 migrated values to `var()` references)
- Hover effects via `onMouseEnter`/`onMouseLeave` — 26 occurrences across 7 files (Header: 10, TabBar: 4, ScriptList: 4, StatusBar: 2, ToolStrip: 2, DebugPanel: 2, App: 2)
- CSS Modules co-located with components decided in Phase 1 (`Header.module.css` alongside `Header.tsx`)
- No CSS files exist yet beyond `index.html` inline styles and `tokens.css`

### Integration Points
- `editor-ui/src/components/`: All 7 component files need CSS Module migration
- `editor-ui/src/primitives/`: New directory for `ToolBtn`, `PanelHeader`, `Separator`, `StatusSegment`
- `editor-ui/src/App.tsx`: Layout shell — border hierarchy applies here for panel boundaries
- `vite.config.ts`: CSS Modules support is built into Vite — no additional config expected

</code_context>

<specifics>
## Specific Ideas

- Primitives define the visual language for Phases 3-9 — getting the API right here means less rework later
- Rider reference is the north star for all visual decisions (hover intensity, border placement, item dividers)
- The `line` + `gap` separator distinction comes from observing Rider's toolbar separators — some are visible lines, some are just spacing

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 02-css-architecture*
*Context gathered: 2026-03-14*
