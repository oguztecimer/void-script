# Phase 8: Gutter Refinements - Context

**Gathered:** 2026-03-15
**Status:** Ready for planning

<domain>
## Phase Boundary

Make the editor gutter match JetBrains Rider: breakpoint markers overlay line numbers in a single combined gutter column (remove separate breakpoint column), and fold icons appear only on hover. All `.cm-*` style overrides must live inside `EditorView.theme()` — no `.cm-*` rules in external CSS files.

</domain>

<decisions>
## Implementation Decisions

### Breakpoint Overlay Appearance
- Rider-exact: red circle completely replaces the line number — number vanishes, filled red circle sits in its place
- Filled circle, ~12px diameter, centered in the gutter cell, sized to fit without touching adjacent rows
- Uses existing `--accent-breakpoint` CSS token for the circle color
- No line number reveal mechanism — number is simply gone when breakpoint is set; adjacent numbers make position obvious

### Fold Icon Hover Behavior
- Fold icons hidden by default, appear on hover anywhere on the gutter row (not just the fold column)
- Rider-style filled triangles: `▶` (right-pointing) for collapsed, `▼` (down-pointing) for expanded
- Color: `--text-tertiary` (matching line number muted color), brightens slightly on hover (matching tool strip icon pattern from Phase 5)
- Fixed-width fold gutter column (~14px) always reserves space; icons use `opacity: 0` when not hovered — prevents gutter width jumping on hover

### Gutter Click Targets
- Clicking anywhere in the gutter column always toggles a breakpoint on that line (no click-to-select-line behavior)
- Faint/translucent red circle preview appears on hover, indicating "click here to set breakpoint" (Rider-style subtle preview)
- No right-click context menu — keep it simple for now
- Breakpoint appear/disappear uses 150ms fade animation (consistent with established 150ms ease transition pattern throughout the app)

### Active Line Gutter Highlight
- Both active line highlight and breakpoint circle visible simultaneously — highlight background shows with red circle layered on top
- Active line number stays brighter than other line numbers: `--text-secondary` for active, `--text-tertiary` for others (current behavior, matches Rider)
- Breakpoint hover preview shows consistently on all lines including the active line
- Breakpoint always wins on active line — red circle shown, line number hidden, regardless of active line status

### Claude's Discretion
- Exact implementation approach for combining breakpoint + line number into a single custom gutter (custom GutterMarker subclass vs lineNumbers() configuration)
- How to wire the gutter row hover detection for fold icon visibility (CSS :hover on gutter row vs EditorView.domEventHandlers)
- Internal CodeMirror extension composition (ordering of gutter extensions, StateField design)
- Exact opacity values for the faint breakpoint hover preview

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `BreakpointMarker` class (`Editor.tsx:27-37`): existing GutterMarker subclass rendering `●` character — needs reimplementation to render a filled circle overlaying line number
- `breakpointState` StateField (`Editor.tsx:43-60`): manages breakpoint RangeSet — reusable as-is
- `toggleBreakpointEffect` StateEffect (`Editor.tsx:41`): toggle breakpoint command — reusable as-is
- `voidScriptTheme` (`voidscript-theme.ts`): EditorView.theme() with existing `.cm-gutters`, `.cm-activeLineGutter` rules — extend here

### Established Patterns
- All CodeMirror theme overrides in `EditorView.theme()` inside `voidscript-theme.ts` (EDIT-03 requires no `.cm-*` rules in external CSS)
- `var(--token)` references for all colors — no hardcoded hex in theme
- `foldGutter()` called with defaults in `buildExtensions()` — needs configuration for custom markers and hover behavior
- `lineNumbers()` called with defaults — may need replacement with custom gutter for combined breakpoint+number column

### Integration Points
- `buildExtensions()` function (`Editor.tsx:113-154`): where all gutter extensions are composed — `lineNumbers()`, `createBreakpointGutter()`, `foldGutter()` all configured here
- `createBreakpointGutter()` (`Editor.tsx:62-84`): creates the separate breakpoint gutter — needs to be merged into line number gutter
- Zustand store `toggleBreakpoint` and `breakpoints` state — click handler interaction unchanged

</code_context>

<specifics>
## Specific Ideas

- The gutter should feel identical to Rider's — single clean column with line numbers, red circles replacing numbers where breakpoints are set, fold triangles appearing on hover
- Breakpoint preview on hover gives the gutter an interactive, polished feel without being distracting
- The 150ms fade on breakpoint toggle maintains the app's established animation language

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 08-gutter-refinements*
*Context gathered: 2026-03-15*
