# Phase 9: Polish and Tooltips - Context

**Gathered:** 2026-03-15
**Status:** Ready for planning

<domain>
## Phase Boundary

Custom tooltips replace all native browser `title` attributes; keyboard shortcut hints shown in tooltip text; breadcrumb bar below the tab strip reflects cursor position in VoidScript code. No new functionality — tooltips are visual replacements, shortcuts are display-only hints (not new keybindings), breadcrumb is informational (not navigational).

</domain>

<decisions>
## Implementation Decisions

### Tooltip visual styling
- Rider-dark tooltip: `#3C3F41` background, `#BBB` text, 1px `#555` border, no box-shadow
- Padding: 4px 8px, border-radius 4px
- Font: 12px Inter (matches Rider's tooltip size — slightly smaller than 13px UI text)
- Shortcut hint text rendered in `--text-secondary` (muted) after the label text, e.g. "Run `Shift+F10`"
- Single Tooltip primitive component in `src/primitives/Tooltip.tsx` with CSS Module

### Tooltip behavior
- Show delay: 800ms (Rider default for toolbar tooltips)
- Hide: instant on mouse leave
- Position: below the trigger by default; flip above if tooltip would clip viewport bottom
- Animation: opacity fade-in 100ms ease, no exit animation
- Only one tooltip visible at a time (moving between buttons resets the delay timer)

### Tooltip integration
- ToolBtn primitive gets tooltip support — replace native `title` attribute with custom Tooltip
- All `title=` attributes across components replaced: Header toolbar buttons, ToolStrip items, ScriptList actions, DebugPanel actions, BottomTabStrip actions, TrafficLight buttons, SearchPill
- ToolBtn interface: `title` prop continues to exist but renders custom Tooltip instead of native attribute

### Keyboard shortcut hints (display-only)
- Shortcuts shown in tooltip text, not wired as keybindings (wiring is a future task)
- Rider-standard shortcut assignments displayed:
  - Run: `Shift+F10`
  - Debug: `Shift+F9`
  - Stop: `Ctrl+F2`
  - Resume: `F9`
  - Step Over: `F8`
  - Step Into: `F7`
  - Step Out: `Shift+F8`
  - Search Everywhere: `Shift Shift` (already shown inline on the pill)
- ToolStrip items already have `shortcut` in their data — tooltip will use it
- Format in tooltip: `"Label (Shortcut)"` — consistent with existing ToolStrip title pattern

### Breadcrumb bar content
- VoidScript uses `StreamLanguage` (token-based, no Lezer syntax tree)
- Heuristic approach: scan document lines backwards from cursor to find enclosing `def` block
- Breadcrumb segments: `filename` › `function_name` (when cursor is inside a `def` block)
- When cursor is at top level (outside any `def`): just `filename`
- Nested blocks (e.g. `if` inside `def`) not tracked — only function-level granularity
- Separator: chevron `›` matching NavPath in status bar

### Breadcrumb bar placement and styling
- Positioned below TabBar, above Editor — matching Rider's breadcrumb bar location
- Full width of the editor area (between left and right panels)
- Height: 24px, matching status bar proportions
- Background: `--bg-panel` (#2B2D30), bottom border: `--border-default` (#393B40)
- Font: 12px Inter, `--text-secondary` for segments, `--text-primary` for last segment
- Non-interactive for now — display only, no click behavior

### Claude's Discretion
- Tooltip portal/positioning implementation (CSS absolute vs React portal)
- Exact heuristic for detecting `def` blocks (regex vs line scanning)
- Whether to debounce breadcrumb updates on rapid cursor movement
- Tooltip arrow/caret presence (Rider tooltips don't have arrows — likely skip)
- Edge case handling for deeply nested code in breadcrumb

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `ToolBtn` primitive (`src/primitives/ToolBtn.tsx`): Already accepts `title` prop — needs modification to render Tooltip instead of native attribute
- `StatusSegment` primitive: Shows similar pattern of icon + text with hover states
- `NavPath` component: Chevron separator and segment styling patterns reusable for breadcrumb
- `tokens.css`: Has `--bg-panel`, `--border-default`, `--text-primary`, `--text-secondary` tokens

### Established Patterns
- CSS Modules co-located with components
- 150ms ease hover transitions on interactive elements
- `var(--token)` references for all design values
- ToolStrip data already carries `shortcut` field per item
- TrafficLight and HeaderWidget use `title` attribute — both need migration

### Integration Points
- `ToolBtn.tsx`: Central place — changing `title` here covers most buttons automatically
- `Header.tsx`: SearchPill, TrafficLight, HeaderWidget have their own `title` usage
- `Editor.tsx`: EditorView.updateListener already fires on selection changes — breadcrumb can subscribe to cursor position
- `App.tsx`: Breadcrumb bar slots between TabBar and Editor in the layout
- `voidscript-lang.ts`: `StreamLanguage.define()` with token-based parsing — no tree, confirms heuristic approach for breadcrumb

</code_context>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches. All decisions based on Rider New UI reference and existing codebase patterns.

</specifics>

<deferred>
## Deferred Ideas

- Functional keyboard shortcuts (wiring actual keybindings for Run/Debug/Stop) — separate from display hints
- Breadcrumb click-to-navigate (clicking function name jumps to its definition) — future enhancement
- Tooltip for CodeMirror hover (e.g. variable type on hover) — requires language server, out of scope
- Autocomplete/suggestion tooltip styling — separate feature

</deferred>

---

*Phase: 09-polish-and-tooltips*
*Context gathered: 2026-03-15*
