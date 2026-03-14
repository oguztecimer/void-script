# Roadmap: VOID//SCRIPT Editor — Rider New UI Restyle

## Overview

This milestone transforms the working VOID//SCRIPT code editor into a pixel-accurate recreation of JetBrains Rider's New UI. The editor is already functionally complete; this roadmap covers the nine-phase visual and architectural restyle, starting with the CSS foundation that everything else depends on and finishing with tooltip and breadcrumb polish. Phases execute in strict dependency order: foundation before visible work, layout before resize logic, individual components before cross-cutting sweeps.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Foundation** - Self-hosted fonts, CSS token system, and macOS rendering fixes (completed 2026-03-14)
- [x] **Phase 2: CSS Architecture** - Primitive extraction, CSS Modules migration, hover pattern cleanup (completed 2026-03-14)
- [x] **Phase 3: Title Bar** - Pixel-accurate header with Search Everywhere and Settings gear (completed 2026-03-14)
- [x] **Phase 4: Tab Bar and Editor State** - Rider tab sizing, close-button hover-reveal, EditorState preservation (completed 2026-03-14)
- [ ] **Phase 5: Tool Strips and Panels** - 40px tool strip, panel headers, bottom panel tab strip
- [ ] **Phase 6: Status Bar** - Navigation path, icon+count diagnostics widget
- [ ] **Phase 7: Resizable Panels** - react-resizable-panels integration replacing fixed widths
- [ ] **Phase 8: Gutter Refinements** - Breakpoint overlay, fold icons on hover, CodeMirror theme polish
- [ ] **Phase 9: Polish and Tooltips** - Custom tooltips, keyboard shortcut hints, breadcrumb live integration

## Phase Details

### Phase 1: Foundation
**Goal**: Typography is correctly loaded, all design values live in a single token file, and macOS rendering matches Rider
**Depends on**: Nothing (first phase)
**Requirements**: FOUN-01, FOUN-02, FOUN-03, FOUN-05
**Success Criteria** (what must be TRUE):
  1. Inter variable font renders at 13px in all UI text without system fallback (verified via DevTools computed font-family)
  2. JetBrains Mono renders in the editor and console without system fallback
  3. A `tokens.css` file exists and every design value (color, dimension, typography) is referenced via `var(--token)` — no hardcoded hex in component CSS
  4. UI text weight matches Rider's Regular weight visually on macOS (font-smoothing applied, no artificial bold artifact)
  5. The browser renders in forced dark mode with no white flash on cold launch
**Plans:** 2/2 plans complete

Plans:
- [x] 01-01-PLAN.md — Install Fontsource fonts, create tokens.css, macOS rendering fixes
- [x] 01-02-PLAN.md — Migrate all components and theme to CSS tokens

### Phase 2: CSS Architecture
**Goal**: All component styles live in CSS Modules with real `:hover` pseudo-classes; shared atoms extracted as primitives
**Depends on**: Phase 1
**Requirements**: FOUN-04, PLSH-01, PLSH-02
**Success Criteria** (what must be TRUE):
  1. No `onMouseEnter`/`onMouseLeave` style mutations exist in any non-titlebar component
  2. A `src/primitives/` directory contains at minimum `ToolBtn`, `PanelHeader`, `Separator`, and `StatusSegment` components
  3. All interactive elements transition on hover with a consistent 150ms ease — verifiable by observing any button or tab in browser DevTools
  4. Panel borders, separators, and dividers use the correct 3-level color hierarchy (`#1E1F22`, `#393B40`, `#43454A`) with no deviations
**Plans:** 2/2 plans complete

Plans:
- [x] 02-01-PLAN.md — Create ToolBtn, PanelHeader, Separator, StatusSegment primitives with CSS Modules
- [ ] 02-02-PLAN.md — Migrate all non-titlebar components to CSS Modules with :hover and border hierarchy

### Phase 3: Title Bar
**Goal**: The header is a pixel-accurate Rider New UI title bar with all required toolbar widgets
**Depends on**: Phase 2
**Requirements**: TBAR-01, TBAR-02, TBAR-03, TBAR-04
**Success Criteria** (what must be TRUE):
  1. Widget buttons in the toolbar are 26px tall — verified by measuring in DevTools
  2. A magnifying-glass Search Everywhere button is visible in the toolbar center-right area
  3. A gear Settings icon button is visible at the far-right of the toolbar
  4. Hovering any toolbar button shows a state change without CSS `:hover` desync after a window drag (macOS drag region pitfall handled)
**Plans:** 1/1 plans complete

Plans:
- [ ] 03-01-PLAN.md — Migrate Header to CSS Module, replace local primitives, add SearchPill and Settings gear

### Phase 4: Tab Bar and Editor State
**Goal**: The tab bar matches Rider's height and spacing; tab close buttons behave correctly; editor state survives tab switching
**Depends on**: Phase 3
**Requirements**: TABS-01, TABS-02
**Success Criteria** (what must be TRUE):
  1. The tab bar is 38px tall with `0 16px` padding — verified in DevTools
  2. Close buttons on inactive tabs are hidden by default and appear on hover; the active tab always shows its close button
  3. Switching between tabs does not reset undo history or scroll position in either tab
**Plans:** 1/1 plans complete

Plans:
- [ ] 04-01-PLAN.md — Rider tab bar sizing, close-button hover-reveal, and EditorState preservation

### Phase 5: Tool Strips and Panels
**Goal**: Tool strips are the correct Rider width; all three side panels have Rider-style header chrome; the bottom panel has a proper tab strip; panels are resizable
**Depends on**: Phase 2
**Requirements**: PNLS-01, PNLS-02, PNLS-03, PNLS-04
**Success Criteria** (what must be TRUE):
  1. Left and right tool strips are 40px wide with 36px icon buttons
  2. ScriptList, DebugPanel, and Console panels each have a header row with a title and right-aligned action icons
  3. The bottom panel has a tab strip with a "Console" tab showing a 2px blue active indicator
  4. Users can drag panel resize handles to change the width of the side panels; sizes persist after closing and reopening the editor
**Plans:** 2 plans

Plans:
- [ ] 05-01-PLAN.md — SVG icons + active edge indicator for ToolStrip, panel header improvements
- [ ] 05-02-PLAN.md — BottomTabStrip extraction, react-resizable-panels DOM prep

### Phase 6: Status Bar
**Goal**: The status bar shows a navigation path and icon-based diagnostics matching Rider's layout
**Depends on**: Phase 2
**Requirements**: STAT-01, STAT-02
**Success Criteria** (what must be TRUE):
  1. The status bar left region shows a static navigation path in the form "project > folder > file"
  2. Diagnostics are shown as icon + count pairs: a red error circle with error count and a yellow warning triangle with warning count — not plain text
  3. The status bar is 24px tall and uses 11px Inter text
**Plans**: TBD

### Phase 7: Resizable Panels
**Goal**: All panels support drag-resize via react-resizable-panels; no hard-coded widths remain in the layout shell
**Depends on**: Phase 5
**Requirements**: PNLS-04
**Success Criteria** (what must be TRUE):
  1. Dragging a panel divider resizes adjacent panels smoothly without layout shift
  2. Collapsing a side panel does not unmount or visually flicker the main editor area
  3. Panel sizes are restored to their last values after a page reload
**Plans**: TBD

**Note:** PNLS-04 is listed here as the primary delivery phase. The `react-resizable-panels` integration in Phase 5 prepares the DOM structure; Phase 7 makes resize handles live and persistent.

### Phase 8: Gutter Refinements
**Goal**: The editor gutter matches Rider — breakpoints overlay line numbers and fold icons appear only on hover
**Depends on**: Phase 4
**Requirements**: EDIT-02, EDIT-03
**Success Criteria** (what must be TRUE):
  1. Fold/unfold icons are invisible on gutter rows where the cursor is not hovering; they appear on hover
  2. Breakpoint markers share the line-number gutter column (no separate breakpoint column) — a circle icon overlays the line number when a breakpoint is set
  3. All `.cm-*` style overrides live inside `EditorView.theme()` — no `.cm-*` rules exist in external CSS files
**Plans**: TBD

### Phase 9: Polish and Tooltips
**Goal**: Custom tooltips replace all native browser title attributes; keyboard shortcuts are shown in tooltips; breadcrumb reflects real syntax tree position
**Depends on**: Phase 8
**Requirements**: EDIT-01, PLSH-03, PLSH-04
**Success Criteria** (what must be TRUE):
  1. Hovering any interactive element shows a custom-styled Rider-dark tooltip — no native browser title attribute tooltips remain
  2. Run, Debug, and Stop button tooltips include keyboard shortcut hints (e.g., "Run (Shift+F10)")
  3. The breadcrumb bar below the tab strip updates to reflect the current cursor position in the VoidScript syntax tree as the user moves the caret
**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation | 2/2 | Complete   | 2026-03-14 |
| 2. CSS Architecture | 1/2 | Complete    | 2026-03-14 |
| 3. Title Bar | 1/1 | Complete   | 2026-03-14 |
| 4. Tab Bar and Editor State | 1/1 | Complete   | 2026-03-14 |
| 5. Tool Strips and Panels | 0/2 | Not started | - |
| 6. Status Bar | 0/? | Not started | - |
| 7. Resizable Panels | 0/? | Not started | - |
| 8. Gutter Refinements | 0/? | Not started | - |
| 9. Polish and Tooltips | 0/? | Not started | - |
