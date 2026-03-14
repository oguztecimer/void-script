# Project Research Summary

**Project:** VOID//SCRIPT Editor — JetBrains Rider New UI Restyling Milestone
**Domain:** IDE-like code editor UI shell (pixel-accurate Rider New UI recreation in React + wry/WKWebView)
**Researched:** 2026-03-14
**Confidence:** HIGH (stack and architecture), MEDIUM-HIGH (features and pitfalls)

## Executive Summary

VOID//SCRIPT is a working code editor embedded in a game, built on a locked stack of React 19, TypeScript, Vite 6, CodeMirror 6, Zustand 5, and Rust/Bevy with wry as the WebView host. The current implementation is functionally complete but relies entirely on inline styles with `onMouseEnter`/`onMouseLeave` hover handlers, hardcoded hex values scattered across 8+ component files, and no design token system. The immediate milestone is a pixel-accurate visual match to JetBrains Rider's New UI — the kind of match that holds up under screenshot comparison. All four research tracks converge on the same prerequisite: establish a CSS foundation first, then restyle components in a defined order.

The recommended approach is to introduce a `tokens.css` design token file (CSS custom properties covering all Rider colors, dimensions, and typography), migrate component styles to CSS Modules, self-host Inter and JetBrains Mono via Fontsource npm packages, and add Lucide React for icons. This three-layer foundation (tokens, modules, fonts) makes the remaining Rider UI work straightforward and deterministic. The architecture research prescribes a 10-step build order that respects component dependencies, starting with tokens and primitive extraction before any visible restyle work. Deferring `react-resizable-panels` until the layout skeleton is finalized is explicitly recommended.

The most significant risks cluster around macOS-specific rendering behaviors inside WKWebView: font weight appearing heavier than Rider without `-webkit-font-smoothing: antialiased`, CSS `:hover` becoming unreliable within `-webkit-app-region: drag` title bar regions, custom protocol font loading silently falling back to system fonts without explicit CORS headers, and CodeMirror's injected styles being overridden by external CSS due to specificity ordering. All of these have known, low-cost mitigations that must be applied in Phase 1 before any visual comparison work begins — applying them later is substantially more expensive.

## Key Findings

### Recommended Stack

The existing stack is locked and appropriate for the milestone. The only additions needed are CSS-layer packages and a font solution. CSS Modules with CSS custom properties is the unanimous choice over CSS-in-JS alternatives — zero runtime cost is essential for an IDE shell that re-renders on every keystroke. The wry/WKWebView constraint rules out CDN fonts and requires self-hosted font delivery; Fontsource npm packages resolve this cleanly. `react-resizable-panels` (defer until layout finalized) and `@floating-ui/react` (defer until breadcrumb/search-everywhere) are the only deferred dependencies.

**Core technologies:**
- CSS Custom Properties (`tokens.css`): design token layer — single-file edit for any Rider color correction, zero runtime cost, works natively in WKWebView
- CSS Modules (Vite built-in): scoped component styles with real `:hover` pseudo-classes — eliminates the inline-style event handler pattern that breaks CSS transitions
- `clsx@2.1.1`: conditional className composition — 239 bytes, replaces the verbose inline style toggle pattern
- `@fontsource-variable/inter@5.2.8`: self-hosted Inter variable font — no CDN, no CORS risk, guarantees pixel-identical UI text
- `@fontsource-variable/jetbrains-mono@5.2.8`: self-hosted JetBrains Mono variable font — eliminates silent fallback to Fira Code or system monospace
- `lucide-react@0.577.0`: tree-shakable icon system — replaces hand-drawn SVG literals; all icons inherit `currentColor`
- `react-resizable-panels@4.7.2`: headless resizable split panes — deferred until layout skeleton is confirmed
- `@floating-ui/react@0.27.19`: viewport-aware tooltip/popup positioning — deferred until breadcrumb/search-everywhere work begins

### Expected Features

The features research identified 14 table-stakes items that form the v1 must-have list (P1), 6 should-have items for v1.x once P1 is stable (P2), and 3 items to defer to v2+ or future milestones. The competitor analysis table shows the current implementation is 0–2px short on several dimension measurements, missing two toolbar widgets (Search Everywhere button, Settings gear), and lacking four major chrome elements (Breadcrumb bar, Panel headers, Bottom panel tab strip, status bar navigation path).

**Must have (table stakes — v1 P1):**
- Inter font loading — typography foundation everything else depends on
- Title bar pixel-accurate sizing — widget heights, font weights, separator positions
- Search Everywhere icon button in toolbar — prominent; absence noticed immediately
- Settings gear icon in toolbar — rightmost persistent toolbar item
- Close button hover-reveal on inactive tabs — very visible Rider behavior, pure CSS fix
- Tab bar height 38px, padding `0 16px` — currently 2px short on both
- Gutter fold icons on hover only — CodeMirror config + CSS, low effort
- Tool window strip 40px wide, 36px buttons — currently 36px wide
- Panel header row (title + action icons) — missing from all three side panels
- Status bar navigation path (left segment) — static, no popup needed for v1
- Status bar icon+count diagnostics widget — replace plain text
- Consistent border/separator color audit — `#393B40` for panel separators
- Hover state transitions (150ms ease) — audit all interactive elements
- Bottom panel tab strip — even a single "Console" tab needs Rider tab chrome

**Should have (v1.x, after v1 stabilizes):**
- Breadcrumb bar — requires CodeMirror `syntaxTree()` integration; worth the effort
- Gutter breakpoint overlay — medium refactor; conflicts with current separate-column approach
- Custom tooltip component — replace all native `title=""` attributes
- Keyboard shortcut hints in tooltips — add with tooltip component
- Animated status bar progress spinner — quick win once status bar layout is finalized
- Panel resize handles with hover feedback

**Defer to v2+:**
- Search Everywhere modal with fuzzy search — needs VoidScript symbol index; not ready
- Compact mode toggle — defer until game HUD/cockpit context is designed
- VCS-style gutter change indicators — defer until runtime execution history exists

**Firm anti-features (do not implement):**
- Light theme — dark is canonical for the game; token layer allows it later without doing it now
- Full Search Everywhere (classes, files, actions) — implement script-name-only filter instead
- Multi-row tabs — implement horizontal tab scrolling instead
- Floating/detachable tool windows — wry cannot spawn child windows with shared state

### Architecture Approach

The architecture is a vertical flex shell (Header / MainArea / StatusBar) with a horizontal PanelGroup inside MainArea (ToolStrip / ScriptList / CenterColumn / ToolStrip / DebugPanel). All state flows through a single Zustand store; the IPC bridge is the sole writer from the Rust side. The critical architectural decision for this milestone is introducing a `primitives/` layer (`ToolBtn`, `PanelHeader`, `Separator`, `StatusSegment`) before any restyle work begins — these atoms are used across every panel and must be consistent. The proposed 10-step build order (tokens → primitives → Header → TabBar/Breadcrumb → ToolStrip → panel headers → resizable panels → StatusBar → gutter → tooltips) reflects hard dependencies between components.

**Major components:**
1. `tokens.css` — single source of truth for all design values; must land before any other CSS work
2. `primitives/` (`ToolBtn`, `PanelHeader`, `Separator`) — reusable atoms; extract before restyling consumers
3. `Header` — most visible component; title bar, run toolbar, drag region; has unique macOS pitfalls
4. `TabBar` + `Breadcrumb` — tab sizing and new breadcrumb row; depends on Inter font being loaded
5. `ToolStrip` / side panels — size corrections and panel header chrome; share `PanelHeader` primitive
6. `StatusBar` — 24px height, segment-based layout, icon+count diagnostics
7. `Editor` (CodeMirror) — gutter refinements and theme; must keep all overrides inside `EditorView.theme()`
8. `react-resizable-panels` integration — replaces fixed pixel widths with drag-resize; deferred to after layout is stable

### Critical Pitfalls

1. **macOS font weight explosion without `-webkit-font-smoothing: antialiased`** — WKWebView applies subpixel antialiasing by default, making Inter 13px appear ~SemiBold instead of Regular compared to Rider. Apply `*, *::before, *::after { -webkit-font-smoothing: antialiased }` globally in Phase 1 before any visual comparison.

2. **CSS `:hover` unreliable inside `-webkit-app-region: drag`** — After the user drags the window, CSS `:hover` on elements inside the drag region desyncs in WKWebView. The title bar is uniquely affected. Mitigation: use JS `onMouseEnter`/`onMouseLeave` with `data-hovered` attribute toggling (not `useState`) specifically for titlebar buttons; apply `-webkit-app-region: no-drag` to every interactive leaf element including SVG children.

3. **CodeMirror specificity conflict from external CSS** — CodeMirror 6 injects theme styles before Vite's CSS, so external stylesheets win by source order. Rule: all `.cm-*` overrides live exclusively in `EditorView.theme()` objects. Never put `.cm-tooltip`, `.cm-content`, or `.cm-gutters` rules in external CSS files.

4. **Font CORS failure with wry custom protocol** — `@font-face` declarations may fail silently on clean systems without JetBrains tools installed because WKWebView's custom protocol handler doesn't add `Access-Control-Allow-Origin` headers for font MIME types. Mitigation: use Fontsource npm packages (bundled into `dist/`, no CORS surface) or explicitly set CORS headers in `window.rs` for font responses.

5. **Destroy/recreate `EditorView` on tab switch loses state** — The current `useEffect` dependency on `activeTabId` destroys and recreates the entire CM instance on every tab switch, losing undo history, scroll position, and fold state. Mitigation: maintain a `Map<scriptId, EditorState>` in Zustand; call `view.setState(stateMap.get(id))` on tab switch. This should be fixed before the UI polish work, not after.

## Implications for Roadmap

Based on combined research, all four files point to the same phase structure: foundation first (non-visual prerequisites), then visible restyle by component group, then polish and deferred features. The dependency graph is clear and should not be reordered.

### Phase 1: Foundation (Typography, Tokens, and macOS Fixes)

**Rationale:** Everything else depends on the font being loaded, the token system existing, and macOS rendering being correct. Visual comparison work is meaningless until these land. These are the lowest-complexity changes with the highest risk if deferred.
**Delivers:** `tokens.css` design system, self-hosted Inter + JetBrains Mono, `-webkit-font-smoothing` fix, `color-scheme: dark` on `html`, font CORS verified, `font-feature-settings` explicit in CodeMirror theme.
**Addresses:** Inter font loading (FEATURES P1), consistent border colors (foundation for all), macOS font weight (PITFALLS critical), dark mode flash prevention (PITFALLS).
**Avoids:** Font weight divergence from Rider reference, CORS font failures on clean systems, ligature rendering inconsistency, dark mode white flash on cold launch.

### Phase 2: Primitive Extraction and CSS Architecture Migration

**Rationale:** Before restyling any specific component, extract the shared atoms and eliminate the inline-style hover pattern. This is a refactor-only phase with no new visible features — it pays for itself immediately on the first component that shares a primitive.
**Delivers:** `src/primitives/` with `ToolBtn`, `PanelHeader`, `Separator`, `StatusSegment`; all `onMouseEnter`/`onMouseLeave` style mutations removed from non-titlebar components; CSS Modules + `clsx` in place.
**Uses:** CSS Modules (Vite built-in), `clsx`, `lucide-react` (install now, migrate icons during component restyle).
**Avoids:** Inline hover style debt that becomes exponentially harder to fix at 30+ elements (PITFALLS critical), React re-render storms from `useState` hover tracking (PITFALLS performance).
**Note:** Titlebar buttons retain JS hover handling (`data-hovered` attribute pattern) due to `-webkit-app-region: drag` CSS `:hover` unreliability.

### Phase 3: Header and Title Bar

**Rationale:** Most scrutinized surface; contains the macOS-specific drag region pitfall that must be handled correctly from the start. Must land after primitives are available.
**Delivers:** Pixel-accurate title bar (40px height, 26px widget buttons, correct font weight), Search Everywhere icon button, Settings gear icon, run config selector, macOS traffic light controls.
**Addresses:** Title bar sizing (FEATURES P1), Search Everywhere button (FEATURES P1), Settings gear (FEATURES P1).
**Avoids:** CSS `:hover` failure after window drag (PITFALLS critical — use `data-hovered` pattern here).

### Phase 4: TabBar, Breadcrumb Stub, and Editor State Fix

**Rationale:** TabBar is the next most-visible element after the title bar. The CodeMirror `EditorView` destroy/recreate bug should be fixed here, before gutter work begins, as it affects what editor state the UI can reliably reflect.
**Delivers:** Tab height 38px, padding `0 16px`, close-button hover-reveal on inactive tabs, bottom 2px blue border active indicator, `Breadcrumb.tsx` stub (static path, no `syntaxTree()` integration yet), `EditorState` map pattern replacing destroy/recreate.
**Addresses:** Tab sizing (FEATURES P1), close button hover-reveal (FEATURES P1), breadcrumb bar groundwork (FEATURES P2).
**Avoids:** Undo history loss on tab switch (PITFALLS critical); note `Breadcrumb` static mock is acceptable here — real `syntaxTree()` integration is v1.x.

### Phase 5: ToolStrip, Side Panel Headers, and Bottom Panel Tab Strip

**Rationale:** These three areas share the `PanelHeader` primitive and should land together to avoid visual inconsistency mid-milestone. All three panels (ScriptList, DebugPanel, Console/BottomPanel) get consistent header chrome in one coordinated pass.
**Delivers:** ToolStrip 40px wide, 36px buttons, Lucide icons at correct sizing convention; `PanelHeader` applied to ScriptList, DebugPanel, Console; BottomPanel extracted from App.tsx; bottom panel tab strip with "Console" tab and Rider chrome.
**Addresses:** Tool window strip sizing (FEATURES P1), panel header row (FEATURES P1), bottom panel tab strip (FEATURES P1).
**Implements:** `BottomPanel` component extraction (ARCHITECTURE anti-pattern fix).

### Phase 6: StatusBar

**Rationale:** StatusBar is largely independent of the other panels and can be done as a clean isolated pass after side panels are stable.
**Delivers:** 24px height, 11px Inter, navigation path left segment (static), icon+count diagnostics widget (error circle + warning triangle with counts), segment hover states.
**Addresses:** Status bar navigation path (FEATURES P1), status bar diagnostics icon widget (FEATURES P1).

### Phase 7: Resizable Panels

**Rationale:** Layout structure changes affect every component's positioning. This must come after all panel components are styled and stable — changing DOM structure mid-restyle would invalidate prior work.
**Delivers:** `react-resizable-panels` integration replacing fixed pixel widths; drag-resize handles; panel size persisted in localStorage; collapse/expand without mount/unmount flicker.
**Uses:** `react-resizable-panels@4.7.2`.
**Avoids:** Panel unmount/remount flicker on toggle (ARCHITECTURE anti-pattern 4); panel state loss on close (PITFALLS performance).

### Phase 8: Gutter Refinements and CodeMirror Theme Polish

**Rationale:** Gutter breakpoint overlay refactor conflicts with the current separate-column approach and must be done as one atomic change. Doing this last for the editor avoids disrupting prior tab/editor work.
**Delivers:** Breakpoint circle overlaying line numbers (replaces separate gutter column); fold icons on hover only; custom scrollbar styling for `.cm-scroller` and panel containers; `color-scheme: dark` enforcement in CodeMirror area.
**Addresses:** Gutter breakpoint overlay (FEATURES P2), gutter fold icons (FEATURES P1), overlay scrollbar fix (PITFALLS moderate).
**Avoids:** Scrollbar appearance divergence from Rider (PITFALLS); CodeMirror CSS specificity conflicts (PITFALLS critical — all overrides stay in `EditorView.theme()`).

### Phase 9: Polish, Tooltips, and v1.x Features

**Rationale:** Tooltip component is a sweep across every component and should be last to minimize rework. Animated progress spinner, keyboard shortcut hints, and breadcrumb `syntaxTree()` integration are quick wins once all layout work is stable.
**Delivers:** Custom `Tooltip` component replacing all native `title=""` attributes; keyboard shortcut hints in Run/Debug/Stop tooltips; animated status bar progress spinner during run/debug state; real breadcrumb via CodeMirror `syntaxTree()`.
**Addresses:** Tooltip styling (FEATURES P1 — deferred correctly to this phase), keyboard shortcut hints (FEATURES P2), animated progress (FEATURES P2), breadcrumb real integration (FEATURES P2).

### Phase Ordering Rationale

- Phases 1-2 are non-visual prerequisites. Attempting visual comparison before fonts are loaded and token system exists produces misleading results.
- Phase 3 (Header) comes before Phase 4 (TabBar) because the title bar drag region pitfall is unique and must be handled with correct patterns established from the first restyle.
- Phases 5 and 6 (panels and status bar) are independent of each other but both depend on primitives from Phase 2.
- Phase 7 (resizable panels) must come after all panel components are visually stable because it changes the DOM structure.
- Phase 8 (gutter) comes after tab/editor state work (Phase 4) is settled.
- Phase 9 (tooltips) comes last because it touches every component and minimizing rework requires other components to be stable first.

### Research Flags

Phases that may benefit from deeper research during planning:

- **Phase 4 (EditorState map):** CodeMirror 6's `EditorState` persistence with multiple tabs is well-documented in the CM6 forums but the exact integration with Zustand requires careful design. The discuss.codemirror.net thread "Preserving state when switching between files" is the canonical reference.
- **Phase 8 (breakpoint overlay gutter):** Replacing the current separate `cm-breakpoint-gutter` column with an overlay-on-line-number approach requires writing a custom `GutterMarker` with absolute positioning. No off-the-shelf solution; needs implementation research.
- **Phase 9 (breadcrumb `syntaxTree()`):** The VoidScript language uses a custom CodeMirror grammar. Reading the syntax node hierarchy at cursor position requires understanding how the custom parser exposes node types. Research the parser output before implementing.

Phases with standard patterns (skip research-phase):

- **Phase 1 (foundation):** CSS custom properties, font smoothing, Fontsource — all trivial, well-documented.
- **Phase 2 (CSS Modules migration):** Established Vite + React pattern with no edge cases.
- **Phase 3 (Header restyle):** The drag region pattern is documented in the pitfalls file with exact code.
- **Phase 5 (panel headers):** Pure CSS + component extraction; no new libraries.
- **Phase 6 (StatusBar):** Isolated, well-understood layout work.
- **Phase 7 (react-resizable-panels):** Library has thorough documentation and examples.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All package versions confirmed via `npm view` on 2026-03-14; Vite CSS Modules built-in confirmed; wry WKWebView CSS support verified |
| Features | MEDIUM-HIGH | Official JetBrains Rider New UI docs verified; pixel measurements inferred from code inspection rather than live Rider measurement — 1-2px discrepancies possible |
| Architecture | HIGH | Component boundaries directly derived from existing codebase; patterns (CSS Modules, react-resizable-panels) are well-established; build order is conservative and dependency-respecting |
| Pitfalls | MEDIUM-HIGH | macOS-specific behaviors (font smoothing, drag region, scrollbar) HIGH confidence from multiple sources; wry custom protocol CORS behavior MEDIUM confidence (sparse wry docs) |

**Overall confidence:** HIGH for the milestone scope.

### Gaps to Address

- **Pixel measurements:** The features research notes that Rider dimensions are inferred from code inspection rather than live measurement. When the first restyle component is in place, do a side-by-side comparison against a real Rider installation and adjust tokens accordingly. The `tokens.css` design makes this a one-file correction.
- **Custom protocol CORS headers:** The PITFALLS research identifies potential font loading failure via wry's custom protocol. The exact mechanism for setting `Access-Control-Allow-Origin` in `window.rs`'s asset handler should be verified against the wry API before Phase 1 is marked complete. If Fontsource packages are bundled correctly by Vite into `dist/`, this may be a non-issue — but needs explicit verification on a clean macOS account.
- **VoidScript grammar node types:** For Phase 9 breadcrumb integration, the syntax tree node type names emitted by the VoidScript CodeMirror grammar are not documented in the research. These need to be read from the parser source before breadcrumb implementation begins.

## Sources

### Primary (HIGH confidence)
- `npm view` for all package versions — run 2026-03-14
- [vite.dev/guide/features](https://vite.dev/guide/features) — CSS Modules built-in confirmation
- [JetBrains Rider New UI documentation](https://www.jetbrains.com/help/rider/New_UI.html) — feature reference
- [react-resizable-panels (Brian Vaughn)](https://github.com/bvaughn/react-resizable-panels) — library API
- [codemirror.net/examples/styling](https://codemirror.net/examples/styling/) — CM6 theme architecture

### Secondary (MEDIUM confidence)
- [deepwiki.com/tauri-apps/wry/3.2-macosios-(wkwebview)](https://deepwiki.com/tauri-apps/wry/3.2-macosios-(wkwebview)) — WKWebView CSS support and custom protocol behavior
- [dbushell.com: What's the deal with WebKit Font Smoothing? (Nov 2024)](https://dbushell.com/2024/11/05/webkit-font-smoothing/) — font antialiasing on macOS
- [discuss.codemirror.net: Preserving state when switching between files](https://discuss.codemirror.net/t/preserving-state-when-switching-between-files/2946) — EditorState map pattern
- [electron/electron #13534](https://github.com/electron/electron/issues/13534) — drag region `:hover` suppression
- [fontsource.org](https://fontsource.org) — font package installation

### Tertiary (LOW confidence)
- [tauri-apps/wry #946](https://github.com/tauri-apps/wry/issues/946) — custom protocol font CORS (issue-level evidence, not official docs; needs empirical verification)
- [JetBrains/JetBrainsMono #588](https://github.com/JetBrains/JetBrainsMono/issues/588) — ligature defaults (issue thread, not specification)

---
*Research completed: 2026-03-14*
*Ready for roadmap: yes*
