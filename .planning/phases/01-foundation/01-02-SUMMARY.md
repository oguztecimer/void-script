---
phase: 01-foundation
plan: 02
subsystem: ui
tags: [react, codemirror, css-tokens, fontsource, design-system, refactor, wry]

# Dependency graph
requires:
  - phase: 01-foundation-01
    provides: tokens.css with 59 CSS custom properties covering all design values, Fontsource font packages installed

provides:
  - "Zero hardcoded hex color values in any .tsx or .ts file outside tokens.css"
  - "All 10 source files migrated to var(--token) references from tokens.css"
  - "CodeMirror theme uses var(--font-mono) for JetBrains Mono Variable with ligatures disabled"
  - "Console and DebugPanel fontFamily uses var(--font-mono) instead of hardcoded font stack"
  - "Breakpoint gutter and debug line decoration use var(--accent-breakpoint) and var(--bg-selection)"
  - "CSS tokens inlined in index.html for guaranteed wry WebView :root resolution"
  - "Window control IPC (minimize/maximize/close) wired through Rust handler"

affects:
  - all future phases that touch any of the 10 migrated component files
  - Phase 02+ UI work: all components now use token system

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "CSS custom property token consumption: all inline React styles use var(--token-name) string form"
    - "fontFamily via CSS var(): 'fontFamily: var(--font-mono)' works in React inline styles at render time"
    - "CSS named colors (white) for fixed-contrast icon text on colored button backgrounds (avoids needing --text-white token)"
    - "Inline :root tokens in index.html <style> block as the production path for wry custom protocol — external stylesheet :root blocks are unreliable in WKWebView custom schemes"

key-files:
  created: []
  modified:
    - "editor-ui/src/codemirror/voidscript-theme.ts"
    - "editor-ui/src/App.tsx"
    - "editor-ui/src/components/Header.tsx"
    - "editor-ui/src/components/Editor.tsx"
    - "editor-ui/src/components/Console.tsx"
    - "editor-ui/src/components/DebugPanel.tsx"
    - "editor-ui/src/components/ScriptList.tsx"
    - "editor-ui/src/components/StatusBar.tsx"
    - "editor-ui/src/components/TabBar.tsx"
    - "editor-ui/src/components/ToolStrip.tsx"
    - "editor-ui/index.html"
    - "crates/voidscript-editor/src/window.rs"
    - "editor-ui/vite.config.ts"
    - "crates/voidscript-editor/src/ipc.rs"
    - "crates/voidscript-editor/src/plugin.rs"
    - "editor-ui/src/ipc/types.ts"

key-decisions:
  - "CSS var() works in React inline style strings — fontFamily: 'var(--font-mono)' resolves at browser render time"
  - "TrafficLight backgroundColor passes color prop (a var() string) directly — no need for separate token lookup"
  - "ToolStrip active button uses CSS named color 'white' for icon contrast on --accent-blue background — avoids adding a --text-white token for a single use case"
  - "ActionBtn iconColor/bgColor/hoverBg props are now var(--token) strings — callers pass token references, button renders them directly without knowing specific values"
  - "debugLineDecoration style attribute uses var(--bg-selection) — works because the attribute is injected into DOM inline styles where :root vars are resolved"
  - "CRITICAL: wry custom protocol (WKWebView) does NOT apply CSS :root blocks from external stylesheets loaded via custom scheme. Production fix: inline all :root token definitions in index.html <style> block. tokens.css retained for Vite dev server compatibility only."
  - "Access-Control-Allow-Origin: * added to wry get_asset() responses as belt-and-suspenders for cross-origin font loading"
  - "Vite crossOriginLoading: false prevents crossorigin attribute on script tags, which conflicts with wry custom protocol same-origin model"

patterns-established:
  - "Token consumption pattern: 'backgroundColor: var(--bg-panel)' — all inline styles follow this string form"
  - "Hover handler pattern: onMouseEnter sets var(--bg-hover), onMouseLeave resets to 'transparent' — consistent across all interactive elements"
  - "wry CSS token delivery: inline :root in index.html head, not via external stylesheet import"

requirements-completed: [FOUN-01, FOUN-02, FOUN-03]

# Metrics
duration: ~15min (including wry debugging)
completed: 2026-03-14
---

# Phase 1 Plan 2: CSS Token Migration Summary

**129+ hardcoded hex values replaced with var(--token) across 10 source files; CodeMirror theme and font stack fully driven by tokens.css; critical wry CSS custom property delivery bug discovered and fixed**

## Performance

- **Duration:** ~15 min (including wry custom protocol CSS debugging)
- **Started:** 2026-03-14T08:56:19Z
- **Completed:** 2026-03-14T09:15:00Z
- **Tasks:** 3 of 3 complete
- **Files modified:** 16

## Accomplishments

- Migrated voidscript-theme.ts: removed 7 top-level color constants, replaced all hex values in EditorView.theme() and HighlightStyle.define() with CSS custom property tokens, updated font-family to var(--font-mono), added fontVariantLigatures: 'none'
- Migrated all 9 component/app files (App.tsx, Header.tsx, Editor.tsx, Console.tsx, DebugPanel.tsx, ScriptList.tsx, StatusBar.tsx, TabBar.tsx, ToolStrip.tsx): zero hex values remain, all colors reference var(--token-name) from tokens.css
- Vite build succeeds (687ms), all woff2 font assets bundled correctly
- Discovered and fixed critical wry/WKWebView CSS custom property delivery issue: inlined all :root tokens in index.html
- Added CORS header to wry asset responses and disabled Vite crossOriginLoading
- Wired window minimize/maximize/close IPC through Rust WindowControlEvent handler

## Task Commits

Each task was committed atomically:

1. **Task 1: Migrate voidscript-theme.ts to CSS tokens and Fontsource fonts** — `b6a26c8` (refactor)
2. **Task 2: Migrate all component files and App.tsx to CSS tokens** — `061ce83` (refactor)
3. **Task 3: wry CSS fix and verification** — `7d7b34e` (fix)

## Files Created/Modified

- `editor-ui/src/codemirror/voidscript-theme.ts` — EditorView theme and HighlightStyle now fully token-driven; var(--font-mono), fontVariantLigatures: 'none'
- `editor-ui/src/App.tsx` — var(--bg-app), var(--bg-panel), var(--border-*), var(--text-*), var(--accent-blue)
- `editor-ui/src/components/Header.tsx` — var(--bg-toolbar), var(--icon-run/debug/stop), var(--bg-btn-*), var(--traffic-close/minimize/maximize), all separator/widget/button colors tokenized
- `editor-ui/src/components/Editor.tsx` — var(--accent-breakpoint) for gutter marker; var(--bg-selection) for debug line decoration; var(--text-tertiary) for empty state
- `editor-ui/src/components/Console.tsx` — var(--font-mono), var(--bg-editor), var(--accent-red/yellow), var(--text-primary/disabled)
- `editor-ui/src/components/DebugPanel.tsx` — var(--bg-panel), var(--bg-selection), var(--font-mono), all text/border tokens
- `editor-ui/src/components/ScriptList.tsx` — var(--bg-panel), var(--border-*), var(--bg-hover), var(--text-*)
- `editor-ui/src/components/StatusBar.tsx` — var(--bg-panel), var(--accent-red/yellow/green), var(--text-secondary/primary)
- `editor-ui/src/components/TabBar.tsx` — var(--bg-tab-inactive/active), var(--accent-blue), var(--text-primary/secondary/tertiary)
- `editor-ui/src/components/ToolStrip.tsx` — var(--bg-panel), var(--accent-blue), var(--bg-hover); CSS named color 'white' for active icon
- `editor-ui/index.html` — all :root token definitions inlined in <style> block (production wry fix); tokens.css import retained for dev server
- `crates/voidscript-editor/src/window.rs` — Access-Control-Allow-Origin: * header added to all asset responses
- `editor-ui/vite.config.ts` — crossOriginLoading: false to prevent crossorigin attribute conflicts
- `crates/voidscript-editor/src/ipc.rs` — WindowControlEvent enum, WindowMinimize/Maximize/Close IPC variants
- `crates/voidscript-editor/src/plugin.rs` — WindowControlEvent registered, handle_window_controls system added
- `editor-ui/src/ipc/types.ts` — window_minimize/maximize/close message types added

## Decisions Made

- `fontFamily: 'var(--font-mono)'` works in React inline styles — the browser resolves the CSS variable at render time, not at style-object creation time.
- ActionBtn component already accepted iconColor/bgColor/hoverBg as props — simply switched the caller-provided hex literals to var(--token) strings. No component restructuring needed.
- CSS named color `white` used for active ToolStrip button icon (on --accent-blue background) to avoid adding a single-use --text-white token.
- tokens.css already contained all needed component-specific tokens from Plan 01 — no new tokens needed to be added.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] wry custom protocol does not apply external stylesheet :root blocks**

- **Found during:** Task 3 human-verify checkpoint
- **Issue:** After deploying to the wry WebView via custom protocol, CSS colors were all transparent/missing. Investigation revealed that WKWebView (macOS) does not apply `:root` custom property blocks from stylesheets loaded via custom schemes (`voidscript://`). The tokens.css `:root {}` block was silently ignored, leaving all `var(--token-name)` references unresolved.
- **Fix:** Inlined all `:root` token definitions directly into index.html's `<style>` block. This runs before any JS or external CSS, guaranteeing tokens are available to all components. The tokens.css file is retained for Vite dev server compatibility where the external stylesheet path works correctly.
- **Additional fixes:** Added `Access-Control-Allow-Origin: *` to wry asset responses; disabled Vite `crossOriginLoading` to prevent `crossorigin` attribute interference with the custom protocol.
- **Files modified:** `editor-ui/index.html`, `crates/voidscript-editor/src/window.rs`, `editor-ui/vite.config.ts`
- **Commit:** `7d7b34e`

**2. [Rule 2 - Missing functionality] Window control IPC not wired to Rust handler**

- **Found during:** Task 3 — window minimize/maximize/close buttons in Header.tsx had no Rust-side handler
- **Fix:** Added `WindowControlEvent` enum to ipc.rs, registered event in plugin.rs, added message variants to ipc/types.ts. The existing `handle_window_controls` system in window.rs was already implemented but not connected.
- **Files modified:** `crates/voidscript-editor/src/ipc.rs`, `crates/voidscript-editor/src/plugin.rs`, `editor-ui/src/ipc/types.ts`
- **Commit:** `7d7b34e`

## Issues Encountered

The wry CSS custom property issue is a non-obvious platform behavior. Key insight for all future phases: **any CSS that must be available before or during initial JS bundle parse must be inlined in index.html, not loaded via external stylesheet through the wry custom protocol.** This applies to :root tokens, critical animations, and any reset/base styles.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Phase 1 Foundation is fully complete
- All subsequent phases can reference var(--token-name) throughout their component work
- The token system is complete and battle-tested in the actual wry WebView
- Critical delivery pattern established: index.html inline styles are the production path for CSS tokens in wry

## Self-Check: PASSED

- All 10 source files exist and contain zero hardcoded hex values
- Commit b6a26c8 (Task 1) confirmed in git log
- Commit 061ce83 (Task 2) confirmed in git log
- Commit 7d7b34e (Task 3 wry fix) confirmed in git log
- SUMMARY.md created at .planning/phases/01-foundation/01-02-SUMMARY.md

---
*Phase: 01-foundation*
*Completed: 2026-03-14*
