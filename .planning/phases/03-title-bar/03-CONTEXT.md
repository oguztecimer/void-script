# Phase 3: Title Bar - Context

**Gathered:** 2026-03-14
**Status:** Ready for planning

<domain>
## Phase Boundary

Pixel-accurate Rider New UI title bar with all required toolbar widgets. Migrate Header.tsx from inline styles to CSS Module. Add Search Everywhere pill and Settings gear icon. Match Rider's exact widget arrangement, separator grouping, and 26px uniform button height. No new functionality — Search Everywhere is a visual shell only (actual search is v2).

</domain>

<decisions>
## Implementation Decisions

### Search Everywhere widget
- Rider-style search pill, not a simple icon button
- Fixed width ~200-240px, matching Rider's proportions
- Content: magnifying glass icon + "Search" text + "⇧⇧" shortcut hint (right-aligned, muted)
- Non-functional for now — visual shell only (SRCH-01/SRCH-02 are v2)

### Toolbar widget arrangement
- Match Rider's exact layout: `[traffic lights] | [hamburger] | [◀ ▶] | [project ▾] [VCS] | ---spacer--- | [run config ▾] [▶] [🪲] | [search pill] | [⚙]`
- Keep non-functional widgets (hamburger, back/forward) for visual completeness
- Separators only between groups — no separator between project and VCS, no separator between run config and action buttons
- Settings gear icon at far-right position

### Debug controls arrangement
- When debugging and paused: `[⬛ Stop] | [▶ Resume] [⤵ Step Over] [↓ Step Into] [↑ Step Out]`
- Stop separated from Resume + stepping buttons (matching Rider)
- Drop the "Running..."/"Debugging..."/"Paused" status text — state communicated through visible buttons only

### Run/Debug controls styling
- Reuse ToolBtn primitive with `variant="filled"` for Run/Debug/Stop/Resume buttons
- Run config selector stays as a header-specific component, migrated to CSS Module
- All toolbar buttons uniformly 26px tall (icon buttons, action buttons, compound widgets)

### Header CSS migration
- Full CSS Module migration — Header.tsx gets `Header.module.css`, all inline styles move to CSS classes
- Completes the Phase 2 deferral for title bar hover migration
- Replace local ToolBtn and ActionBtn with ToolBtn primitive from `src/primitives/`
- Replace local Separator with Separator primitive from `src/primitives/`
- Keep WindowControls, TrafficLight, HeaderWidget, RunConfigSelector as header-specific local components

### macOS drag region + hover
- Pragmatic split: JS hover (`onMouseEnter`/`onMouseLeave`) for buttons directly in the drag region, CSS `:hover` for buttons inside `titlebar-no-drag` containers
- Traffic lights migrate to CSS Module with `:hover` (they're in a no-drag zone)
- This avoids the known WKWebView desync where `:hover` gets stuck after a window drag

### Claude's Discretion
- Exact pixel widths for search pill (within ~200-240px range)
- Icon sizing and SVG details for Search Everywhere magnifying glass and Settings gear
- Exact gap/margin values between widget groups (match Rider reference)
- Border radius on search pill and widget buttons
- HeaderWidget internal styling (icon + label + chevron compound layout)
- How to structure the CSS Module (class naming, grouping)

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `ToolBtn` primitive (`src/primitives/ToolBtn.tsx`): 36px default / 26px small, ghost/filled variants, CSS Module with CSS custom property `--_btn-hover-bg` for filled hover — directly reusable for all toolbar icon buttons and action buttons
- `Separator` primitive (`src/primitives/Separator.tsx`): line/gap variants, horizontal/vertical — replaces local Header separator
- `tokens.css`: 59 CSS custom properties including `--bg-toolbar`, `--border-strong`, `--bg-hover`, `--bg-btn-run`, `--bg-btn-debug`, `--bg-btn-stop` and their hover variants

### Established Patterns
- CSS Modules co-located with components (e.g., `ToolBtn.module.css` alongside `ToolBtn.tsx`)
- CSS `:hover` with 150ms ease transitions on all interactive elements
- `var(--token)` references for all design values — no hardcoded hex
- ToolBtn uses `className` prop for external styling, `style` prop for dynamic CSS custom properties

### Integration Points
- `Header.tsx` (`editor-ui/src/components/`): Current 379-line component with 7 local sub-components — needs full restructure
- `App.tsx`: Renders `<Header />` at top of layout — no changes needed
- `index.html`: `titlebar-drag` and `titlebar-no-drag` CSS classes defined here for wry drag regions
- `store.ts`: Zustand state already provides `isRunning`, `isDebugging`, `isPaused`, `activeTabId`, `tabs`

</code_context>

<specifics>
## Specific Ideas

- Search Everywhere pill should look like Rider's — a visually prominent, wide rounded rectangle that anchors the right side of the toolbar
- The toolbar should feel like opening Rider — every widget in the right place, right size, right grouping
- Debug stepping buttons appear as a cohesive group separated from Stop, matching Rider's debugging toolbar state

</specifics>

<deferred>
## Deferred Ideas

- Search Everywhere actual functionality (fuzzy search, modal, tabbed results) — v2 requirements SRCH-01/SRCH-02
- Settings menu/panel behind the gear icon — future phase

</deferred>

---

*Phase: 03-title-bar*
*Context gathered: 2026-03-14*
