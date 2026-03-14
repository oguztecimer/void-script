# Phase 5: Tool Strips and Panels - Context

**Gathered:** 2026-03-14
**Status:** Ready for planning

<domain>
## Phase Boundary

Restyle tool strips to Rider's 40px width with 36px icon buttons, add Rider-style panel headers with action icons to all three panels, create a proper bottom panel tab strip with active indicator, and implement resizable side panels with drag handles.

</domain>

<decisions>
## Implementation Decisions

### Panel Header Actions
- ScriptList header: close button + add-script (plus icon) button
- DebugPanel header: Claude's discretion on structure and actions
- Console header: Claude's discretion on whether tab strip row serves as header or separate PanelHeader inside
- Action icon hover behavior: Claude's discretion

### Bottom Panel Tab Strip
- Tab selection (which tabs appear): Claude's discretion
- Visibility toggle behavior (fully hideable vs always-visible strip): Claude's discretion
- Active tab click behavior (toggle vs no-op): Claude's discretion
- Tab strip sizing relative to top TabBar: Claude's discretion

### Panel Resize
- Overall resize approach (DOM prep vs full implementation vs CSS variables): Claude's discretion
- Side panel widths must have min/max constraints (e.g. min 150px, max 50% viewport) — panels cannot crush the editor area
- Collapsing a panel animates the width transition (~150ms ease, matching established hover transitions)
- Reopening a collapsed panel restores its last dragged width, not a default

### Tool Strip Icons
- Replace emoji text icons with simple monochrome SVG icons matching Rider's style
- Icon color: tertiary (`--text-tertiary`) by default, brighten on hover
- Rotated text labels alongside icons: Claude's discretion
- Active tool strip button: 2px colored border indicator on the inner edge (left edge for left strip, right edge for right strip)

### Claude's Discretion
- DebugPanel header structure and action icons
- Console panel header approach
- Action icon hover states in panel headers
- Bottom panel tab selection, visibility, click behavior, and sizing
- Panel resize implementation strategy (Phase 7 dependency consideration)
- Whether tool strip buttons show rotated text labels

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `PanelHeader` primitive: already has title + actions API, used by ScriptList and DebugPanel
- `ToolBtn` primitive: `default` (36px) and `small` sizes, CSS Module with hover transitions
- `BottomTab` inline component in App.tsx: basic tab with 2px blue active indicator — needs extraction or replacement
- `Separator` primitive: available for visual dividers

### Established Patterns
- CSS Modules + tokens.css for all styling (no inline styles, no CSS-in-JS)
- `--width-toolstrip: 40px` and `--size-toolstrip-btn: 36px` tokens already defined
- `--accent-blue` token used for active indicators (tab underline, ToolStrip active)
- 150ms ease hover transitions on all interactive elements
- `opacity:0 + pointer-events:none` for hover-reveal pattern (Phase 4 close buttons)
- Zustand store manages panel open/close state (`toggleLeftPanel`, `toggleRightPanel`, `toggleBottomPanel`, `bottomPanelTab`)

### Integration Points
- `App.tsx` is the layout shell — tool strips, panels, and bottom panel all render here
- `store.ts` holds `leftPanelOpen`, `rightPanelOpen`, `bottomPanelOpen`, `bottomPanelTab` state
- Panel width persistence will need Zustand state or localStorage
- Tool strip items defined as arrays in App.tsx (`LEFT_ITEMS`, `RIGHT_ITEMS`)

</code_context>

<specifics>
## Specific Ideas

- Active tool strip indicator should be a 2px bar on the inner edge matching Rider's visual language (blue accent color)
- Tool strip icons should be tertiary color by default and brighten on hover — subtle until interacted with
- Panel collapse/expand should feel smooth with animation, not jarring snap transitions

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 05-tool-strips-and-panels*
*Context gathered: 2026-03-14*
