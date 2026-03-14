---
phase: 03-title-bar
verified: 2026-03-14T15:00:00Z
status: human_needed
score: 7/7 must-haves verified
re_verification: false
human_verification:
  - test: "Open DevTools, select any toolbar button (hamburger, back, forward, run, debug, settings), verify computed height = 26px"
    expected: "All toolbar icon buttons report 26px computed height in browser DevTools"
    why_human: "CSS custom property var(--height-widget-btn) resolves at runtime; static analysis confirms the token chain is correct but cannot measure computed pixel output"
  - test: "Drag the app window, then hover a left-side button (hamburger/back/forward) — verify hover state appears and does not get stuck"
    expected: "No stuck :hover state after window drag; hover transitions work normally after releasing the drag"
    why_human: "WKWebView CSS :hover desync after drag is a runtime-only behavior; the correct pattern (JS hover for drag-zone widgets, CSS hover for no-drag zone) is verified in code but real-world correctness requires running the app"
  - test: "Run a script, confirm no 'Running...' / 'Debugging...' / 'Paused' text appears anywhere in the toolbar"
    expected: "State communicated through visible buttons only; no status text label"
    why_human: "Status text absence is verified statically, but dynamic state rendering requires confirming no other code path injects status text at runtime"
---

# Phase 3: Title Bar Verification Report

**Phase Goal:** The header is a pixel-accurate Rider New UI title bar with all required toolbar widgets
**Verified:** 2026-03-14T15:00:00Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Every toolbar button renders at 26px computed height | ? HUMAN | `ToolBtn.module.css` `.small { width: var(--height-widget-btn); height: var(--height-widget-btn); }` — token resolves to `26px` per `index.html :root`. `.widget`, `.runConfig`, `.searchPill` all use `height: var(--height-widget-btn)`. Chain is correct; runtime measurement needed. |
| 2 | Search Everywhere pill visible with magnifying glass icon, "Search" text, and shift-shift hint | VERIFIED | `SearchPill()` at line 248: `<button className={styles.searchPill}>` with SVG circle/line magnifying glass, `<span>Search</span>`, `<span className={styles.searchShortcut}>&#8679;&#8679;</span>`. CSS `.searchPill` is 220px wide, `height: var(--height-widget-btn)`, with border and `:hover` transition. |
| 3 | Settings gear icon is the rightmost element in the toolbar | VERIFIED | Line 145: `<ToolBtn size="small" title="Settings">` with gear SVG is the last child inside `.rightGroup` div (line 151 closes the div immediately after). `SearchPill` precedes it at line 144. |
| 4 | Separators appear only between widget groups, not within groups | VERIFIED | Lines 44-56: project `HeaderWidget` followed directly by VCS `HeaderWidget` with no `<Separator>` between them (comment at line 51 explicitly confirms: "no separator between project and VCS"). Separators present at group boundaries: after traffic lights, after hamburger, after back/forward, between run-config and action buttons, between action buttons and search pill. |
| 5 | Hovering any toolbar button shows a state change without desync after window drag | ? HUMAN | Code pattern is correct: drag-zone `HeaderWidget` uses JS hover (`onMouseEnter`/`onMouseLeave`), traffic lights and right-group elements use CSS `:hover`. Runtime verification required. |
| 6 | No local ToolBtn, ActionBtn, or Separator definitions remain in Header.tsx | VERIFIED | `grep -n "function ToolBtn\|function ActionBtn\|function Separator"` returns exit 1 (no matches). File imports `ToolBtn` from `../primitives/ToolBtn` (line 4) and `Separator` from `../primitives/Separator` (line 5). Both are used (ToolBtn: 23 references; Separator: 7 references). |
| 7 | No status text (Running.../Debugging.../Paused) appears in the toolbar | VERIFIED | `grep "Running\.\.\.\|Debugging\.\.\.\|\"Paused\""` returns exit 1 (no matches). State is communicated via visible buttons (Stop replaces Run/Debug; separator + stepping buttons appear when debugging and paused). |

**Score:** 5/7 truths fully verified; 2/7 require human confirmation (runtime behavior)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `editor-ui/src/components/Header.module.css` | All header layout and widget styles — must contain `.toolbar` | VERIFIED | Exists, 146 lines. Contains: `.toolbar`, `.spacer`, `.rightGroup`, `.trafficLights`, `.trafficLight`, `.trafficLight:hover`, `.widget`, `.widgetMuted`, `.widgetIcon`, `.widgetChevron`, `.runConfig`, `.runConfig:hover`, `.runConfigIcon`, `.runConfigChevron`, `.searchPill`, `.searchPill:hover`, `.searchShortcut`. |
| `editor-ui/src/components/Header.tsx` | Restructured header with primitives and new widgets — min 150 lines | VERIFIED | Exists, 259 lines (well over minimum). Imports from primitives, contains `SearchPill`, `WindowControls`, `TrafficLight`, `HeaderWidget`, `RunConfigSelector` local sub-components, and the main `Header` export. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `Header.tsx` | `src/primitives/ToolBtn.tsx` | `import { ToolBtn } from '../primitives/ToolBtn'` | WIRED | Line 4 confirms import; ToolBtn used 23 times in file. Pattern `import.*ToolBtn.*from.*primitives` confirmed. |
| `Header.tsx` | `src/primitives/Separator.tsx` | `import { Separator } from '../primitives/Separator'` | WIRED | Line 5 confirms import; Separator used 7 times in file. Pattern `import.*Separator.*from.*primitives` confirmed. |
| `Header.module.css` | `editor-ui/index.html` | CSS custom property references `var(--height-widget-btn)` | WIRED | Token appears 3 times in Header.module.css (lines 55, 95, 124 — `.widget`, `.runConfig`, `.searchPill`). Token defined in `index.html :root` as `26px`. `ToolBtn.module.css` `.small` rule also uses `var(--height-widget-btn)` — full chain verified. |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| TBAR-01 | 03-01-PLAN.md | Widget buttons sized to 26px height matching Rider New UI proportions | SATISFIED | `.small { height: var(--height-widget-btn) }` in ToolBtn.module.css; `.widget`, `.runConfig`, `.searchPill` all use `height: var(--height-widget-btn)`. Token = 26px in index.html. Runtime confirmation is human item #1. |
| TBAR-02 | 03-01-PLAN.md | Correct spacing, separator positions, and font weights across all toolbar widgets | SATISFIED | No separator between project and VCS widgets. Separators only between groups. `.widget` has `font-weight: 600`, `.widgetMuted` has `font-weight: 400`. Gap and padding values use CSS tokens throughout. |
| TBAR-03 | 03-01-PLAN.md | Search Everywhere magnifying glass icon button in toolbar center-right area | SATISFIED | `SearchPill()` renders with `<circle>` + `<line>` SVG magnifying glass, "Search" span, and `&#8679;&#8679;` shortcut hint. Positioned inside `.rightGroup` after action buttons. CSS `.searchPill` provides 220px width, 26px height, border, and hover transition. |
| TBAR-04 | 03-01-PLAN.md | Settings gear icon at toolbar far-right position | SATISFIED | `<ToolBtn size="small" title="Settings">` with gear path SVG is the last element inside `.rightGroup`. No siblings follow it before the closing div. |

No orphaned requirements found — all four Phase 3 TBAR requirements are covered by 03-01-PLAN.md, and REQUIREMENTS.md marks all four as Complete.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `Header.tsx` | 194 | `style={{ backgroundColor: color }}` on `TrafficLight` | Info | Legitimate per-instance dynamic color (three different traffic light colors). Plan explicitly allowed this; static equivalent is impossible. Not a stub or blocker. |

No TODO/FIXME/PLACEHOLDER markers found. No status text. No empty `return null` or `return {}`. No local copies of project primitives.

**Build status:** `npm run build` passes with zero TypeScript errors. Two CSS syntax warnings from a comment block in a separate CSS file — non-fatal, unrelated to phase 3 work.

### Human Verification Required

#### 1. 26px Computed Height (TBAR-01)

**Test:** Open DevTools in the running app, click any toolbar button element, view Computed > height
**Expected:** 26px for all icon buttons (hamburger, back, forward, run, debug, settings) and compound widgets (run config selector, search pill)
**Why human:** `var(--height-widget-btn)` resolves to 26px per index.html, but computed pixel measurement requires a live browser

#### 2. Hover State After Window Drag

**Test:** Run the app, drag the window by the title bar region, release, then hover over the hamburger/back/forward buttons on the left side
**Expected:** Hover background appears correctly on each button; no stuck hover state from the drag
**Why human:** WKWebView :hover desync after drag is a macOS runtime-only behavior; code uses the correct pattern (JS hover for drag-zone widgets) but final correctness requires hardware testing

#### 3. No Status Text Under Active Script (TBAR-02 completeness)

**Test:** Start a script, observe the toolbar while it runs
**Expected:** Only Stop button is visible — no "Running..." or similar text label anywhere in the toolbar
**Why human:** Static grep confirmed no status text literals, but runtime state rendering (via Zustand store) requires live observation

### Gaps Summary

No blocking gaps. All seven must-have truths are either fully verified statically or verified through code pattern inspection with the only remaining uncertainty being runtime/visual behavior. Three human verification items remain to confirm runtime behavior — these are quality confirmations, not missing implementations.

---

_Verified: 2026-03-14T15:00:00Z_
_Verifier: Claude (gsd-verifier)_
