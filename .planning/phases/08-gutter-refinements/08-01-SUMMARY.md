---
phase: 08-gutter-refinements
plan: 01
subsystem: ui
tags: [codemirror, gutter, breakpoints, fold, lineNumberMarkers, EditorView.theme]

# Dependency graph
requires:
  - phase: 04-tab-bar-and-editor-state
    provides: Editor.tsx with buildExtensions, breakpointState StateField, EditorState cache
  - phase: 02-css-architecture
    provides: tokens.css with --accent-breakpoint, --text-tertiary, --text-secondary, --transition-hover
provides:
  - Breakpoint markers overlay line numbers in a single combined gutter column (no separate column)
  - Fold icons hidden by default, appear on hover with Rider-style triangles (▼/▶)
  - Faint breakpoint hover preview via CSS ::after pseudo-element (25% opacity)
  - All .cm-* gutter rules inside EditorView.theme() in voidscript-theme.ts
affects: [09-syntax-breadcrumb]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "lineNumberMarkers.computeN([stateField], state => [state.field(...)]) for dynamic gutter marker injection"
    - "foldGutter({ markerDOM }) for custom fold triangle markers with CSS opacity hover reveal"
    - "EditorView.theme() pseudo-element rules (.cm-lineNumbers .cm-gutterElement::after) for hover preview circles"

key-files:
  created: []
  modified:
    - editor-ui/src/components/Editor.tsx
    - editor-ui/src/codemirror/voidscript-theme.ts

key-decisions:
  - "lineNumberMarkers.computeN() used instead of ViewPlugin because the facet takes a static RangeSet — computeN derives it from breakpointState each time state changes"
  - "BreakpointOverlayMarker.toDOM() defined so lineNumberGutter.lineMarker suppresses the NumberMarker automatically (per @codemirror/view source line 11602: others.some(m => m.toDOM))"
  - "Breakpoint appear animation skipped (CSS transition does not fire on GutterMarker.toDOM() creation) — instant appear accepted for v1, deferred to future polish"
  - "Hover preview uses opacity 0.25 on ::after pseudo-element for faint but visible breakpoint affordance"

patterns-established:
  - "Pattern: Use lineNumberMarkers.computeN() to inject GutterMarkers with toDOM into line-number gutter — suppresses line number automatically"
  - "Pattern: CSS :hover on .cm-gutterElement (row cell, not column) for per-row hover effects inside EditorView.theme()"

requirements-completed: [EDIT-02, EDIT-03]

# Metrics
duration: 3min
completed: 2026-03-15
---

# Phase 8 Plan 01: Gutter Refinements Summary

**Breakpoint markers overlaying line numbers via lineNumberMarkers.computeN() facet, fold icons hidden by default with Rider-style triangles on hover, all .cm-* rules consolidated in EditorView.theme()**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-15T15:26:14Z
- **Completed:** 2026-03-15T15:29:36Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- Removed separate breakpoint gutter column; breakpoints now overlay line numbers using the `lineNumberMarkers` facet — the built-in `lineMarker` suppresses line numbers on breakpoint rows automatically
- `foldGutter` configured with `markerDOM` callback to render `▼`/`▶` triangle markers hidden by default (`opacity: 0`) and visible on gutter row hover (`opacity: 1`)
- Breakpoint hover preview: faint red circle (25% opacity) via `::after` pseudo-element on `.cm-lineNumbers .cm-gutterElement:hover::after`
- All `.cm-*` gutter rules live exclusively in `EditorView.theme()` in `voidscript-theme.ts` — zero `.cm-*` rules in external CSS files

## Task Commits

Each task was committed atomically:

1. **Task 1: Add gutter CSS rules to EditorView.theme()** - `293c5d6` (feat)
2. **Task 2: Replace breakpoint gutter with lineNumberMarkers facet and configure foldGutter** - `23490cb` (feat)

**Plan metadata:** (committed with final docs commit)

## Files Created/Modified
- `editor-ui/src/components/Editor.tsx` - Replaced BreakpointMarker with BreakpointOverlayMarker, removed createBreakpointGutter, added breakpointLineNumberMarkers via computeN, configured foldGutter with markerDOM, added mousedown handler to lineNumbers
- `editor-ui/src/codemirror/voidscript-theme.ts` - Added fold gutter rules (width, opacity, hover), breakpoint circle rules (cm-bp-circle), and hover preview pseudo-element rules

## Decisions Made
- `lineNumberMarkers.computeN([breakpointState], state => [state.field(breakpointState)])` used instead of a `ViewPlugin` because the facet's `.of()` requires a static `RangeSet` — `computeN` derives it from state reactively
- `BreakpointOverlayMarker.toDOM()` kept concrete so CodeMirror's `lineMarker` suppression fires for breakpoint rows
- CSS `animation` on `.cm-bp-circle` skipped: `GutterMarker.toDOM()` is called synchronously on element creation; CSS `transition` does not fire, only `@keyframes` would work. Accepted instant appear for v1.
- Hover selector targets `.cm-foldGutter .cm-gutterElement:hover .cm-fold-marker` (row cell hover), not `.cm-foldGutter:hover` (entire column), to show icon only on the hovered row

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] lineNumberMarkers.computeN() instead of .of() with function**
- **Found during:** Task 2 (breakpoint lineNumberMarkers wiring)
- **Issue:** Plan suggested `lineNumberMarkers.of(view => view.state.field(breakpointState))` but the facet type is `Facet<RangeSet<GutterMarker>>` — `.of()` requires a static `RangeSet`, not a function. TypeScript error: `Argument of type '(view: EditorView) => RangeSet<GutterMarker>' is not assignable to parameter of type 'RangeSet<GutterMarker>'`
- **Fix:** Used `lineNumberMarkers.computeN([breakpointState], state => [state.field(breakpointState)])` which derives the `RangeSet` from state reactively — simpler than a `ViewPlugin` and type-safe
- **Files modified:** editor-ui/src/components/Editor.tsx
- **Verification:** TypeScript compiles without errors; `lineNumberMarkers` properly wired
- **Committed in:** `23490cb` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - bug in plan's API usage)
**Impact on plan:** Necessary correction for type safety; semantics identical — breakpointState feeds lineNumberMarkers dynamically in both approaches.

## Issues Encountered
- `lineNumberMarkers.of()` does not accept a function — confirmed from TypeScript type `Facet<RangeSet<GutterMarker>>` and source. `computeN` is the correct reactive pattern for this use case.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- EDIT-02 and EDIT-03 requirements fulfilled
- Editor gutter matches JetBrains Rider layout: single combined line-number/breakpoint column, hover-only fold icons
- TypeScript compiles cleanly; frontend dist rebuild required to see visual changes (`npm run build` in editor-ui + `cargo build`)
- Phase 9 (syntax breadcrumb) can proceed — no gutter concerns remain

---
*Phase: 08-gutter-refinements*
*Completed: 2026-03-15*
