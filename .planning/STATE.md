---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
status: completed
stopped_at: Completed 06-status-bar-01-PLAN.md
last_updated: "2026-03-15T10:36:12.183Z"
last_activity: 2026-03-14 — Phase 2 Plan 2 complete (7 components migrated to CSS Modules)
progress:
  total_phases: 9
  completed_phases: 6
  total_plans: 9
  completed_plans: 9
  percent: 100
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-14)

**Core value:** The code editor must look and feel like JetBrains Rider's New UI — professional, polished, and immediately familiar to developers.
**Current focus:** Phase 2 complete — CSS Architecture

## Current Position

Phase: 2 of 9 complete (CSS Architecture)
Plan: 2/2 plans complete in Phase 2
Status: Phase 2 complete, ready for Phase 3
Last activity: 2026-03-14 — Phase 2 Plan 2 complete (7 components migrated to CSS Modules)

Progress: [██████████] 100%

## Performance Metrics

**Velocity:**
- Total plans completed: 0
- Average duration: -
- Total execution time: 0 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| - | - | - | - |

**Recent Trend:**
- Last 5 plans: -
- Trend: -

*Updated after each plan completion*
| Phase 01-foundation P01 | 7 min | 3 tasks | 5 files |
| Phase 01-foundation P02 | 15 min | 3 tasks | 16 files |
| Phase 02-css-architecture P01 | 2 min | 2 tasks | 9 files |
| Phase 02-css-architecture P02 | 4min | 2 tasks | 14 files |
| Phase 03-title-bar P01 | 15min | 2 tasks | 2 files |
| Phase 03-title-bar P01 | 30min | 3 tasks | 3 files |
| Phase 04-tab-bar-and-editor-state P01 | 3min | 2 tasks | 2 files |
| Phase 05-tool-strips-and-panels P01 | 1 min | 2 tasks | 6 files |
| Phase 05-tool-strips-and-panels P02 | 4 min | 2 tasks | 8 files |
| Phase 06-status-bar P01 | 1min | 2 tasks | 6 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- Init: Editor-first milestone — players spend 90% of time in the editor; foundation must land before any visual comparison
- Init: CSS Modules + tokens.css approach confirmed; no CSS-in-JS (zero runtime cost critical for keystroke re-renders)
- Init: Fontsource npm packages chosen over CDN fonts to avoid wry custom protocol CORS surface
- [Phase 01-foundation]: html { background: #1E1F22 } in index.html stays hardcoded — CSS tokens unavailable at boot time before JS bundle parses
- [Phase 01-foundation]: mime_guess v2 correctly returns font/woff2 for .woff2 — no explicit fallback needed in wry get_asset()
- [Phase 01-foundation]: tokens.css uses --{category}-{variant} naming convention; all 59 tokens available for Phase 2+ to reference instead of hardcoded hex
- [Phase 01-foundation]: CSS var() works in React inline style strings — fontFamily: 'var(--font-mono)' resolves at browser render time
- [Phase 01-foundation]: tokens.css already contained all component-specific tokens from Plan 01 — no new tokens needed for migration
- [Phase 01-foundation P02]: CRITICAL — wry custom protocol (WKWebView) does NOT apply CSS :root blocks from external stylesheets. Production fix: inline all :root token definitions in index.html <style> block. tokens.css retained for dev server only.
- [Phase 01-foundation P02]: Access-Control-Allow-Origin: * added to wry get_asset() responses; Vite crossOriginLoading: false prevents crossorigin attribute conflicts
- [Phase 02-css-architecture P01]: CSS custom property --_btn-hover-bg for filled variant hover avoids JS handlers while supporting per-instance colors
- [Phase 02-css-architecture P01]: StatusSegment renders <button> when onClick provided, <div> otherwise for semantic HTML
- [Phase 02-css-architecture P01]: CSS Modules type declaration (css-modules.d.ts) required for TypeScript to resolve *.module.css imports
- [Phase 02-css-architecture P02]: Bottom panel header uses CSS Module div instead of PanelHeader primitive: API takes string title but bottom panel needs BottomTab ReactNode on left side
- [Phase 03-title-bar]: TrafficLight uses CSS :hover (no useState) — in titlebar-no-drag container
- [Phase 03-title-bar]: HeaderWidget retains JS hover for drag-zone safety; RunConfigSelector switches to CSS :hover (inside no-drag rightGroup)
- [Phase 03-title-bar]: No separator between project widget and VCS branch widget — matches Rider exact layout
- [Phase 03-title-bar]: wry with_accept_first_mouse(true) required for frameless window drag — default false absorbs first mouseDown to focus window, preventing -webkit-app-region: drag on first click
- [Phase 04-tab-bar-and-editor-state]: EditorState cache is module-level Map (not Zustand) — EditorState is large/non-serializable per CodeMirror docs
- [Phase 04-tab-bar-and-editor-state]: scrollSnapshot() returns StateEffect<ScrollTarget>, dispatched after view construction via dispatch(), not passed as Extension
- [Phase 04-tab-bar-and-editor-state]: opacity:0 + pointer-events:none for close button hiding — preserves tab width preventing layout shift
- [Phase 05-tool-strips-and-panels]: className prop (not active prop) used for strip active state — avoids solid blue fill; compound CSS selectors place inset shadow on correct inner edge
- [Phase 05-tool-strips-and-panels]: create_script added to JsToRustMessage union immediately as harmless type stub setting up IPC contract for future backend implementation
- [Phase 05-tool-strips-and-panels]: react-resizable-panels v4 API adaptation: Group/Separator/panelRef/useDefaultLayout instead of v2 PanelGroup/PanelResizeHandle/ref/autoSaveId
- [Phase 05-tool-strips-and-panels]: Drag detection via Group onLayoutChange/onLayoutChanged callbacks — v4 Separator has no onDragging prop
- [Phase 06-status-bar]: NavPath project segment is <button> (toggleLeftPanel); folder/file are <span> to avoid implying non-existent functionality
- [Phase 06-status-bar]: TYPE_LABELS/TYPE_ORDER extracted to state/scriptTypes.ts — shared by NavPath and ScriptList
- [Phase 06-status-bar]: Chevron inside preceding segment button as .chevron span keeps --text-secondary on parent hover

### Pending Todos

None yet.

### Blockers/Concerns

- Phase 4: CodeMirror 6 EditorState map pattern with Zustand needs careful design — see discuss.codemirror.net "Preserving state when switching between files"
- Phase 8: Breakpoint overlay gutter requires a custom GutterMarker implementation; no off-the-shelf solution exists
- Phase 9: VoidScript parser syntax node type names are not documented; need to read parser source before breadcrumb implementation

## Session Continuity

Last session: 2026-03-15T10:33:35.848Z
Stopped at: Completed 06-status-bar-01-PLAN.md
Resume file: None
