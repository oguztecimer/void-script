---
phase: 05-tool-strips-and-panels
plan: "01"
subsystem: editor-ui
tags: [tool-strip, svg-icons, panel-headers, css-modules]
dependency_graph:
  requires: []
  provides: [SVG icon tool strip, active edge indicator, ScriptList add-script button, DebugPanel top-level header]
  affects: [editor-ui/src/components/ToolStrip.tsx, editor-ui/src/components/ToolStrip.module.css, editor-ui/src/App.tsx, editor-ui/src/components/ScriptList.tsx, editor-ui/src/components/DebugPanel.tsx, editor-ui/src/ipc/types.ts]
tech_stack:
  added: []
  patterns: [CSS Modules compound selector for strip-context active state, ReactNode icons via className prop instead of active prop]
key_files:
  created: []
  modified:
    - editor-ui/src/components/ToolStrip.tsx
    - editor-ui/src/components/ToolStrip.module.css
    - editor-ui/src/App.tsx
    - editor-ui/src/components/ScriptList.tsx
    - editor-ui/src/components/DebugPanel.tsx
    - editor-ui/src/ipc/types.ts
decisions:
  - className prop (not active prop) used for strip active state — avoids ToolBtn's solid blue fill; compound CSS selectors .left .activeBtn / .right .activeBtn place the inset shadow on the correct inner edge per side
  - create_script added to JsToRustMessage union — harmless type stub that sets up the IPC contract for future backend implementation
metrics:
  duration: "1 min"
  completed_date: "2026-03-14"
  tasks_completed: 2
  files_modified: 6
---

# Phase 05 Plan 01: Tool Strips and Panel Headers Summary

**One-liner:** Monochrome SVG icons with inset blue edge active indicator replacing emoji text; ScriptList gains add-script button, DebugPanel gains top-level header.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | SVG icons and active edge indicator for ToolStrip | 67e377f | ToolStrip.tsx, ToolStrip.module.css, App.tsx |
| 2 | Panel header improvements for ScriptList and DebugPanel | bfea1d4 | ScriptList.tsx, DebugPanel.tsx, ipc/types.ts |

## What Was Built

### Task 1: SVG icons and active edge indicator

- `ToolStrip.tsx`: exported `ToolStripItem` interface; changed `icon` field from `string` to `React.ReactNode`; removed `active` prop from `ToolBtn`, replaced with `className={isActive ? styles.activeBtn : styles.stripBtn}`
- `ToolStrip.module.css`: added `.stripBtn` (tertiary color, brightens on hover) and `.activeBtn` (primary color) plus compound selectors `.left .activeBtn` / `.right .activeBtn` for the 2px inset blue edge bar
- `App.tsx`: imported `ToolStripItem` type; replaced `'S'`/`'D'` string icons with inline SVG — document icon for Scripts, bug icon for Debug; both use `currentColor` so they inherit CSS color transitions

### Task 2: Panel header improvements

- `ipc/types.ts`: added `{ type: 'create_script' }` to `JsToRustMessage` union
- `ScriptList.tsx`: PanelHeader actions now has a plus SVG button (`Add Script`, calls `sendToRust({ type: 'create_script' })`) followed by the existing close button
- `DebugPanel.tsx`: new top-level `PanelHeader title="Debug"` with close button appears above the Frames sub-section; Frames and Variables headers are now action-less, matching Rider's panel structure

## Decisions Made

1. **className over active prop for strip active state** — `ToolBtn`'s `active` prop triggers `background-color: var(--accent-blue)` (solid fill). For the tool strip we need only a 2px edge bar. Using `className` to inject `.activeBtn` bypasses that. CSS compound selectors (`.left .activeBtn`, `.right .activeBtn`) place the box-shadow on the correct inner edge per side without any JS.

2. **create_script added to IPC union immediately** — Even though the Rust backend doesn't handle it yet, adding the type to the union is harmless and avoids a TypeScript cast workaround. The contract is established; backend implementation follows later.

## Deviations from Plan

None — plan executed exactly as written.

## Verification

- `npm run build` passes with zero TypeScript errors after both tasks
- CSS warnings in build output are pre-existing comment syntax issues in tokens.css, unrelated to this plan

## Self-Check: PASSED

- All modified files exist on disk
- Commits 67e377f and bfea1d4 verified in git log
- `npm run build` passed with zero TypeScript errors
