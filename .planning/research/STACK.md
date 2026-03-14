# Stack Research

**Domain:** IDE-like editor UI shell — pixel-accurate JetBrains Rider New UI restyling
**Researched:** 2026-03-14
**Confidence:** HIGH (core choices), MEDIUM (icon approach)

---

## Context

The existing stack is React 19 + TypeScript + Vite 6 + CodeMirror 6 + Zustand 5, served inside a wry
(WKWebView on macOS / WebView2 on Windows) webview hosted by Rust/Bevy. The tech stack is locked.
This document covers what to add for the Rider UI restyling milestone only: CSS architecture,
font loading, icon system, and component-level patterns.

The webview constraint matters for CSS: wry on macOS uses WKWebView (WebKit), which has full CSS
custom properties support (verified via WebKit/WebKit and the wry DeepWiki docs). No CSS compat
shims are needed for the target platforms.

---

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| CSS Custom Properties (native) | CSS Level 4 / all browsers | Design token layer for Rider color palette | Zero runtime cost, works natively in WKWebView, survives React re-renders, JS-readable via `getComputedStyle`. Declaring `--color-bg-primary: #1E1F22` once in `:root` makes palette changes instant across every component. No library needed. |
| CSS Modules (`.module.css`) | Built into Vite 6 | Scoped styles per component | Vite has built-in CSS Modules support with zero config. Eliminates the inline-style performance penalty (inline styles require camelCase mapping and per-element style recalculation; CSS Modules emit static rules). Critical for an IDE shell that re-renders on every keystroke via CodeMirror state updates. |
| `clsx` | 2.1.1 | Conditional className composition | At 239 bytes, it replaces the current pattern of manual `onMouseEnter`/`onMouseLeave` style mutation. Allows `clsx(styles.btn, isActive && styles.btnActive)` without the DOM thrashing of toggling inline style objects. |
| `@fontsource-variable/inter` | 5.2.8 | Self-hosted Inter variable font for UI text | Fontsource packages fonts as npm dependencies — no CDN, no network request, no CORS issues in the wry webview. The variable version (single WOFF2 file, weights 100–900) is 328 KB vs 724 KB for 7 static-weight files. One import line in `main.tsx` replaces the current `Inter, -apple-system` fallback chain. |
| `@fontsource-variable/jetbrains-mono` | 5.2.8 | Self-hosted JetBrains Mono variable font for the code editor | Same rationale as Inter. The current codebase references `'JetBrains Mono'` with a `'Fira Code'` fallback, meaning it falls back silently on machines without JetBrains Mono installed. Self-hosting guarantees pixel-identical rendering. |

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `react-resizable-panels` | 4.7.2 | Accessible, keyboard-navigable resizable split panes | Needed once the left-panel/editor/right-panel layout needs drag-to-resize handles matching Rider's behavior. The library is headless (no imposed styles), authored by Brian Vaughn (React core team alumnus), and used by shadcn/ui's Resizable component. Use for the horizontal three-column split and the vertical editor/bottom-panel split. Do NOT add until the layout skeleton is finalized — it changes the DOM structure. |
| `@floating-ui/react` | 0.27.19 | Tooltip and autocomplete popup positioning | Rider tooltips and the autocomplete dropdown require viewport-aware positioning (flip on edges, shift to avoid overflow). `@floating-ui/react` is the successor to Popper.js and is what CodeMirror's own tooltip system is modeled after. Use it for any UI-layer popups (breadcrumb dropdown, search-everywhere panel, settings popover). Do NOT use it for the CodeMirror autocomplete dropdown — that is styled via `.cm-tooltip-autocomplete` CSS already in `voidscript-theme.ts`. |
| `lucide-react` | 0.577.0 | Icon system for tool strip, panel headers, status bar | 1,000+ icons, all tree-shakable ES module named exports (no unused icons in bundle), first-class TypeScript, renders inline SVG at any size. Replaces the current hand-drawn SVG literals scattered across Header.tsx, StatusBar.tsx, etc. Use specific Lucide icon names: `Play`, `Bug`, `Square` (stop), `ChevronLeft`, `ChevronRight`, `Settings`, `Search`, `X`, `GitBranch`, `AlertCircle`, `AlertTriangle`. See "What NOT to Use" for alternatives considered. |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| Vite CSS Modules (built-in) | Scoped `.module.css` per component | No config needed. Vite auto-generates unique class names. Add `modules: { localsConvention: 'camelCaseOnly' }` to `vite.config.ts` `css.modules` for TypeScript autocomplete on class names. |
| Browser DevTools / WKWebView inspector | Verify pixel dimensions against Rider reference screenshots | On macOS: Safari > Develop > [app webview]. Measure exact heights (title bar: 40px, tab bar: 35px, status bar: 24px, tool strip width: 36px) from live Rider installation. |

---

## Installation

```bash
# Fonts (self-hosted, no CDN)
npm install @fontsource-variable/inter @fontsource-variable/jetbrains-mono

# CSS class composition
npm install clsx

# Icon system
npm install lucide-react

# Resizable panels (defer until layout skeleton is confirmed)
npm install react-resizable-panels

# Tooltip/popup positioning (defer until breadcrumb/search-everywhere)
npm install @floating-ui/react
```

Import fonts once in `editor-ui/src/main.tsx` (before any component imports):

```typescript
import '@fontsource-variable/inter';
import '@fontsource-variable/jetbrains-mono';
```

Update `editor-ui/index.html` body font-family:

```css
body {
  font-family: 'Inter Variable', -apple-system, sans-serif;
}
```

Update `voidscript-theme.ts` CodeMirror font:

```typescript
'.cm-content': {
  fontFamily: "'JetBrains Mono Variable', 'JetBrains Mono', monospace",
}
```

---

## CSS Architecture Decision

### Use CSS Modules + CSS Custom Properties. Do NOT use CSS-in-JS.

The current codebase uses 100% inline styles with `onMouseEnter`/`onMouseLeave` handlers for hover
states. This approach has three problems at IDE scale:

1. **Performance**: Inline styles require per-element style recalculation on every render. The IDE
   shell re-renders frequently (cursor position, debug state, diagnostics). CSS Modules emit static
   rules evaluated once by the browser.
2. **Hover states**: The current pattern of `e.currentTarget.style.backgroundColor = '#393B40'` in
   event handlers cannot be animated with CSS transitions and breaks when React re-renders the
   component (the inline style wins over any CSS rule). CSS `:hover` pseudo-class handles this
   correctly with zero JS.
3. **Maintainability**: 200+ hardcoded hex values (`#1E1F22`, `#2B2D30`, `#393B40`, etc.) spread
   across 8+ component files. A single palette adjustment requires touching every file.

**The fix**: Declare a design token layer once in a global CSS file, then reference tokens in
component `.module.css` files via `:hover` and CSS custom properties.

### Token Layer (single file: `src/styles/tokens.css`)

```css
:root {
  /* Rider dark surface hierarchy */
  --rider-bg-base:     #1E1F22;  /* editor area, deepest background */
  --rider-bg-elevated: #2B2D30;  /* panels, title bar, status bar */
  --rider-bg-hover:    #393B40;  /* hovered buttons, selected items */
  --rider-bg-active:   #43454A;  /* pressed state */
  --rider-bg-selection:#2E436E;  /* editor selection, active tab highlight */

  /* Borders and separators */
  --rider-border-subtle:  #1E1F22; /* same as base — zero-thickness visual separator */
  --rider-border-default: #393B40;
  --rider-border-strong:  #43454A;

  /* Foreground */
  --rider-fg-primary:   #DFE1E5; /* main text */
  --rider-fg-secondary: #9DA0A8; /* de-emphasized labels */
  --rider-fg-muted:     #6F737A; /* gutter numbers, placeholder text */
  --rider-fg-disabled:  #5A5D63;

  /* Accent / interactive */
  --rider-accent-blue:  #3574F0; /* active tab underline, selection */
  --rider-accent-green: #57965C; /* run button */
  --rider-accent-red:   #DB5C5C; /* stop button, errors */
  --rider-accent-amber: #E08855; /* warnings */

  /* Typography */
  --rider-font-ui:   'Inter Variable', -apple-system, sans-serif;
  --rider-font-code: 'JetBrains Mono Variable', 'JetBrains Mono', monospace;
  --rider-font-size-ui:   13px;
  --rider-font-size-code: 14px;
  --rider-font-size-small: 11px;

  /* Layout dimensions (Rider New UI measurements) */
  --rider-titlebar-height:   40px;
  --rider-tabbar-height:     35px;
  --rider-statusbar-height:  24px;
  --rider-toolstrip-width:   36px;
  --rider-panel-header-height: 30px;
}
```

Import this file once in `main.tsx` after the font imports.

---

## Component Pattern for IDE-Style Buttons

Replace the current inline-style + event handler pattern with CSS Modules + tokens:

**Before (current pattern in Header.tsx):**
```tsx
// 40 lines of inline style + onMouseEnter/Leave per button
```

**After:**
```tsx
// ToolBtn.module.css
.btn {
  width: 28px;
  height: 28px;
  background: none;
  border: none;
  border-radius: 6px;
  color: var(--rider-fg-secondary);
  cursor: pointer;
  transition: background-color 80ms ease, color 80ms ease;
}
.btn:hover:not(:disabled) {
  background-color: var(--rider-bg-hover);
  color: var(--rider-fg-primary);
}
.btn:disabled {
  color: var(--rider-fg-disabled);
  opacity: 0.5;
  cursor: default;
}

// ToolBtn.tsx
import styles from './ToolBtn.module.css';
import clsx from 'clsx';

function ToolBtn({ disabled, className, ...props }) {
  return <button className={clsx(styles.btn, className)} disabled={disabled} {...props} />;
}
```

This pattern scales cleanly to the ~15 components that need hover states.

---

## Icon System: Lucide React

Replace the hand-drawn SVG literals with Lucide named imports:

```tsx
import { Play, Bug, Square, ChevronLeft, ChevronRight,
         GitBranch, Settings, Search, X, AlertCircle } from 'lucide-react';

// Usage — size prop controls both width and height
<Play size={10} />        // Run button (was: <svg width="10" height="10" ...>)
<GitBranch size={10} />   // VCS branch icon in header and status bar
<Settings size={14} />    // Settings gear
```

**Icon sizing convention for Rider New UI:**
- Tool strip icons: `size={16}`
- Toolbar action icons: `size={14}` (run/debug/stop)
- Inline toolbar mini-icons (menu, nav arrows): `size={12}`
- Status bar icons: `size={10}`
- Gutter/panel header icons: `size={12}`

All icons inherit `currentColor` by default — works with the existing color token approach.

---

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| CSS Modules + custom properties | Tailwind CSS | If the project had started with Tailwind. Adding it now means rewriting all existing inline styles AND fighting the `bg-[#1E1F22]` arbitrary-value pattern, which is harder to read than named tokens. Not worth the migration cost for a single-app dark-theme-only UI. |
| CSS Modules + custom properties | styled-components / emotion | CSS-in-JS has a runtime cost on every render (style injection into the CSSOM). For an IDE shell where the editor component re-renders on every keystroke, this is measurable overhead. The community trend since 2023 is away from runtime CSS-in-JS (see Spot Virtual's "Why We're Breaking Up with CSS-in-JS"). |
| CSS Modules + custom properties | vanilla-extract | Zero-runtime CSS-in-TypeScript is excellent for larger systems. Overkill here — CSS Modules with a token file achieves the same outcome with less setup and no additional Vite plugin. |
| `lucide-react` | `@jetbrains/icons` npm package | Use JetBrains icons if exact icon shape fidelity with the real IDE is a hard requirement. The `@jetbrains/icons` package requires `svg-sprite-loader` (webpack-centric, needs Vite adapter). Lucide's `File`, `GitBranch`, `Play`, `Bug` etc. are close enough for this game context and dramatically simpler to use. |
| `lucide-react` | Inline SVG literals (current) | Keep inline SVGs only for the macOS traffic lights (red/yellow/green dots), which have no Lucide equivalent and are 3 lines each. All other icons should migrate to Lucide. |
| `@fontsource-variable/inter` | Google Fonts CDN | CDN requests fail or are slow in the wry webview depending on network. Fontsource npm packages guarantee the font is always available, no network dependency, correct WOFF2 served by Vite's dev server and bundled in `dist/`. |
| `react-resizable-panels` | CSS `resize` property | CSS `resize` only works on block-level elements with overflow set, and cannot produce the two-directional split (horizontal panels + vertical editor/console split) Rider needs. `react-resizable-panels` is headless and designed exactly for this use case. |

---

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| Runtime CSS-in-JS (styled-components, emotion) | Injects styles into CSSOM on every component mount/update. In an IDE shell the editor re-renders on every keystroke — this adds measurable scripting overhead that compounds with CodeMirror's own work. | CSS Modules (zero runtime cost) |
| Tailwind CSS (added mid-project) | Adding Tailwind to an existing inline-style codebase means two styling paradigms co-existing. The migration is all-or-nothing to be maintainable. Dark-theme-only + a fixed design token set doesn't benefit from Tailwind's responsive/multi-theme utilities. | CSS Custom Properties token file |
| `classnames` (npm package) | Superseded by `clsx`, which is smaller (239 B vs 1.1 kB), faster, and has identical API. | `clsx` |
| `@jetbrains/icons` npm package | Requires webpack's `svg-sprite-loader`; has no official Vite plugin; last meaningful update 2022; needs manual Vite raw SVG import workaround. | `lucide-react` |
| Google Fonts / any external CDN for fonts | Network requests may fail or be blocked inside wry webview. Creates a flash of unstyled text (FOUT) on first load. Violates self-containment. | `@fontsource-variable/inter`, `@fontsource-variable/jetbrains-mono` |
| `onMouseEnter`/`onMouseLeave` for hover styling | Causes React synthetic event overhead, breaks CSS transitions, and fires even during rapid pointer movement across many buttons (tool strip, status bar). Cannot use CSS `transition` property. | CSS `:hover` pseudo-class in `.module.css` |
| CSS `transition` on `height: auto` (panel collapse) | Does not animate. The bottom panel and side panels currently snap open/close with no animation. | Use `max-height` transition OR `react-resizable-panels` collapse with CSS `transition: width` on a fixed-size panel |

---

## Stack Patterns by Variant

**For new UI-only components (breadcrumb, search-everywhere, settings gear):**
- Create `ComponentName.tsx` + `ComponentName.module.css` in `src/components/`
- Reference only `var(--rider-*)` tokens in CSS, never hardcode hex
- Use `clsx` for conditional class composition
- Use Lucide for icons

**For refactoring existing components (Header.tsx, StatusBar.tsx, ToolStrip.tsx):**
- Migrate inline styles to `.module.css` one component at a time
- Keep the existing JSX structure — only change `style={{...}}` to `className={styles.xxx}`
- Remove all `onMouseEnter`/`onMouseLeave` style mutation handlers; replace with `:hover` in CSS
- Do NOT change component props or state during the CSS migration pass — pure visual change only

**For CodeMirror theme (voidscript-theme.ts):**
- Do NOT migrate to CSS Modules — CodeMirror's `EditorView.theme()` API generates scoped CSS
  internally; this is the correct approach for editor internals
- DO update hardcoded hex values to reference CSS custom properties via `var(--rider-*)` where
  possible using the `EditorView.baseTheme` approach, but only if it simplifies the token story.
  Otherwise, keep the hex values centralized in `tokens.css` as the single source of truth and
  mirror them manually in the theme file — acceptable for a fixed dark theme.

---

## Version Compatibility

| Package | Compatible With | Notes |
|---------|-----------------|-------|
| `@fontsource-variable/inter@5.2.8` | Vite 6, React 19 | Pure CSS import, no JS — no compatibility surface |
| `@fontsource-variable/jetbrains-mono@5.2.8` | Vite 6, React 19 | Same |
| `lucide-react@0.577.0` | React 19 | Uses `React.forwardRef`; compatible with React 19's ref-as-prop |
| `clsx@2.1.1` | TypeScript 5.7, React 19 | Pure utility, no framework coupling |
| `react-resizable-panels@4.7.2` | React 19 | Actively maintained; v4.x uses React 19 concurrent-safe patterns |
| `@floating-ui/react@0.27.19` | React 19 | Works with React 19; uses hooks only |
| `css-modules` (Vite built-in) | Vite 6.0 | No separate package needed |

---

## Sources

- [fontsource.org/fonts/inter/install](https://fontsource.org/fonts/inter/install) — MEDIUM confidence (official Fontsource docs, version confirmed via `npm view`)
- [fontsource.org/fonts/jetbrains-mono/install](https://fontsource.org/fonts/jetbrains-mono/install) — MEDIUM confidence (official Fontsource docs, version confirmed via `npm view`)
- [npmjs.com/package/lucide-react](https://www.npmjs.com/package/lucide-react) — HIGH confidence (version 0.577.0 confirmed via `npm view lucide-react version`)
- [npmjs.com/package/react-resizable-panels](https://www.npmjs.com/package/react-resizable-panels) — HIGH confidence (version 4.7.2 confirmed via `npm view`)
- [npmjs.com/package/@floating-ui/react](https://www.npmjs.com/package/@floating-ui/react) — HIGH confidence (version 0.27.19 confirmed via `npm view`)
- [npmjs.com/package/clsx](https://www.npmjs.com/package/clsx) — HIGH confidence (version 2.1.1 confirmed via `npm view`)
- [vite.dev/guide/features](https://vite.dev/guide/features) — HIGH confidence (CSS Modules built-in, no version needed)
- [spotvirtual.com/blog/why-were-breaking-up-with-css-in-js](https://www.spotvirtual.com/blog/why-were-breaking-up-with-css-in-js) — MEDIUM confidence (industry context for CSS-in-JS avoidance)
- [deepwiki.com/tauri-apps/wry/3.2-macosios-(wkwebview)](https://deepwiki.com/tauri-apps/wry/3.2-macosios-(wkwebview)) — MEDIUM confidence (wry WKWebView CSS support confirmation)
- `npm view` for all package versions — HIGH confidence (run 2026-03-14)

---

*Stack research for: VOID//SCRIPT Editor — Rider New UI restyling milestone*
*Researched: 2026-03-14*
