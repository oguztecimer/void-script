---
phase: 05-tool-strips-and-panels
plan: "02"
subsystem: editor-ui
tags: [bottom-panel, tab-strip, react-resizable-panels, zustand, imperative-panel, css-modules]
dependency_graph:
  requires: [05-01]
  provides: [BottomTabStrip component (PNLS-02 Console header), react-resizable-panels layout (PNLS-03, PNLS-04), imperative panel collapse/expand]
  affects:
    - editor-ui/src/components/BottomTabStrip.tsx
    - editor-ui/src/components/BottomTabStrip.module.css
    - editor-ui/src/App.tsx
    - editor-ui/src/App.module.css
    - editor-ui/src/state/store.ts
    - editor-ui/src/components/ScriptList.module.css
    - editor-ui/src/components/DebugPanel.module.css
    - editor-ui/package.json
tech_stack:
  added: [react-resizable-panels@4.7.3]
  patterns:
    - v4 Group/Panel/Separator API (not v2 PanelGroup/PanelResizeHandle)
    - panelRef prop on Panel (not ref) for imperative handle
    - useDefaultLayout hook for localStorage persistence (not autoSaveId prop)
    - onLayoutChange/onLayoutChanged on Group for drag detection
    - Zustand useEffect watchers for imperative collapse/expand syncing
key_files:
  created:
    - editor-ui/src/components/BottomTabStrip.tsx
    - editor-ui/src/components/BottomTabStrip.module.css
  modified:
    - editor-ui/src/App.tsx
    - editor-ui/src/App.module.css
    - editor-ui/src/state/store.ts
    - editor-ui/src/components/ScriptList.module.css
    - editor-ui/src/components/DebugPanel.module.css
    - editor-ui/package.json
decisions:
  - react-resizable-panels v4 API differs from researched v2 API — adapted automatically (Group not PanelGroup, Separator not PanelResizeHandle, panelRef prop not ref, useDefaultLayout hook not autoSaveId prop, onLayoutChange/onLayoutChanged for drag detection not onDragging on handle)
  - isResizing toggled via Group onLayoutChange/onLayoutChanged callbacks rather than Separator onDragging prop (which does not exist in v4)
  - center Panel has no defaultSize — takes remaining flex space automatically as Group distributes unallocated percentage
metrics:
  duration: "4 min"
  completed_date: "2026-03-14"
  tasks_completed: 2
  files_modified: 8
---

# Phase 05 Plan 02: BottomTabStrip and react-resizable-panels Layout Summary

**One-liner:** BottomTabStrip component wired to Zustand satisfies PNLS-02 Console header; App layout wrapped in react-resizable-panels v4 Group with always-rendered imperative-collapse side panels, 150ms ease animation, and localStorage persistence.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Extract BottomTabStrip component | f506f15 | BottomTabStrip.tsx, BottomTabStrip.module.css, App.tsx, App.module.css, store.ts |
| 2 | Install react-resizable-panels and wrap App layout | b5d190e | package.json, App.tsx, App.module.css, ScriptList.module.css, DebugPanel.module.css |

## What Was Built

### Task 1: BottomTabStrip component

- `BottomTabStrip.tsx`: standalone component using `useStore` selectors for `bottomPanelTab`, `setBottomPanelTab`, `clearConsole`, `toggleBottomPanel`; renders Console tab with 2px blue active indicator; right-aligned ToolBtn action icons (clear, close). Serves as Console panel header (PNLS-02).
- `BottomTabStrip.module.css`: `.strip` flex layout, `.tab` with `border-bottom: 2px solid transparent`, `.active` with `border-bottom-color: var(--accent-blue)`, `.actions` flex gap
- `store.ts`: `bottomPanelTab` default changed from `'run'` to `'console'`
- `App.tsx`: removed inline `BottomTab` function, removed `ToolBtn` import, removed `toggleBottomPanel` selector (moved to BottomTabStrip), replaced bottom panel header div with `<BottomTabStrip />`
- `App.module.css`: removed `.bottomPanelHeader`, `.bottomTabs`, `.bottomActions`, `.bottomTab`, `.bottomTabActive` (all moved to BottomTabStrip.module.css); kept `.bottomPanel` container

### Task 2: react-resizable-panels layout

- `package.json`: added `react-resizable-panels@^4.7.3`
- `App.tsx`: full layout rewrite using v4 Group/Panel/Separator API; `leftPanelRef`/`rightPanelRef` using `useRef<PanelImperativeHandle>` passed as `panelRef` prop; `useDefaultLayout({ id: 'void-main-layout' })` for localStorage persistence; side panels always rendered with `collapsible` + `collapsedSize={0}`; `useEffect` watchers for imperative `collapse()`/`expand()` synced to Zustand state; `isResizing` state toggled by Group `onLayoutChange`/`onLayoutChanged` to disable `panelAnimated` class during drag
- `App.module.css`: added `.panelGroup` (flex:1), `.resizeHandle` (1px border-strong col-resize), `.panelAnimated` (transition: flex-basis 150ms ease)
- `ScriptList.module.css`: removed `width: 220px`; added `height: 100%`
- `DebugPanel.module.css`: removed `width: 250px`; added `height: 100%`

## Decisions Made

1. **react-resizable-panels v4 API adaptation** — npm installed v4.7.3 (latest) instead of the v2 API documented in research. All API mappings updated: `PanelGroup` → `Group`, `PanelResizeHandle` → `Separator`, `direction` → `orientation`, `ref` → `panelRef`, `ImperativePanelHandle` → `PanelImperativeHandle`, `autoSaveId` → `useDefaultLayout` hook.

2. **Drag detection via Group callbacks** — v4 `Separator` has no `onDragging` prop. Instead, `isResizing` is set `true` by Group `onLayoutChange` (fires each pointer move) and `false` by `onLayoutChanged` (fires after pointer release). This achieves the same "disable animation during drag" behavior.

3. **center Panel has no defaultSize** — The center panel takes the remaining flex space automatically. react-resizable-panels v4 distributes unallocated percentage to panels without `defaultSize`, which is the correct behavior for the main editor area.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Adapted v2 API to installed v4 API**
- **Found during:** Task 2 (first build attempt)
- **Issue:** npm installed react-resizable-panels v4.7.3 (latest). The plan was written for v2 API which exports `PanelGroup`, `PanelResizeHandle`, `ImperativePanelHandle` and uses `direction`, `ref`, `autoSaveId`, `onDragging` props. v4 exports `Group`, `Separator`, `PanelImperativeHandle` and uses `orientation`, `panelRef`, `useDefaultLayout` hook, `onLayoutChange`/`onLayoutChanged` callbacks.
- **Fix:** Rewrote App.tsx imports and component tree for v4 API while preserving all intended behavior: always-rendered collapsible panels, imperative collapse/expand, 150ms ease animation disabled during drag, localStorage persistence.
- **Files modified:** `editor-ui/src/App.tsx`
- **Commit:** b5d190e

## Verification

- `npm run build` passes with zero TypeScript errors after both tasks
- CSS warnings in build output are pre-existing comment syntax issues in tokens.css, unrelated to this plan
- All plan must_haves satisfied:
  - Bottom panel has BottomTabStrip with Console tab and 2px blue active indicator
  - Tab clicking wired to Zustand `setBottomPanelTab`
  - BottomTabStrip serves as Console panel header with title and action icons (PNLS-02)
  - App layout wrapped in react-resizable-panels Group with side panels as always-rendered collapsible Panels
  - Closing calls `panel.collapse()` via imperative ref — Panel never unmounts
  - Reopening calls `panel.expand()` which restores last dragged size (v4 imperative expand behavior)
  - Collapse/expand animates via `.panelAnimated` 150ms ease (removed during drag via `isResizing`)
  - Side panels no longer have hardcoded pixel widths
  - Tool strips remain fixed-width outside the Group
  - Separator elements render as resize handles with 1px border-strong, col-resize cursor

## Self-Check: PASSED

- `editor-ui/src/components/BottomTabStrip.tsx` — exists
- `editor-ui/src/components/BottomTabStrip.module.css` — exists
- `editor-ui/src/App.tsx` — modified, imports Group/Panel/Separator/PanelImperativeHandle/useDefaultLayout
- `editor-ui/src/App.module.css` — contains panelAnimated
- `editor-ui/src/state/store.ts` — bottomPanelTab: 'console'
- `editor-ui/package.json` — contains react-resizable-panels
- Commits f506f15 and b5d190e verified in git log
- `npm run build` passed with zero TypeScript errors
