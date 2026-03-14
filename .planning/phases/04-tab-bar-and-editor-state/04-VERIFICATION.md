---
phase: 04-tab-bar-and-editor-state
verified: 2026-03-14T00:00:00Z
status: human_needed
score: 5/5 must-haves verified
human_verification:
  - test: "Hover over an inactive tab and confirm the close button fades in, then fades out when the mouse leaves"
    expected: "Close button appears smoothly on hover (opacity transition), disappears when mouse leaves, with no layout shift"
    why_human: "CSS :hover interaction and visual transition cannot be verified by static analysis"
  - test: "Confirm the active tab always shows its close button without hovering"
    expected: "Active tab's close button is visible at rest"
    why_human: "Active-state CSS application requires a live browser"
  - test: "Open a script, type several characters, switch to another tab, switch back, press Cmd+Z repeatedly"
    expected: "All typed characters undo correctly — undo history is not lost across tab switches"
    why_human: "EditorState cache correctness (undo history preservation) requires live CodeMirror interaction"
  - test: "Open a long script, scroll to the bottom, switch to another tab, switch back"
    expected: "Scroll position is restored to where you left it"
    why_human: "Scroll restoration via dispatched StateEffect requires a mounted DOM and live scroll position"
---

# Phase 4: Tab Bar and Editor State Verification Report

**Phase Goal:** The tab bar matches Rider's height and spacing; tab close buttons behave correctly; editor state survives tab switching
**Verified:** 2026-03-14
**Status:** human_needed — all automated checks passed; 4 visual/behavioral items require live browser testing
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Tab bar is 38px tall with 0 16px padding | VERIFIED | `TabBar.module.css` line 5: `height: 38px`; line 6: `padding: 0 16px` |
| 2 | Close button on inactive tabs is hidden until hover | VERIFIED | `.closeBtn` has `opacity: 0; pointer-events: none`; `.tab:hover .closeBtn` sets `opacity: 1; pointer-events: auto` |
| 3 | Active tab always shows its close button | VERIFIED | `.active .closeBtn { opacity: 1; pointer-events: auto }` — CSS rule present |
| 4 | Switching tabs preserves undo history (Cmd+Z works after switching back) | VERIFIED (code path) | `editorStates` Map saves `viewRef.current.state` before destroying; `StateEffect.reconfigure` replays closures on restore; human test required for runtime confirmation |
| 5 | Switching tabs preserves scroll position | VERIFIED (code path) | `scrollSnapshot()` stored, dispatched via `view.dispatch({ effects: cached.scrollSnapshot })` after mount; human test required for runtime confirmation |

**Score:** 5/5 truths verified by static analysis (4 require human runtime confirmation)

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `editor-ui/src/components/TabBar.module.css` | Rider-accurate tab bar sizing and close button visibility | VERIFIED | 78 lines; `height: 38px`, `padding: 0 16px`, all three close-button CSS rules present |
| `editor-ui/src/components/TabBar.tsx` | Tab rendering with hover-reveal close button | VERIFIED | 42 lines; full rendering implementation, `switchTab` and `closeTab` wired |
| `editor-ui/src/components/Editor.tsx` | EditorState map for per-tab state preservation | VERIFIED | 302 lines; `editorStates` Map declared at module level, used in 5 locations |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `TabBar.tsx` | `editor-ui/src/state/store.ts` | `useStore switchTab` | WIRED | Line 20: `useStore.getState().switchTab(tab.scriptId)`; line 32: `useStore.getState().closeTab(tab.scriptId)` |
| `Editor.tsx` | `@codemirror/state` | `editorStates` Map keyed by scriptId | WIRED | `EditorState` imported line 3; `editorStates` Map declared line 23; `.set()` line 196, `.get()` line 216, `.delete()` line 271 |

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| TABS-01 | 04-01-PLAN.md | Tab bar height 38px with padding 0 16px matching Rider spacing | SATISFIED | `TabBar.module.css` lines 5-6: `height: 38px; padding: 0 16px` |
| TABS-02 | 04-01-PLAN.md | Close button hidden on inactive tabs, appearing on hover only; always visible on active tab | SATISFIED | `.closeBtn` opacity:0 default; `.tab:hover .closeBtn` and `.active .closeBtn` rules restore opacity:1 |

No orphaned requirements — REQUIREMENTS.md maps only TABS-01 and TABS-02 to Phase 4, both claimed and satisfied by 04-01-PLAN.md.

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `TabBar.tsx` | 9 | `return null` | Info | Legitimate empty-state guard (no tabs open) — not a stub |

No blockers. No FIXME/TODO/placeholder comments. No stub implementations.

---

### TypeScript Compilation

`npx tsc --noEmit` completed with no output — zero errors.

Commits referenced in SUMMARY exist and are verified:
- `3ff8734` — feat(04-01): Rider tab bar sizing and close-button hover-reveal
- `d3477af` — feat(04-01): EditorState map for per-tab state preservation

---

### Human Verification Required

#### 1. Close button hover reveal

**Test:** Open the app, hover the mouse over an inactive tab.
**Expected:** Close button fades in smoothly (opacity transition). Moving the mouse away fades it out. No layout shift — the tab does not change width.
**Why human:** CSS `:hover` pseudo-class behavior and visual transitions cannot be verified by static analysis.

#### 2. Active tab close button always visible

**Test:** Open the app with at least two tabs. Observe the active tab at rest without hovering.
**Expected:** The active tab's close button is visible without any hover interaction.
**Why human:** Active CSS class application and visual rendering require a live browser.

#### 3. Undo history preservation across tab switches

**Test:** Open script A, type at least 10 characters. Switch to script B. Switch back to script A. Press Cmd+Z repeatedly.
**Expected:** All 10 typed characters undo correctly — full undo history is preserved.
**Why human:** EditorState cache correctness requires live CodeMirror runtime to confirm `StateEffect.reconfigure` correctly preserves undo history without resetting it.

#### 4. Scroll position restoration across tab switches

**Test:** Open a long script, scroll to the bottom. Switch to another tab. Switch back.
**Expected:** Scroll position is restored to the bottom where you left it.
**Why human:** `scrollSnapshot()` / dispatch behavior requires a mounted DOM with actual scroll state.

---

### Implementation Notes

One notable deviation from the plan was correctly handled by the executor:

- `EditorView.scrollSnapshot()` returns `StateEffect<ScrollTarget>`, not an `Extension`. The plan incorrectly described it as an Extension to be passed to the view constructor. The implementation correctly stores it and dispatches it via `view.dispatch({ effects: cached.scrollSnapshot })` after mount. This is verified correct per CodeMirror types.

- `StateEffect.reconfigure` (not `EditorState.reconfigure`) is used correctly to replay fresh linter and `handleUpdate` closures onto the cached state without destroying undo history.

---

_Verified: 2026-03-14_
_Verifier: Claude (gsd-verifier)_
