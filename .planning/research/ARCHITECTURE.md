# Architecture Research

**Domain:** IDE-like code editor UI in React (JetBrains Rider New UI recreation)
**Researched:** 2026-03-14
**Confidence:** HIGH

## Standard Architecture

### System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        Rust/Bevy + wry                          │
│  (frameless window, webview host, VoidScript interpreter)       │
├──────────────────────────────┬──────────────────────────────────┤
│        IPC Layer             │  window.__IPC_RECEIVE / ipc.postMessage  │
├──────────────────────────────┴──────────────────────────────────┤
│                     React Frontend                               │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                    App Shell (App.tsx)                    │   │
│  │  PanelGroup (root vertical flex)                         │   │
│  │  ┌────────────┐                                          │   │
│  │  │  Header    │  (title bar, traffic lights, toolbar)    │   │
│  │  ├────────────┤                                          │   │
│  │  │ Main Area  │  PanelGroup (horizontal)                 │   │
│  │  │ ┌────┬───────────────────────────┬────┐              │   │
│  │  │ │LS  │ Panel (resizable)         │ RS │              │   │
│  │  │ │    │ ┌──────────────────────┐  │    │              │   │
│  │  │ │    │ │  Center Column       │  │DP  │              │   │
│  │  │ │SL  │ │  TabBar              │  │    │              │   │
│  │  │ │    │ │  Editor (CodeMirror) │  │    │              │   │
│  │  │ │    │ │  BottomPanel         │  │    │              │   │
│  │  │ └────┴───────────────────────────┴────┘              │   │
│  │  ├────────────┤                                          │   │
│  │  │ StatusBar  │                                          │   │
│  │  └────────────┘                                          │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  ┌──────────────┐    ┌───────────────────────────────────────┐  │
│  │  Zustand     │    │  CSS Custom Properties (design tokens) │  │
│  │  useStore    │    │  :root { --rider-bg: #1E1F22; ... }   │  │
│  └──────────────┘    └───────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘

LS = Left ToolStrip, RS = Right ToolStrip, SL = ScriptList, DP = DebugPanel
```

### Component Responsibilities

| Component | Responsibility | Communicates With |
|-----------|---------------|-------------------|
| `App` | Root shell, panel open/close orchestration, layout flex tree | All children via useStore |
| `Header` | Title bar, macOS controls, run/debug toolbar, nav widgets | useStore (run state), ipc/bridge |
| `ToolStrip` | Vertical icon rail; toggles adjacent side panel | App (onToggle callback) → useStore |
| `ScriptList` | Left tool window; script tree, click to open tab | useStore (scriptList), ipc/bridge |
| `TabBar` | Horizontal file tabs with active/modified/close state | useStore (tabs, activeTabId) |
| `Editor` | CodeMirror 6 instance; breakpoint gutter, debug line | useStore (tab content, debug state), ipc/bridge |
| `DebugPanel` | Right tool window; variables, call stack | useStore (debugVariables, debugCallStack) |
| `Console` | Bottom tool window; run output log | useStore (consoleOutput) |
| `StatusBar` | Persistent footer; cursor pos, diagnostics, encoding | useStore (cursor, diagnostics) |
| `useStore` | Single Zustand store; all UI + editor state | IPC bridge writes to it; components read from it |
| `ipc/bridge` | Adapter between wry postMessage and Zustand | Rust events → store; store actions → Rust |
| `tokens.css` (new) | CSS custom properties for all design values | All components via `var(--rider-*)` |

## Recommended Project Structure

```
editor-ui/src/
├── tokens.css               # ALL design tokens — single source of truth
├── global.css               # Reset, body, scrollbar, font-face (augments index.html)
├── components/
│   ├── Header/
│   │   ├── Header.tsx
│   │   ├── Header.module.css
│   │   ├── WindowControls.tsx
│   │   ├── WindowControls.module.css
│   │   ├── RunToolbar.tsx
│   │   └── RunToolbar.module.css
│   ├── TabBar/
│   │   ├── TabBar.tsx
│   │   ├── TabBar.module.css
│   │   └── Breadcrumb.tsx      # new: breadcrumb row below tabs
│   ├── ToolStrip/
│   │   ├── ToolStrip.tsx
│   │   └── ToolStrip.module.css
│   ├── ScriptList/
│   │   ├── ScriptList.tsx
│   │   └── ScriptList.module.css
│   ├── Editor/
│   │   ├── Editor.tsx          # unchanged logic
│   │   └── breakpoints.ts      # extracted CM extension
│   ├── DebugPanel/
│   │   ├── DebugPanel.tsx
│   │   └── DebugPanel.module.css
│   ├── Console/
│   │   ├── Console.tsx
│   │   └── Console.module.css
│   ├── BottomPanel/
│   │   ├── BottomPanel.tsx     # extracted from App.tsx inline
│   │   └── BottomPanel.module.css
│   └── StatusBar/
│       ├── StatusBar.tsx
│       └── StatusBar.module.css
├── primitives/              # shared atoms used across components
│   ├── ToolBtn.tsx          # icon button (28x28, hover ring)
│   ├── ToolBtn.module.css
│   ├── PanelHeader.tsx      # standard tool-window header bar
│   ├── PanelHeader.module.css
│   ├── Separator.tsx        # 1px vertical divider
│   └── Separator.module.css
├── codemirror/
│   ├── voidscript-lang.ts
│   ├── voidscript-completion.ts
│   └── voidscript-theme.ts
├── ipc/
│   ├── bridge.ts
│   └── types.ts
├── state/
│   └── store.ts
└── main.tsx
```

### Structure Rationale

- **tokens.css:** Centralising every color, spacing, border-radius, and font value as a CSS custom property eliminates the ~60 magic hex strings currently scattered across inline `style={}` props. It is the single edit point for Rider color matching — change one value, every component updates.
- **components/[Name]/:** Colocating a component's CSS module next to its TSX file prevents naming collisions and makes it obvious which styles belong to which component. No global class pollution.
- **primitives/:** `ToolBtn`, `PanelHeader`, and `Separator` appear in Header, ToolStrip, DebugPanel, and BottomPanel. Extracting them stops duplication of identical inline-style logic across files and gives a single place to fix Rider spacing.
- **CSS modules, not styled-components:** This project has no runtime theme switching (dark only). CSS modules give static class names, real `:hover`/`:focus-visible` pseudo-classes, zero runtime overhead, and native IDE support for CSS. styled-components adds 12 kB runtime and requires a Babel plugin for SSR (irrelevant here) and template-string CSS with degraded editor tooling.

## Architectural Patterns

### Pattern 1: CSS Custom Property Design Token System

**What:** A single `tokens.css` file declares all Rider color, spacing, typography, and dimension values as `:root` CSS custom properties. Components reference only `var(--rider-*)` tokens — never raw hex strings or pixel literals.

**When to use:** Always. Every styled element in this project. Tokens apply to both CSS modules and CodeMirror's `EditorView.theme({})` (pass them in via `getComputedStyle(document.documentElement).getPropertyValue('--rider-*')`).

**Trade-offs:** One extra file and variable look-up indirection. The benefit is enormous: pixel-accurate Rider matching becomes a single-file edit, and token values can be verified against Rider's UI inspection tool without hunting through 15 component files.

**Example:**
```css
/* tokens.css */
:root {
  /* Surfaces */
  --rider-bg:            #1E1F22;  /* editor area, deepest background */
  --rider-surface:       #2B2D30;  /* panels, title bar, status bar */
  --rider-surface-hover: #393B40;  /* hover state on all buttons/segments */
  --rider-border:        #393B40;  /* panel separators */
  --rider-border-deep:   #1E1F22;  /* between title bar and main area */

  /* Text */
  --rider-text-primary:  #DFE1E5;
  --rider-text-secondary:#9DA0A8;
  --rider-text-muted:    #6F737A;
  --rider-text-disabled: #5A5D63;

  /* Accent */
  --rider-accent:        #3574F0;  /* active tab underline, active strip icon */
  --rider-accent-green:  #57965C;  /* run */
  --rider-accent-blue:   #6B9BFA;  /* debug */
  --rider-accent-red:    #DB5C5C;  /* stop / errors */
  --rider-accent-orange: #E08855;  /* warnings */

  /* Layout dimensions */
  --rider-titlebar-h:    40px;
  --rider-statusbar-h:   24px;
  --rider-tabbar-h:      36px;
  --rider-toolstrip-w:   36px;
  --rider-panel-min-w:   180px;
  --rider-bottom-h:      200px;
  --rider-icon-btn-size: 28px;

  /* Typography */
  --rider-font-ui:       'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  --rider-font-code:     'JetBrains Mono', 'Fira Code', monospace;
  --rider-font-size-ui:  13px;
  --rider-font-size-sm:  11px;

  /* Radius */
  --rider-radius-sm: 4px;
  --rider-radius:    6px;
}
```

### Pattern 2: CSS Modules for Component Styling

**What:** Each component gets a `.module.css` file containing its scoped classes. Hover, focus-visible, active pseudo-classes are written in pure CSS. No JavaScript event handlers manage color changes.

**When to use:** Every component that currently uses `onMouseEnter`/`onMouseLeave` to swap inline style colors. That is all of them.

**Trade-offs:** More files, but each file is small and purely declarative. The project gains real `:hover` with hardware-accelerated CSS transitions, which the current approach (React state reconcile on every hover) cannot provide. No new build-time dependencies — Vite handles CSS modules natively.

**Example:**
```css
/* ToolBtn.module.css */
.btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: var(--rider-icon-btn-size);
  height: var(--rider-icon-btn-size);
  background: none;
  border: none;
  border-radius: var(--rider-radius);
  color: var(--rider-text-secondary);
  cursor: pointer;
  padding: 0;
  transition: background-color 80ms ease, color 80ms ease;
}
.btn:hover:not(:disabled) {
  background-color: var(--rider-surface-hover);
  color: var(--rider-text-primary);
}
.btn:disabled {
  color: var(--rider-text-disabled);
  opacity: 0.5;
  cursor: default;
}
```

```tsx
// ToolBtn.tsx
import styles from './ToolBtn.module.css';
export function ToolBtn({ title, onClick, disabled, children }) {
  return (
    <button className={styles.btn} title={title} onClick={onClick} disabled={disabled}>
      {children}
    </button>
  );
}
```

### Pattern 3: react-resizable-panels for Splitter Layout

**What:** Replace the current fixed `width: '200px'` side panels and hard-coded `height: '200px'` bottom panel with `PanelGroup` + `Panel` + `PanelResizeHandle` from `react-resizable-panels` (by Brian Vaughn, React core team alum). Panels get `collapsible` and `defaultSize` props. Collapse/expand is driven imperatively via `panelRef.current.collapse()` / `.expand()` from the Zustand toggle actions.

**When to use:** Whenever a panel's size should be user-draggable, or whenever the panel needs to show/hide with an animated collapse rather than a DOM removal.

**Trade-offs:** Adds one dependency (`react-resizable-panels` ~8 kB). Returns: true drag-resize, keyboard-accessible splitter handles, panel size persistence via `autoSaveId`, and no layout jank from conditional rendering (`{leftPanelOpen && <ScriptList />}` currently causes full mount/unmount on every toggle). This library is authored by the creator of `react-virtualized` and `react-window` — it is actively maintained with v4 adding pixel/rem size constraints.

**Example:**
```tsx
// App.tsx layout sketch
import { Panel, PanelGroup, PanelResizeHandle } from 'react-resizable-panels';
import styles from './App.module.css';

<PanelGroup direction="horizontal" autoSaveId="rider-main">
  {leftPanelOpen && (
    <>
      <Panel
        ref={leftPanelRef}
        defaultSize={20}
        minSize={12}
        collapsible
        onCollapse={() => store.setLeftPanelOpen(false)}
      >
        <ScriptList />
      </Panel>
      <PanelResizeHandle className={styles.resizeHandle} />
    </>
  )}
  <Panel minSize={40}>
    {/* center column: TabBar + Editor + BottomPanel */}
  </Panel>
  {rightPanelOpen && isDebugging && (
    <>
      <PanelResizeHandle className={styles.resizeHandle} />
      <Panel ref={rightPanelRef} defaultSize={22} minSize={14} collapsible>
        <DebugPanel />
      </Panel>
    </>
  )}
</PanelGroup>
```

### Pattern 4: Extract Primitive Components Before Restyling

**What:** Before restyling any region, extract the repeated atomic widgets (`ToolBtn`, `Separator`, `PanelHeader`, `StatusSegment`) into `src/primitives/`. Restyle them once. All consumers inherit the fix automatically.

**When to use:** The pre-condition for any component restyle pass. Without this, fixing the 28px icon button requires editing Header.tsx, ToolStrip.tsx, DebugPanel.tsx, and App.tsx separately, and they will drift again.

**Trade-offs:** One refactor step before the visible styling work begins. Pays for itself on the first component that re-uses a primitive.

## Data Flow

### IPC Event Flow (Rust → UI)

```
Rust backend emits event
    ↓
window.__IPC_RECEIVE(msg)  [ipc/bridge.ts]
    ↓
useStore.getState().action(payload)
    ↓
Zustand state update
    ↓
React re-render of subscribed components
```

### User Action Flow (UI → Rust)

```
User clicks Run button (Header.tsx)
    ↓
sendToRust({ type: 'run_script', script_id })  [ipc/bridge.ts]
    ↓
window.ipc.postMessage(JSON.stringify(msg))
    ↓
Rust handler in voidscript-editor crate
    ↓
Interpreter runs → emits 'console_output' / 'script_finished' events
    ↓
(back to IPC Event Flow above)
```

### Panel Visibility Flow

```
User clicks ToolStrip button
    ↓
onToggle() → useStore.toggleLeftPanel()
    ↓
leftPanelOpen: true → false  (Zustand)
    ↓
App re-renders: PanelResizeHandle imperative collapse OR conditional render
    ↓
ScriptList unmounts / panel collapses
```

### Key Data Flows

1. **Breakpoint toggle:** CodeMirror gutter `mousedown` → dispatches `toggleBreakpointEffect` to CM state → calls `store.toggleBreakpoint()` → calls `sendToRust({ type: 'toggle_breakpoint' })`. The CM state is the local source of truth for gutter markers; Zustand is the authoritative persistent store and sync point with Rust.

2. **Diagnostics display:** Rust emits `error_update` → bridge calls `store.setDiagnostics()` → `Editor.tsx` uses the diagnostics array as a linter source → StatusBar reads error/warning count from the same store slice. Both consumers are always in sync.

3. **Design token application:** `tokens.css` is imported once in `main.tsx` (or `global.css`). CSS custom properties cascade to every child automatically. CodeMirror theme reads them via `getComputedStyle` at extension construction time — the theme object is reconstructed when `activeTabId` changes, which is the natural refresh point.

## Scaling Considerations

This is a single-user desktop application embedded in a game. Scaling in the network sense is irrelevant. The relevant scaling dimension is **component complexity** as more IDE features are added.

| Concern | Now | After this milestone | Future milestones |
|---------|-----|---------------------|------------------|
| Style drift | High (60+ inline hex strings) | Low (tokens.css) | None — add tokens to extend |
| Hover performance | Each hover fires React reconcile | Zero React cost — pure CSS | Remains zero cost |
| Panel layout | Fixed pixel heights, no resize | react-resizable-panels | Add more side panels same way |
| New tool windows | Add in App.tsx, wire manually | Follow Panel + PanelHeader pattern | Consistent by convention |
| State growth | One giant store slice | Same store, add slices as needed | Split to domain slices if > ~30 actions |

### Scaling Priorities

1. **First friction point:** Too many unrelated state fields in one Zustand store. Mitigation: if state exceeds ~40 fields, split into `editorStore` (tabs, content, diagnostics) and `uiStore` (panel visibility, run/debug state). Both stores can be imported together where needed — Zustand has no penalty for multiple stores.
2. **Second friction point:** CodeMirror is destroyed and recreated on every tab switch (current `useEffect` dep array includes `activeTabId`). This is acceptable for a small script count, but if the game eventually supports dozens of open tabs, switch to a view map: keep one CM instance per tab mounted but `display: none`, swap visibility on tab change.

## Anti-Patterns

### Anti-Pattern 1: Inline Style Hover via onMouseEnter/onMouseLeave

**What people do:** `onMouseEnter={(e) => { e.currentTarget.style.backgroundColor = '#393B40'; }}` — as seen in every current component.

**Why it's wrong:** Every hover causes a React synthetic event, a DOM property mutation bypassing React's virtual DOM, and a subtly different code path from the actual render. It blocks CSS transitions (you cannot `transition` an inline style set imperatively this way reliably). It duplicates the same 2-line handler across 30+ elements. It makes the hover color a code constant, not a token.

**Do this instead:** Move hover styles to CSS modules with `:hover` pseudo-class referencing `var(--rider-surface-hover)`. Delete all `onMouseEnter`/`onMouseLeave` handlers.

### Anti-Pattern 2: Hardcoded Hex Strings as Style Prop Values

**What people do:** `style={{ backgroundColor: '#2B2D30' }}` repeated across every component.

**Why it's wrong:** When the correct Rider color for a surface is off by one stop (`#2B2D30` vs `#2C2E33`), finding and correcting every occurrence requires a codebase-wide search-and-replace with manual verification. The 15 currently modified files all repeat `#2B2D30`, `#393B40`, `#1E1F22` with subtle inconsistencies already visible.

**Do this instead:** `var(--rider-surface)`, `var(--rider-surface-hover)`, `var(--rider-bg)` in CSS modules. One token file edit to correct a color.

### Anti-Pattern 3: Inline BottomPanel and Tool Components in App.tsx

**What people do:** `BottomTab`, `PanelHeaderBtn` defined as function components at the bottom of App.tsx, used only once inline.

**Why it's wrong:** They cannot be unit-tested, they cannot be reused in DebugPanel or future tool windows, and their styles cannot be targeted by the global token system without the file growing further.

**Do this instead:** Move `BottomPanel` to `src/components/BottomPanel/`, extract `ToolBtn` and `PanelHeader` to `src/primitives/`. App.tsx becomes a layout-only file under ~60 lines.

### Anti-Pattern 4: Conditional Rendering for Panel Visibility

**What people do:** `{leftPanelOpen && <ScriptList />}` — fully unmounts the panel on close.

**Why it's wrong:** `ScriptList` fetches and renders the script list on mount. Unmounting it discards that work. Re-opening it causes a visible flash while it re-mounts. It also prevents smooth CSS collapse animations.

**Do this instead:** Use `react-resizable-panels` with `collapsible` prop. The panel component stays mounted; the library handles the animated width transition to 0. Panel state (scroll position, selected item) persists across open/close cycles.

## Integration Points

### External Services

| Service | Integration Pattern | Notes |
|---------|---------------------|-------|
| Rust/Bevy backend | `window.ipc.postMessage` (out) / `window.__IPC_RECEIVE` (in) | wry IPC — already implemented, stable |
| CodeMirror 6 | Imperative ref (`EditorView`) managed inside Editor.tsx | Not a React-controlled component; communicate via `view.dispatch()` |
| `react-resizable-panels` | Declarative layout components + imperative `panelRef` for store-driven collapse | Install: `npm install react-resizable-panels` |
| Inter font | Google Fonts or self-hosted via `@font-face` in `global.css` | Currently fallback chain only; needs explicit load for pixel accuracy |
| JetBrains Mono | Same as Inter; already referenced in voidscript-theme.ts | Self-host from JetBrains CDN or include in `/public/fonts/` |

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| IPC bridge ↔ Zustand store | Direct function calls (`store.action()`) in bridge.ts | Bridge is the only writer from the Rust side; components never call sendToRust directly except Header |
| Header ↔ IPC | Header calls `sendToRust()` directly for run/debug/stop/window controls | Acceptable — Header owns the run toolbar; no need for an intermediate store action for fire-and-forget commands |
| Editor ↔ Zustand | Editor reads `activeTab` from store; writes `updateContent`, `setCursor`, `toggleBreakpoint` | Editor is the only writer for content and cursor; all other components read, not write |
| CSS tokens ↔ CodeMirror theme | `voidscript-theme.ts` reads token values via `getComputedStyle` at extension construction | Keeps theme in sync with CSS tokens without duplicating hex values in JS |
| Primitives ↔ Components | Primitives import only tokens (CSS variables), no store | Primitives are pure presentational; state lives one level up |

## Suggested Build Order

This ordering respects dependencies between components. Each step is independently testable.

1. **tokens.css** — Establish the full token vocabulary. No component work, pure CSS. Gate: every current hex string in the codebase has a token name.

2. **Primitive extraction** — Create `ToolBtn`, `Separator`, `PanelHeader`, `StatusSegment` in `src/primitives/` using CSS modules + tokens. Replace all inline-style versions across existing components. Gate: zero `onMouseEnter`/`onMouseLeave` style handlers remain.

3. **Header restyle** — Largest and most visible component. Rider title bar height (40px confirmed), widget spacing, Inter font weight, run config selector, breadcrumb area stub. Gate: pixel comparison against Rider screenshot passes.

4. **TabBar restyle + Breadcrumb** — Rider tabs are 36px tall, 13px Inter, active underline 2px `--rider-accent`. Add `Breadcrumb.tsx` row below the tab bar. Gate: tab active/hover/modified states match Rider.

5. **ToolStrip restyle** — 36px wide, icons 20px, Rider uses icon-only with tooltip. Gate: active icon uses `--rider-accent` background pill, not full-width fill.

6. **Side panel tool window headers** — Extract `BottomPanel` from App.tsx. Standardize tool window header (label + action buttons) using `PanelHeader` primitive. Gate: Scripts, Debug, Run panels all use identical header treatment.

7. **react-resizable-panels integration** — Replace fixed-size panels with resizable layout. Wire collapse/expand to Zustand toggles. Gate: panels drag-resize, persist size in localStorage, collapse without flicker.

8. **StatusBar restyle** — 24px height, 11px Inter, segment hover states via CSS. Gate: matches Rider status bar pixel-for-pixel.

9. **Gutter refinements** — Breakpoint circle overlays line number (absolute position in gutter cell). Fold icons appear on hover. Gate: matches Rider 2025.x gutter behavior.

10. **Tooltip and autocomplete styling** — CodeMirror `.cm-tooltip` and `.cm-tooltip-autocomplete` border-radius (0 in Rider), background, selection highlight. Gate: autocomplete popup matches Rider.

## Sources

- JetBrains Rider New UI documentation: https://www.jetbrains.com/help/rider/New_UI.html
- react-resizable-panels (Brian Vaughn): https://github.com/bvaughn/react-resizable-panels
- CSS Modules vs CSS-in-JS analysis: https://dev.to/alexsergey/css-modules-vs-css-in-js-who-wins-3n25
- CSS custom properties for React: https://www.joshwcomeau.com/css/css-variables-for-react-devs/
- Inline styles performance pitfalls: https://blog.logrocket.com/why-you-shouldnt-use-inline-styling-in-production-react-apps/
- Design tokens guide: https://penpot.app/blog/the-developers-guide-to-design-tokens-and-css-variables/
- Zustand architecture at scale: https://brainhub.eu/library/zustand-architecture-patterns-at-scale

---
*Architecture research for: IDE-like React UI (JetBrains Rider New UI recreation)*
*Researched: 2026-03-14*
