---
phase: 01-foundation
verified: 2026-03-14T10:00:00Z
status: human_needed
score: 5/5 automated truths verified
human_verification:
  - test: "Open the built app via cargo run, open DevTools, inspect any UI text element. In the Computed tab, verify font-family resolves to 'Inter Variable' — not a system fallback like -apple-system or Helvetica."
    expected: "font-family shows 'Inter Variable' as the resolved family"
    why_human: "Font rendering in the wry WKWebView custom protocol cannot be verified programmatically — the critical delivery path (tokens inlined in index.html, CORS header on asset responses) is wired correctly, but actual font resolution in the macOS WebView requires visual DevTools inspection."
  - test: "In the same running app, click inside the CodeMirror editor and inspect a .cm-content element. Verify font-family resolves to 'JetBrains Mono Variable' — not Fira Code or a system monospace."
    expected: "font-family shows 'JetBrains Mono Variable'"
    why_human: "Same wry WebView rendering reason as above. The var(--font-mono) token is wired correctly in voidscript-theme.ts but actual resolution requires running app inspection."
  - test: "Close the app and relaunch (cargo run). Watch for any white flash between process launch and first frame render."
    expected: "Window background is dark (#1E1F22) from the moment the window appears — no white flash"
    why_human: "Cold-launch flash is a rendering artifact that cannot be inspected statically. The html background guard is in index.html but real-world verification requires visual observation."
  - test: "In the running app, compare UI text weight against JetBrains Rider New UI on the same macOS machine. Text should appear crisp and Regular weight — not blurry or artificially bold."
    expected: "Text looks crisp and weight-correct, matching Rider's Regular weight appearance"
    why_human: "Font smoothing quality (-webkit-font-smoothing: antialiased) is a subjective visual assessment that requires comparing two running applications."
---

# Phase 1: Foundation Verification Report

**Phase Goal:** Typography is correctly loaded, all design values live in a single token file, and macOS rendering matches Rider
**Verified:** 2026-03-14
**Status:** human_needed — all automated checks pass; 4 items require running-app inspection
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|---------|
| 1 | Fontsource Inter and JetBrains Mono packages installed and font CSS importable | VERIFIED | Both `@fontsource-variable/inter` and `@fontsource-variable/jetbrains-mono` present in `package.json` dependencies; woff2 files exist in node_modules; Vite build bundles 7+ woff2 files into dist/ |
| 2 | Font CSS imported at app entry point before React render | VERIFIED | `main.tsx` lines 1-3: imports `@fontsource-variable/inter`, `@fontsource-variable/jetbrains-mono`, `./theme/tokens.css` — in that order, before `createRoot` |
| 3 | A single tokens.css file exists with 59 CSS custom properties covering every design value category | VERIFIED | `editor-ui/src/theme/tokens.css` exists, 59 `--` properties on `:root` covering: backgrounds (11+), action-button backgrounds (6), text (5), icon colors (3), traffic lights (3), borders (3), accents (5), syntax (9), dimensions (6), typography (6), transitions (1) |
| 4 | Zero hardcoded hex color values remain in any component .tsx/.ts file (except intentional anti-flash guard) | VERIFIED | `grep -rE '#[0-9A-Fa-f]{6}' editor-ui/src/ --include='*.tsx' --include='*.ts'` returns no matches. One `rgba(0,0,0,0.6)` in Header.tsx (TrafficLight hover symbol contrast) and one named color `white` in ToolStrip.tsx (active icon on --accent-blue background) — both are intentional design choices documented in 01-02-SUMMARY.md as deliberate non-token usage |
| 5 | macOS rendering properties applied at root and anti-flash guard in index.html | VERIFIED | `index.html` contains: `html { background: #1E1F22; color-scheme: dark; }` and `body { -webkit-font-smoothing: antialiased; -moz-osx-font-smoothing: grayscale; text-rendering: optimizeLegibility; }`. All :root tokens are also inlined in index.html `<style>` block as a wry/WKWebView production fix |

**Score:** 5/5 truths verified (automated)

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `editor-ui/src/theme/tokens.css` | All design tokens as CSS custom properties; min 60 lines | VERIFIED | 217 lines, 59 custom properties on `:root`, all 8 required token categories present |
| `editor-ui/src/main.tsx` | Font and token imports at app entry | VERIFIED | Imports `@fontsource-variable/inter`, `@fontsource-variable/jetbrains-mono`, `./theme/tokens.css` — all three present before `createRoot` |
| `editor-ui/index.html` | macOS rendering fixes and dark-mode anti-flash | VERIFIED | `color-scheme: dark`, `background: #1E1F22`, `-webkit-font-smoothing: antialiased`, all tokens inlined in `<style>` |
| `crates/voidscript-editor/src/embedded_assets.rs` | Asset serving with correct MIME types including font/woff2 | VERIFIED | Uses `mime_guess::from_path(path).first_or_octet_stream()`; unit test `woff2_mime_type_is_correct` passes (confirmed by `cargo test` run) |
| `editor-ui/src/codemirror/voidscript-theme.ts` | CodeMirror theme using CSS tokens for all colors and Fontsource font-family | VERIFIED | 35 `var(--)` references; `fontFamily: 'var(--font-mono)'`; `fontVariantLigatures: 'none'`; zero hardcoded hex values |
| `editor-ui/src/App.tsx` | Root layout using CSS tokens | VERIFIED | 9 `var(--)` references; zero hex values |
| `editor-ui/src/components/Header.tsx` | Title bar using CSS tokens (32 hex values migrated) | VERIFIED | 32 `var(--)` references; zero hex values; `rgba(0,0,0,0.6)` for TrafficLight hover symbol is intentional (no opaque token equivalent exists) |
| `editor-ui/src/components/TabBar.tsx` | Tab bar using CSS tokens | VERIFIED | 11 `var(--)` references; zero hex values |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `editor-ui/src/main.tsx` | `editor-ui/src/theme/tokens.css` | CSS import | WIRED | Line 3: `import './theme/tokens.css'` — confirmed present |
| `editor-ui/src/main.tsx` | `node_modules/@fontsource-variable` | Fontsource imports | WIRED | Lines 1-2: both `@fontsource-variable/inter` and `@fontsource-variable/jetbrains-mono` imported |
| `crates/voidscript-editor/src/embedded_assets.rs` | `mime_guess` crate | MIME type resolution | WIRED | Line 15: `mime_guess::from_path(path).first_or_octet_stream()`; test passes |
| `editor-ui/src/codemirror/voidscript-theme.ts` | `editor-ui/src/theme/tokens.css` | CSS custom properties inherited from :root | WIRED | 35 `var(--)` references; tokens available because index.html inlines :root block before JS runs |
| `editor-ui/src/components/Header.tsx` | `editor-ui/src/theme/tokens.css` | CSS custom properties in inline styles | WIRED | 32 `var(--)` references confirmed |
| `editor-ui/src/App.tsx` | `editor-ui/src/theme/tokens.css` | CSS custom properties in inline styles | WIRED | 9 `var(--)` references confirmed |
| `editor-ui/index.html` (:root inline) | All components | wry WebView CSS delivery | WIRED | Critical fix: tokens are inlined in `<style>` block because WKWebView does not apply `:root` blocks from externally-loaded stylesheets via custom scheme. tokens.css retained for Vite dev server only. |

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|---------|
| FOUN-01 | 01-01, 01-02 | Inter font self-hosted via Fontsource applied universally | SATISFIED | `@fontsource-variable/inter` installed; imported in main.tsx; woff2 files in dist/; `--font-ui` token defined and applied in index.html body font-family | Needs human: visual confirmation in wry WebView |
| FOUN-02 | 01-01, 01-02 | JetBrains Mono font self-hosted via Fontsource for editor/console | SATISFIED | `@fontsource-variable/jetbrains-mono` installed; imported in main.tsx; `--font-mono` token applied in voidscript-theme.ts and Console.tsx | Needs human: visual confirmation in wry WebView |
| FOUN-03 | 01-01, 01-02 | CSS custom properties token system (tokens.css) replacing all hardcoded hex | SATISFIED | tokens.css exists with 59 properties; zero hex values in any .tsx/.ts file outside tokens.css; all 10 component files use var(--token) exclusively |
| FOUN-05 | 01-01 | macOS font smoothing fix applied (-webkit-font-smoothing: antialiased, color-scheme: dark) | SATISFIED | index.html body has -webkit-font-smoothing: antialiased, -moz-osx-font-smoothing: grayscale, text-rendering: optimizeLegibility; html has color-scheme: dark | Needs human: visual quality assessment |

**Orphaned requirements check:** REQUIREMENTS.md traceability table maps FOUN-01, FOUN-02, FOUN-03, FOUN-05 to Phase 1. FOUN-04 (CSS Modules migration) is mapped to Phase 2 — correctly not claimed by any Phase 1 plan. No orphaned requirements found.

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `editor-ui/src/components/Header.tsx` | 209 | `rgba(0,0,0,0.6)` — non-token color value | INFO | TrafficLight hover symbol contrast color. Intentional design choice: semi-transparent black on a variable-color background cannot be expressed as a :root token. Documented in 01-02-SUMMARY.md. Does not violate FOUN-03 intent. |
| `editor-ui/src/components/ToolStrip.tsx` | 45 | `'white'` — CSS named color, not a token | INFO | Active ToolStrip button icon color on `--accent-blue` background. Intentional: avoids adding a single-use `--text-white` token. Documented in 01-02-SUMMARY.md. Does not violate FOUN-03 intent. |
| `editor-ui/src/theme/tokens.css` | 44 | `--bg-tab-active: #2B2D30` — diverges from plan intent | WARNING | Plan 01-01 specified `--bg-tab-active: #1E1F22` ("same as editor for seamless visual merge"). Actual value is `#2B2D30` (panel color). TabBar uses this token correctly. The value change was made during implementation without documentation in either SUMMARY. This is a design value to confirm against Rider — does not block foundation goal. |

---

### Human Verification Required

#### 1. Inter Variable font renders in wry WebView

**Test:** Build and launch the app (`cargo run` from project root). Open DevTools, select any UI text element (e.g., a tab label, status bar text, or toolbar widget). In the Computed tab, check the resolved `font-family` value.
**Expected:** Font resolves to `Inter Variable` — not `-apple-system`, `BlinkMacSystemFont`, `Helvetica`, or other system fallbacks
**Why human:** Font resolution in the macOS WKWebView custom protocol requires a running app with DevTools inspection. The infrastructure (imports, woff2 files bundled, tokens inlined in index.html) is fully wired, but the critical wry-specific CSS delivery fix (inlining :root in index.html) requires human confirmation that fonts actually load in production.

#### 2. JetBrains Mono Variable renders in editor and console

**Test:** In the running app, click inside the CodeMirror editor. Open DevTools and inspect a `.cm-content` element. Check the resolved `font-family` in the Computed tab.
**Expected:** Font resolves to `JetBrains Mono Variable` — not `Fira Code`, `Menlo`, `monospace`, or other fallbacks. Ligatures should be disabled (e.g., `->` and `!=` should not merge into single glyphs).
**Why human:** Same wry WebView reasoning. Additionally, `fontVariantLigatures: 'none'` is wired in theme but ligature behavior requires visual confirmation.

#### 3. No white flash on cold launch

**Test:** Fully quit the app if running. Launch fresh (`cargo run`). Observe the window from the moment it appears.
**Expected:** The window background is dark (`#1E1F22`) from first paint — no white or grey flash before content loads.
**Why human:** Cold-launch flash is a timing artifact between WebView initialization and first paint. The anti-flash guard (`html { background: #1E1F22 }` in index.html) is present but real-world effectiveness depends on macOS WKWebView rendering order.

#### 4. macOS font smoothing matches Rider Regular weight visually

**Test:** Open both the VOID//SCRIPT editor and JetBrains Rider side by side on the same macOS display. Compare text at equivalent sizes (13px UI text, 14px editor text).
**Expected:** Text in the VOID//SCRIPT editor appears equally crisp and Regular-weight as Rider — no blurry, fuzzy, or artificially bold rendering.
**Why human:** Font smoothing quality is a subjective visual comparison that requires two running applications and a human judge. `-webkit-font-smoothing: antialiased` is applied correctly in index.html but rendering quality on specific macOS/display combinations cannot be verified programmatically.

---

### Notes on Token Value Discrepancy

The `--bg-tab-active` token is defined as `#2B2D30` in `tokens.css` but Plan 01-01 specified `#1E1F22` with the comment "same as editor for seamless look." The actual Rider New UI has active tabs at the same color as the editor area — this may be a visual bug introduced during implementation. It does not block the Phase 1 foundation goal (which is about infrastructure, not visual fidelity) but should be confirmed when visual comparison against Rider is done in the human verification step.

---

### Gaps Summary

No automated gaps. All 5 observable truths verified. All 8 artifacts exist, are substantive, and are wired. All 4 requirements (FOUN-01, FOUN-02, FOUN-03, FOUN-05) have implementation evidence.

4 items require human verification in the running wry WebView application — these cannot be assessed statically because they depend on the macOS WKWebView custom protocol rendering behavior that required a non-obvious fix (inlining :root tokens in index.html) during implementation.

---

_Verified: 2026-03-14_
_Verifier: Claude (gsd-verifier)_
