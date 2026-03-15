# Phase 6: Status Bar - Context

**Gathered:** 2026-03-15
**Status:** Ready for planning

<domain>
## Phase Boundary

Restyle the status bar to show a dynamic navigation path on the left and icon-based diagnostics (icon + count pairs) on the right, matching Rider's status bar layout. Remove the VCS branch widget. No new file management features — folder creation is deferred.

</domain>

<decisions>
## Implementation Decisions

### Navigation path content
- Path segments are dynamic based on actual script directory structure
- Default flat case: `VOID//SCRIPT › miner_brain.vs` (two segments)
- Nested case: `VOID//SCRIPT › combat › miner_brain.vs` (segments per directory level)
- Separator: chevron `›` character (Rider's breadcrumb style)
- Last segment (filename) rendered in `--text-primary`; all other segments and chevrons in `--text-secondary`
- When no file is open: show just `VOID//SCRIPT` (project name always visible)

### Status bar layout
- Navigation path on far left
- VCS branch widget removed entirely (cosmetic in a game editor — no real git repo)
- Right cluster order unchanged: diagnostics, `Ln X, Col Y`, LF, UTF-8, VoidScript
- Layout: `[nav path] — spacer — [diagnostics] [cursor] [encoding] [language]`

### Segment interactivity
- All path segments are clickable: project name opens ScriptList panel, folder segment filters to that folder, file segment scrolls to top
- Chevron separators are clickable as part of the segment to their left (larger hit target)
- Hover: Rider-style subtle — text color shifts to `--text-primary` on hover, no underline or background change
- Click behavior when no file is open: Claude's discretion

### Claude's Discretion
- Diagnostics icon SVG design (error circle, warning triangle) — match Rider's icon style
- Whether error/warning counts show at 0 or only when non-zero
- Whether to keep the green "OK" state when no diagnostics
- Combined diagnostics widget vs separate StatusSegments
- Click behavior on navigation path when no file is open

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `StatusSegment` primitive: icon + label + optional onClick, renders `<button>` or `<div>`, CSS Module with hover transitions
- `StatusBar.module.css`: already 24px height via `--height-statusbar`, 11px font via `--font-size-status`
- `StatusSegment.module.css`: padding, gap, hover states all established
- Zustand store: `activeTabId`, `tabs`, `cursorLine`, `cursorCol` already available
- `toggleLeftPanel` action in store for opening ScriptList

### Established Patterns
- CSS Modules + tokens.css for all styling
- `--height-statusbar: 24px`, `--font-size-status: 11px` tokens defined
- `--text-primary`, `--text-secondary` for text hierarchy
- `--accent-red`, `--accent-yellow`, `--accent-green` for diagnostic colors
- 150ms ease hover transitions on all interactive elements
- StatusSegment already supports icon + label pattern

### Integration Points
- `StatusBar.tsx` renders inside `App.tsx` layout shell at the bottom
- Script path data available from `tabs` array in Zustand store (each tab has `scriptId`)
- `toggleLeftPanel` in store can be called to open ScriptList panel
- Diagnostics data already read from `activeTab.diagnostics` array

</code_context>

<specifics>
## Specific Ideas

- Navigation path should feel like Rider's breadcrumb — subtle, informational, but interactive when needed
- Chevrons as part of click target gives a more forgiving hit area without visible affordance change
- Removing VCS branch simplifies the status bar and avoids fake context in a game editor

</specifics>

<deferred>
## Deferred Ideas

- Folder creation for organizing scripts into directories — new capability, separate phase
- Folder filtering behavior when clicking folder segments — depends on ScriptList supporting folder views

</deferred>

---

*Phase: 06-status-bar*
*Context gathered: 2026-03-15*
