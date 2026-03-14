# Pitfalls Research

**Domain:** Pixel-accurate IDE UI recreation (JetBrains Rider New UI) in React + CodeMirror 6, embedded in wry/WKWebView on macOS
**Researched:** 2026-03-14
**Confidence:** MEDIUM-HIGH (macOS-specific behaviors HIGH, wry-specific LOW where docs are sparse)

---

## Critical Pitfalls

### Pitfall 1: Font Weight Explosion on macOS — Omitting `-webkit-font-smoothing: antialiased`

**What goes wrong:**
On macOS, WKWebView (which wry uses) inherits the default WebKit font rendering, which uses subpixel antialiasing. This makes Inter and JetBrains Mono render noticeably **heavier/bolder** than they appear in the actual Rider IDE or on any other OS. An Inter Regular at 13px looks like a SemiBold. The Rider reference screenshots (and Rider itself) use grayscale antialiasing, producing thinner, crisper strokes.

**Why it happens:**
macOS removed subpixel antialiasing from the OS in Mojave (2018) but browsers/WebKit still enable it for web content by default. WKWebView inherits this behavior. Rider (a JVM app using JetBrains Runtime) has its own font rasterizer that does NOT apply subpixel AA, so comparison screenshots will always look lighter than what the default webview renders.

**How to avoid:**
Apply globally in `index.html`:
```css
*, *::before, *::after {
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}
```
This is safe for the dark-on-dark theme (light text on `#1E1F22` benefits from this). Do **not** apply selectively — inconsistency is visible.

**Warning signs:**
- Title bar text looks heavier than reference screenshots
- Inter at 13px renders at an apparent weight of ~500–600 instead of 400
- JetBrains Mono looks "chunky" rather than crisp in the editor area

**Phase to address:**
Foundation / typography setup — before any visual comparison work begins.

---

### Pitfall 2: CodeMirror Theme Specificity Conflict — Global CSS Overriding `.cm-editor` Styles

**What goes wrong:**
The existing codebase applies styles in `index.html` with `* { margin: 0; padding: 0; box-sizing: border-box; }` and `body { font-family: Inter; font-size: 13px; }`. When components add more global CSS (e.g., for scrollbars, focus rings, or button resets), these can override CodeMirror's injected theme styles because `EditorView.theme()` injects rules with a generated class prefix of moderate specificity. A rule like `button { border: none }` in global CSS can interact unexpectedly with CodeMirror's internal buttons (search panel, etc.). Rider-style overrides on `.cm-tooltip`, `.cm-panels`, `.cm-scroller` from outside the theme object silently lose to conflicting rules.

**Why it happens:**
CodeMirror 6 injects theme styles into a `<style>` tag placed **before** other stylesheets in the `<head>`. This means any stylesheet loaded after it (including Vite's injected CSS) has higher precedence by default. Developers assume that putting `.cm-editor { ... }` in a global CSS file will work — it does for some rules but silently fails for others where the generated prefix raises the injected rule's specificity to match.

**How to avoid:**
- Put ALL CodeMirror visual overrides inside `EditorView.theme({...})` objects, never in external CSS files.
- When an override absolutely must live in CSS (e.g., scrollbar pseudo-elements which can't go in `EditorView.theme`), scope it under `.cm-editor` and verify specificity matches the generated rule.
- Never use `!important` in CodeMirror-adjacent CSS — it breaks the theme cascade in non-obvious ways.
- Test by adding `debugger` in browser devtools and checking computed styles on `.cm-content`, `.cm-gutters`, `.cm-tooltip`.

**Warning signs:**
- A theme color is set in `EditorView.theme` but the computed style in DevTools shows a different value from a CSS file
- Tooltip background (`#393B40`) reverts to white or system default
- Font in the editor area ignores the `fontFamily` in `.cm-content` rule

**Phase to address:**
Theme implementation phase — establish a rule: "CodeMirror styles live in the theme object, nowhere else."

---

### Pitfall 3: Inline Style Hover State Performance and Maintenance Collapse

**What goes wrong:**
The existing codebase uses `onMouseEnter`/`onMouseLeave` event handlers to mutate `e.currentTarget.style.*` directly for hover effects on toolbar buttons, panel buttons, and widgets. This pattern is repeated across `Header.tsx`, `App.tsx`, `ToolStrip.tsx`, etc. As the Rider UI adds more interactive elements (breadcrumbs, search widget, settings gear, tool window headers, panel tabs), the inline hover-style pattern causes: (1) React re-render storms when hover state is tracked in `useState`, (2) stale style after unmount/remount if the DOM mutation persists, (3) inability to do multi-property transitions cleanly, and (4) a maintenance nightmare where changing a hover color requires editing 15+ event handlers.

**Why it happens:**
Inline styles were used because CSS Modules weren't initially set up and it's the fastest path to prototyping. The pattern works fine for 3–4 buttons but breaks down at 30+.

**How to avoid:**
Migrate all hover states to CSS classes. Options in priority order:
1. **CSS Modules** (`.module.css`) — no build config change needed with Vite, zero runtime cost, scoped.
2. **CSS custom properties** — define the palette once as CSS variables in `:root`, then all hover states are `color: var(--jb-text-active)`.
3. Do NOT introduce a CSS-in-JS library (styled-components, emotion) — adds bundle weight and a new abstraction layer for what is essentially static IDE chrome.

Retain inline styles only for values that are genuinely dynamic at runtime (e.g., panel widths from resize handles).

**Warning signs:**
- A new interactive element takes 10+ lines of JSX just for its hover state
- `onMouseLeave` restores wrong color after fast mouse movement between elements
- Profiler shows many re-renders on mouse-over of toolbar area

**Phase to address:**
CSS architecture refactor — should happen before adding new Rider UI components, not after.

---

### Pitfall 4: Destroy/Recreate CodeMirror on Tab Switch — State and Performance Loss

**What goes wrong:**
The current `Editor.tsx` destroys and recreates the entire `EditorView` instance every time `activeTabId` changes. This is simpler to implement but has two serious problems: (1) all transient editor state is lost (scroll position, cursor position, fold state, undo history), and (2) with many open scripts, rapid tab switching causes noticeable flicker as the DOM is torn down and rebuilt with new syntax highlighting applied.

**Why it happens:**
The `useEffect` dependency on `activeTabId` plus calling `viewRef.current.destroy()` at the top of the effect is the straightforward "make it work" approach. It also avoids the complexity of keeping `EditorState` in sync across multiple tabs.

**How to avoid:**
Maintain one `EditorView` instance and a `Map<scriptId, EditorState>` in the store:
- On tab switch: call `view.setState(stateMap.get(scriptId))` to swap state without recreating the DOM
- On every `update.docChanged`: `stateMap.set(scriptId, update.state)` to keep the map current
- On first open of a script: create `EditorState.create(...)` and insert into map
- On tab close: delete from map

This preserves undo history, scroll position, and fold state per tab — exactly what Rider does.

**Warning signs:**
- Switching tabs resets cursor to line 1
- Undo history is lost after switching away and back
- Performance profiler shows `EditorView` constructor running on every tab click

**Phase to address:**
Editor state management — ideally addressed before any UI polish work, as the architecture affects what state the UI can reflect.

---

### Pitfall 5: `-webkit-app-region: drag` Suppresses Hover Styles on Toolbar Buttons

**What goes wrong:**
The title bar uses `className="titlebar-drag"` to make the whole bar draggable, then uses `className="titlebar-no-drag"` on interactive elements. However, elements inside a drag region that are marked `no-drag` correctly receive clicks but **do not receive `:hover` CSS pseudo-class activation reliably** in some WKWebView configurations — particularly on first focus after window activation. This means CSS `:hover` rules on toolbar buttons inside the no-drag area may appear to work in regular testing but fail after the user drags the window and returns.

**Why it happens:**
This is a known behavior in Electron, Tauri, and wry: the drag region intercepts mouse events at the OS level, and the transition between drag-region and no-drag-region hit-testing can miss mouseenter/mouseleave events. The inline `onMouseEnter`/`onMouseLeave` handlers are affected less than CSS `:hover` because they are JS events on the element, but CSS `:hover` depends on the browser's internal hover tracking, which can get desynced.

**How to avoid:**
- Keep using JS `onMouseEnter`/`onMouseLeave` for hover state on all elements inside the titlebar (this is actually one case where the inline style approach is correct).
- When migrating to CSS classes, use a `data-hovered` attribute toggled by JS rather than relying on `:hover` pseudo-class within drag regions.
- Set `-webkit-app-region: no-drag` on ANY interactive element, not just button containers — sometimes the SVG icon inside a button needs it too.
- Test window drag → hover → click sequence explicitly after any titlebar change.

**Warning signs:**
- Hover highlight on a title bar button only works the first time, then stops
- After dragging the window, toolbar buttons are unresponsive to hover until you click elsewhere
- Traffic light buttons show no hover symbol after a drag operation

**Phase to address:**
Title bar implementation phase — establish the drag/no-drag pattern correctly from the start.

---

### Pitfall 6: JetBrains Mono Ligatures Enabled by Default — May Not Match Rider's Rendering

**What goes wrong:**
JetBrains Mono ships with programming ligatures enabled by default (`calt`, `liga` OpenType features). The current theme sets `fontFamily: "'JetBrains Mono', 'Fira Code', monospace"` without controlling ligature behavior. Rider's actual code editor has ligatures **disabled by default** in most installations. If the game's editor shows ligatures (`->` as an arrow glyph, `!=` as `≠`), it will look subtly wrong compared to the reference, and players familiar with Rider will notice.

**Why it happens:**
Web fonts loaded via `@font-face` enable ligatures by default if the font file contains them. Without an explicit `font-feature-settings` declaration, the browser applies whatever OpenType features the font encodes as default.

**How to avoid:**
Decide the canonical behavior, then enforce it explicitly in the CodeMirror theme:
```css
.cm-content {
  font-feature-settings: "calt" 0, "liga" 0;  /* disable ligatures, match Rider default */
}
```
Or if ligatures are desired:
```css
font-feature-settings: "calt" 1, "liga" 1;
```
Either way, be explicit — never rely on browser/font defaults.

**Warning signs:**
- `!=` operator renders as a single glyph in the editor
- `->` or `=>` renders as arrows rather than two characters
- Code looks subtly different from the Rider screenshot reference

**Phase to address:**
Typography/theme phase — set once in `voidscript-theme.ts` and never revisit.

---

### Pitfall 7: Custom Protocol Font Loading — CORS/Origin Mismatch for `@font-face`

**What goes wrong:**
The wry webview loads assets via a custom protocol (`voidscript://localhost/`). On macOS, WKWebView treats the custom protocol origin as its own scheme, but `@font-face` declarations that reference font files are subject to same-origin checks. If the font files are embedded as assets via the custom protocol handler but the CSS references them by a path that the browser resolves to a different origin, the fonts silently fall back to the system font stack — `Inter` falls back to `-apple-system` (SF Pro), and `JetBrains Mono` falls back to `monospace` (Courier New or Monaco). The UI will look correct on systems where Inter and JetBrains Mono are already installed (development machines) but wrong on systems where they are not.

**Why it happens:**
The Vite build produces a `dist/` directory with font files referenced by relative URLs. The custom protocol handler serves them, but the browser's font loading engine may check CORS response headers. WKWebView's custom protocol does not automatically add permissive CORS headers unless the handler explicitly sets `Access-Control-Allow-Origin: *`.

**How to avoid:**
- Bundle fonts as Base64 data URIs in the CSS at build time (small overhead: ~400KB for JetBrains Mono Regular + Italic woff2), or
- Ensure the custom protocol handler in `window.rs` sets the `Access-Control-Allow-Origin` header in the response for font MIME types (`font/woff2`, `font/ttf`).
- Verify at startup using `document.fonts.check("13px 'JetBrains Mono'")` in the webview console.
- Test on a clean macOS account without JetBrains tools installed.

**Warning signs:**
- Editor text looks like Monaco or Courier New on a fresh system
- `document.fonts.check("13px 'JetBrains Mono'")` returns `false` in DevTools
- UI text looks like SF Pro (slightly different letter spacing compared to Inter)

**Phase to address:**
Asset bundling / infrastructure phase — resolve before any typography QA.

---

### Pitfall 8: macOS Overlay Scrollbar Appearing Inside CodeMirror Scroller

**What goes wrong:**
macOS shows overlay scrollbars (semi-transparent, appearing on scroll) by default when the system has "Show scroll bars: Automatically based on mouse or trackpad" set. Inside the CodeMirror `.cm-scroller`, this produces a scrollbar that floats over the last few characters of code. Unlike in the actual Rider IDE (a native JVM app that controls its own scrollbar rendering), the WKWebView overlay scrollbar uses macOS's native styling — it will be a rounded gray overlay, not Rider's thin dark-themed scrollbar.

**Why it happens:**
WKWebView inherits the system scrollbar style. `overflow: hidden` on the container prevents scrollbars on outer wrappers but `.cm-scroller` uses `overflow: auto` internally and this cannot be changed without patching CodeMirror.

**How to avoid:**
Apply custom scrollbar styling scoped to `.cm-scroller` and all panel containers:
```css
.cm-scroller::-webkit-scrollbar { width: 8px; height: 8px; }
.cm-scroller::-webkit-scrollbar-track { background: transparent; }
.cm-scroller::-webkit-scrollbar-thumb { background: #4E5157; border-radius: 4px; }
.cm-scroller::-webkit-scrollbar-thumb:hover { background: #6F737A; }
```
These `::-webkit-scrollbar` pseudo-elements are fully supported in WKWebView. Apply the same pattern to `.ScriptList`, `.Console`, and `.DebugPanel` containers.

Note: `scrollbar-width: thin` (the standards-track property) does not change overlay scrollbar appearance on macOS — it only affects non-overlay (Windows-style) scrollbars.

**Warning signs:**
- A gray macOS scrollbar thumb appears inside the editor when scrolling code
- The scrollbar looks out of place against the `#1E1F22` background
- Scrollbar appearance changes when the user changes their macOS system scroll preferences

**Phase to address:**
Theme polish phase — apply after core layout is stable.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Inline hover styles via `onMouseEnter`/`onMouseLeave` | Fast to write, works immediately | 15+ elements become unmaintainable; transitions are impossible; no CSS hover for drag-region elements | Only in drag-region titlebar buttons where CSS `:hover` is unreliable |
| Hardcoded hex color values in inline styles | No setup required | Changing a color token requires grep-and-replace across 10+ files | Never for palette colors — use CSS custom properties |
| Destroying and recreating `EditorView` on tab switch | Simpler state management | Loses undo history, scroll, folds; noticeable flicker | Acceptable as initial prototype, must be replaced before release |
| Skipping `font-display: swap` on `@font-face` | Slightly simpler CSS | Code editor shows wrong fallback font until JetBrains Mono loads | Never — always declare `font-display: block` for code fonts (swap flicker in editor is worse than brief invisible text) |
| Global CSS file for component styles | Fast to write | Specificity conflicts with CodeMirror injected styles; impossible to predict which rule wins | Never for CodeMirror-adjacent styles |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| wry custom protocol + fonts | Serving font files without CORS headers | Set `Access-Control-Allow-Origin: *` on font responses in `embedded_assets` handler, or embed fonts as data URIs |
| wry + frameless window + drag | Applying `-webkit-app-region: drag` to entire header, then forgetting to mark SVG children as `no-drag` | Apply `no-drag` to every interactive leaf element, including SVG icons inside buttons |
| CodeMirror 6 + React `useEffect` | Recreating `EditorView` on every render cycle | Use a stable ref, create once, update via `view.setState()` and `view.dispatch()` |
| CodeMirror 6 + external CSS | Overriding `.cm-tooltip` in a global stylesheet | Use `EditorView.theme()` for all CodeMirror visual overrides; external CSS for non-CM elements only |
| wry + macOS dark mode | Not setting `prefers-color-scheme: dark` media query | WKWebView inherits system appearance; add `color-scheme: dark` to `html` element to suppress any light-mode flash |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| `useState` for hover on every toolbar button | Each hover triggers React reconciliation for the toolbar subtree | JS DOM mutation in `onMouseEnter`/`onMouseLeave` without `useState`, or CSS classes | Noticeable at ~8+ buttons rendering simultaneously |
| Recreating `EditorView` on tab switch | 50–150ms blank flash on tab click; GC pressure | Reuse one `EditorView`, swap `EditorState` per tab | Immediately perceptible |
| Inline `style` objects recreated on every render | React's reconciler diffs every style property on every render | `useMemo` or extract static style objects outside component | After ~20 re-renders/second (e.g., animated debug step) |
| Diagnostic linter running synchronously | Editor freezes briefly when diagnostics update | Run linter async via `linter()` with a `delay` option | Any script with >50 lines and frequent edits |
| Many `useStore` subscriptions per component | Zustand fires component re-renders for every unrelated store change | Select minimal slices; split subscriptions | After ~10 store fields per component |

---

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Breadcrumb showing raw path segments instead of semantic names | Feels like a file path, not an IDE navigator | Show class/function hierarchy as Rider does: `FileName > functionName` |
| Tab close button visible at all times | Visual clutter; Rider shows it only on hover | Show `×` only on hover or when tab has unsaved changes |
| "Select a script to begin editing" empty state text is italic and vague | Feels like an error state, not a welcome state | Match Rider's "No file is open" treatment with centered icon + shorter label |
| Status bar items with no click affordance | Developer expects to click "UTF-8" to change encoding, "Ln 1 Col 1" to jump to line | Add pointer cursor and click handlers even if they are no-ops initially |
| Breakpoint gutter has no hover affordance | Player doesn't know the gutter is clickable | Show a faint circle on gutter hover (Rider's pattern) before the click |

---

## "Looks Done But Isn't" Checklist

- [ ] **Font loading:** `document.fonts.check("13px 'JetBrains Mono'")` returns `true` in a clean webview session (no system fonts installed)
- [ ] **Font smoothing:** Apply `-webkit-font-smoothing: antialiased` globally and verify Inter at 13px matches Rider weight visually
- [ ] **Ligatures:** Confirm `font-feature-settings` is explicit in `.cm-content` — check by typing `->` and `!=` in the editor
- [ ] **Drag regions:** After dragging the window to a new position, verify all toolbar buttons still respond to hover and click
- [ ] **Scrollbars:** Scroll code in editor and script list — confirm macOS overlay scrollbar is replaced by custom styled scrollbar
- [ ] **CodeMirror theme specificity:** Open DevTools, inspect `.cm-tooltip` background — should be `#393B40` not white
- [ ] **Tab switch state:** Switch tabs 3 times, return to first tab — undo history (`Cmd+Z`) should still work
- [ ] **Dark mode coherence:** `html { color-scheme: dark }` is set — no white flash on load or when system appearance changes
- [ ] **No-drag children:** Verify SVG icons inside header buttons do not accidentally receive `titlebar-drag` class via inheritance
- [ ] **Transition consistency:** All hover state changes use the same duration (Rider uses ~100ms) — no instant jumps, no 300ms delays

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Font weight wrong on macOS | LOW | Add two CSS lines to `index.html`; no component changes |
| CodeMirror specificity conflicts discovered late | MEDIUM | Audit all `.cm-*` styles in external CSS files; move each to `EditorView.theme()` |
| Inline hover styles across 20+ components | HIGH | Introduce CSS Modules file-by-file; cannot be done in one pass |
| Destroy/recreate pattern baked into state architecture | HIGH | Requires refactoring store (Zustand) to hold `EditorState` map; affects `Editor.tsx`, `store.ts`, tab close logic |
| Font CORS failure in production builds | MEDIUM | Add `Access-Control-Allow-Origin` header to font responses in `window.rs` Rust code |
| Drag region hover bug found post-shipping | MEDIUM | Convert affected CSS `:hover` rules to JS-driven `data-hovered` attributes |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Font weight (missing antialiasing) | Phase 1: Typography foundation | Visual comparison of Inter 13px to Rider screenshot |
| CodeMirror CSS specificity | Phase 1: Theme architecture | DevTools computed style on `.cm-tooltip`, `.cm-content` |
| JetBrains Mono ligature behavior | Phase 1: Typography foundation | Type `->` and `!=` in editor, confirm expected rendering |
| Font CORS / custom protocol | Phase 1: Asset infrastructure | Test on fresh macOS user account without JetBrains tools |
| Drag region hover bugs | Phase 2: Title bar implementation | Drag window, then hover toolbar buttons; run in CI on macOS |
| Inline style debt | Phase 2: CSS architecture migration | No `onMouseEnter` style mutations outside titlebar drag context |
| Destroy/recreate CodeMirror | Phase 2: Editor state architecture | Switch tabs 5x, verify undo history preserved on return |
| Overlay scrollbar appearance | Phase 3: Theme polish | Compare scrollbar in editor/panels to Rider reference |
| macOS dark mode flash | Phase 1: HTML foundation | Cold-launch app with system dark mode; no white flash |

---

## Sources

- [What's the deal with WebKit Font Smoothing? — dbushell.com, Nov 2024](https://dbushell.com/2024/11/05/webkit-font-smoothing/)
- [CodeMirror Styling Example — codemirror.net](https://codemirror.net/examples/styling/)
- [CM6 base theme override — discuss.codemirror.net](https://discuss.codemirror.net/t/cm6-base-theme-override/2836)
- [Increase specificity of editor styles — discuss.codemirror.net](https://discuss.codemirror.net/t/increase-specificity-of-editor-styles/9146)
- [Preserving state when switching between files — discuss.codemirror.net](https://discuss.codemirror.net/t/preserving-state-when-switching-between-files/2946)
- [Frameless window makes -webkit-app-region:drag styled element ignore :hover — electron/electron #13534](https://github.com/electron/electron/issues/13534)
- [Set scroll bounce on WKWebView — tauri-apps/wry #557](https://github.com/tauri-apps/wry/issues/557)
- [JetBrains Mono ligatures and font-variant-ligatures — JetBrains/JetBrainsMono #588](https://github.com/JetBrains/JetBrainsMono/issues/588)
- [Custom protocol not working on Mac OS X — tauri-apps/wry #946](https://github.com/tauri-apps/wry/issues/946)
- [macOS/iOS (WKWebView) — deepwiki.com/tauri-apps/wry](https://deepwiki.com/tauri-apps/wry/3.2-macosios-(wkwebview))
- [Revisiting CodeMirror 6 implementation in React — codiga.io](https://www.codiga.io/blog/revisiting-codemirror-6-react-implementation/)
- [::-webkit-scrollbar — MDN Web Docs](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Selectors/::-webkit-scrollbar)
- [macOS font rendering — skip.house](https://skip.house/blog/macos-font-rendering)

---
*Pitfalls research for: Rider-accurate IDE UI in React + CodeMirror 6 + wry/WKWebView*
*Researched: 2026-03-14*
