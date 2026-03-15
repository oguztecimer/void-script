# Phase 7: Resizable Panels - Context

**Gathered:** 2026-03-15
**Status:** Ready for planning

<domain>
## Phase Boundary

Make all panels support drag-resize via react-resizable-panels; eliminate hard-coded dimensions from the layout shell. Side panels (left/right) are already resizable from Phase 5; this phase adds bottom panel resize, collapse interactions, and polishes the overall resize experience.

</domain>

<decisions>
## Implementation Decisions

### Bottom Panel Resize
- Nested vertical `Group` (react-resizable-panels) inside center panel, wrapping editor area and bottom panel
- Percentage-based constraints: min 10%, max 50% of center area height
- Persist height via `useDefaultLayout` with a named storage key (consistent with horizontal group)
- Collapsible by dragging past minimum size, synced to Zustand `bottomPanelOpen` state
- Default height: 25% of center area

### Resize Handle Appearance
- No hover feedback — handle stays as plain 1px `border-strong` line (matches Rider)
- No drag feedback — no color change during active drag
- No grip indicator (dots/lines) — Rider doesn't show grip marks on panel dividers
- Vertical (bottom panel) handle uses `row-resize` cursor; horizontal handles keep `col-resize`

### Collapse Behavior
- Double-click a resize handle toggles collapse/expand of the adjacent panel
- Snap-to-collapse threshold: dragging below 50% of minimum size auto-collapses to zero
- All collapse interactions (drag snap, double-click) sync to Zustand state (`leftPanelOpen`, `rightPanelOpen`, `bottomPanelOpen`)
- Collapse animation: 150ms ease (existing `panelAnimated` class)

### Panel Size Defaults
- Side panels: 18% default width (already set, unchanged)
- Bottom panel: 25% of center area height
- No "reset layout" UI option — users drag to preferred sizes; Rider doesn't have a visible reset
- Named storage keys for layout persistence (discoverable in localStorage)

### Claude's Discretion
- Vertical group storage key naming convention
- Implementation details of double-click handler on Separator
- How to wire snap-to-collapse threshold with react-resizable-panels v4 API
- Whether the bottom panel vertical group needs its own `onLayoutChange`/`onLayoutChanged` callbacks or shares with the horizontal group

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `react-resizable-panels` v4.7.3: `Group`, `Panel`, `Separator`, `PanelImperativeHandle`, `useDefaultLayout` already imported in App.tsx
- `panelAnimated` CSS class: `transition: flex-basis 150ms ease` — reusable for bottom panel collapse animation
- `resizeHandle` / `resizeHandleHidden` CSS classes: styling for visible/hidden resize handles
- Zustand store: `bottomPanelOpen`, `toggleBottomPanel` already exist for open/close state

### Established Patterns
- Imperative collapse/expand via `panelRef` synced to Zustand state with `useEffect` (left/right panels)
- `useDefaultLayout({ id: 'void-main-layout' })` for horizontal layout persistence
- `onLayoutChange` sets `isResizing` to disable animation during drag; `onLayoutChanged` re-enables and saves
- Separator `disabled` prop + `resizeHandleHidden` class when panel is collapsed

### Integration Points
- `App.tsx` center panel div (`.center`) needs to become a vertical `Group` wrapping editor area and bottom panel
- `.bottomPanel` CSS class currently has `height: 200px` — must be removed in favor of Panel `defaultSize`
- `BottomTabStrip` component renders inside the bottom panel div
- `bottomPanelOpen` Zustand state needs sync with bottom panel imperative collapse/expand (matching left/right pattern)

</code_context>

<specifics>
## Specific Ideas

- Bottom panel resize should feel identical to side panel resize — same animation timing, same collapse behavior, same persistence approach
- Keep handles minimal (Rider reference) — no hover highlights, no grip indicators, just a clean line

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 07-resizable-panels*
*Context gathered: 2026-03-15*
