---
phase: 04-tab-bar-and-editor-state
plan: "01"
subsystem: editor-ui
tags: [tab-bar, codemirror, editor-state, undo-history, scroll-preservation, css]
dependency_graph:
  requires: []
  provides: [tab-bar-sizing, editor-state-cache]
  affects: [editor-ui/src/components/TabBar.module.css, editor-ui/src/components/Editor.tsx]
tech_stack:
  added: []
  patterns:
    - Module-level Map<scriptId, CachedTabState> for CodeMirror EditorState persistence
    - StateEffect.reconfigure for refreshing dynamic extensions on cached state restoration
    - scrollSnapshot() dispatched after view mount for scroll position restoration
key_files:
  created: []
  modified:
    - editor-ui/src/components/TabBar.module.css
    - editor-ui/src/components/Editor.tsx
decisions:
  - EditorState cache is module-level (not Zustand) — EditorState is large/non-serializable
  - StateEffect.reconfigure used to refresh linter+handleUpdate closures on cached state without losing undo history
  - scrollSnapshot() returns StateEffect<ScrollTarget>, dispatched after view construction (not passed as Extension)
  - opacity:0 + pointer-events:none for close button hiding — preserves tab width to prevent layout shift
metrics:
  duration: "3 min"
  completed: "2026-03-14"
  tasks_completed: 2
  files_modified: 2
---

# Phase 4 Plan 1: Tab Bar and Editor State Summary

**One-liner:** Rider-accurate tab bar (38px, hover-reveal close button) with per-tab EditorState cache preserving undo history and scroll position across tab switches.

## Tasks Completed

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | Rider tab bar sizing and close-button hover-reveal | 3ff8734 | TabBar.module.css |
| 2 | EditorState map for per-tab state preservation | d3477af | Editor.tsx |

## What Was Built

**Task 1 — Tab bar restyling:**
- Changed `.bar` from `min-height: 36px` to `height: 38px` with `padding: 0 16px`
- Close button hidden by default (`opacity: 0; pointer-events: none`) on all tabs
- Close button revealed on hover (`.tab:hover .closeBtn`) with smooth opacity transition
- Active tab always shows close button (`.active .closeBtn { opacity: 1; pointer-events: auto }`)
- `opacity` used (not `display: none`) to prevent layout shift when button appears

**Task 2 — EditorState preservation:**
- Added module-level `editorStates: Map<string, CachedTabState>` (outside component, survives re-renders)
- On tab switch: saves current view's `state` + `scrollSnapshot()` before destroying the view
- On tab restore: uses `StateEffect.reconfigure.of(buildExtensions(...))` to replay fresh linter/handleUpdate closures onto the cached EditorState — preserves undo history, selection, and all StateField values
- External content change detection: compares `cached.state.doc.toString()` with `activeTab.content`; creates fresh state if they differ (IPC updated the tab while inactive)
- Scroll restoration: dispatches the cached `StateEffect<ScrollTarget>` after view mount
- Tab close cleanup: `useEffect` watching `tabs` array removes orphaned cache entries
- Extracted `buildExtensions()` helper used by both fresh and restored code paths

## Deviations from Plan

**1. [Rule 1 - Bug] scrollSnapshot() returns StateEffect, not Extension**
- **Found during:** Task 2 implementation
- **Issue:** Plan described passing `scrollSnapshot` as a view-level Extension, but `EditorView.scrollSnapshot()` returns `StateEffect<ScrollTarget>` per CodeMirror type definitions — not an `Extension`
- **Fix:** Store it as `ReturnType<EditorView['scrollSnapshot']>` and dispatch it via `view.dispatch({ effects: cached.scrollSnapshot })` after view mount
- **Files modified:** Editor.tsx
- **Commit:** d3477af

**2. [Rule 1 - Bug] EditorState.reconfigure does not exist**
- **Found during:** Task 2 first TypeScript check
- **Issue:** Plan referenced `EditorState.reconfigure` but the correct API is `StateEffect.reconfigure` (on `StateEffect`, imported from `@codemirror/state`)
- **Fix:** Used `StateEffect.reconfigure.of(extensions)` in a `.update()` transaction on the cached state
- **Files modified:** Editor.tsx
- **Commit:** d3477af

## Self-Check: PASSED

- FOUND: editor-ui/src/components/TabBar.module.css
- FOUND: editor-ui/src/components/Editor.tsx
- FOUND commit 3ff8734 (Task 1)
- FOUND commit d3477af (Task 2)
