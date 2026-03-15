# Phase 09: Polish and Tooltips - Research

**Researched:** 2026-03-15
**Domain:** React tooltip primitives, CSS absolute positioning, CodeMirror 6 updateListener, VoidScript line-scan heuristic
**Confidence:** HIGH

## Summary

Phase 9 is a pure polish phase: no new data flows or backend changes. It has three self-contained deliverables. First, a custom `Tooltip` primitive that replaces every native `title=` attribute across the app. Second, a breadcrumb bar slotted below the `TabBar` in `App.tsx` that reads cursor position from Zustand and scans the active document for `def` blocks. Third, keyboard shortcut hint text wired into the tooltip content for Run/Debug/Stop and debug step buttons.

The implementation is entirely frontend. No new npm packages are required — the tooltip can be built with plain React + CSS Modules using absolute positioning (no portal needed in a wry single-document WebView). The breadcrumb heuristic reads from an existing Zustand store field (`cursorLine`) plus the active tab's document content that is already synchronised via `updateContent`. CodeMirror's `EditorView.updateListener` (already used in `Editor.tsx`) provides the hook point for emitting cursor position to the store — this is already wired; the only addition is calling a new `setBreadcrumbFunction` store action from the listener.

**Primary recommendation:** Build `Tooltip.tsx` as a pure CSS-absolute wrapper; build `BreadcrumbBar.tsx` as a display-only component that reads store state; modify `ToolBtn.tsx` to render `<Tooltip>` instead of the native `title` attribute.

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Tooltip visual styling**
- Rider-dark tooltip: `#3C3F41` background, `#BBB` text, 1px `#555` border, no box-shadow
- Padding: 4px 8px, border-radius 4px
- Font: 12px Inter (matches Rider's tooltip size — slightly smaller than 13px UI text)
- Shortcut hint text rendered in `--text-secondary` (muted) after the label text, e.g. "Run `Shift+F10`"
- Single Tooltip primitive component in `src/primitives/Tooltip.tsx` with CSS Module

**Tooltip behavior**
- Show delay: 800ms (Rider default for toolbar tooltips)
- Hide: instant on mouse leave
- Position: below the trigger by default; flip above if tooltip would clip viewport bottom
- Animation: opacity fade-in 100ms ease, no exit animation
- Only one tooltip visible at a time (moving between buttons resets the delay timer)

**Tooltip integration**
- ToolBtn primitive gets tooltip support — replace native `title` attribute with custom Tooltip
- All `title=` attributes across components replaced: Header toolbar buttons, ToolStrip items, ScriptList actions, DebugPanel actions, BottomTabStrip actions, TrafficLight buttons, SearchPill
- ToolBtn interface: `title` prop continues to exist but renders custom Tooltip instead of native attribute

**Keyboard shortcut hints (display-only)**
- Shortcuts shown in tooltip text only; no keybinding wiring
- Rider-standard shortcut assignments:
  - Run: `Shift+F10`
  - Debug: `Shift+F9`
  - Stop: `Ctrl+F2`
  - Resume: `F9`
  - Step Over: `F8`
  - Step Into: `F7`
  - Step Out: `Shift+F8`
  - Search Everywhere: `Shift Shift` (already shown inline on the pill)
- ToolStrip items already have `shortcut` in their data — tooltip will use it
- Format in tooltip: `"Label (Shortcut)"` — consistent with existing ToolStrip title pattern

**Breadcrumb bar content**
- VoidScript uses `StreamLanguage` (token-based, no Lezer syntax tree)
- Heuristic approach: scan document lines backwards from cursor to find enclosing `def` block
- Breadcrumb segments: `filename` › `function_name` (when cursor is inside a `def` block)
- When cursor is at top level (outside any `def`): just `filename`
- Nested blocks (e.g. `if` inside `def`) not tracked — only function-level granularity
- Separator: chevron `›` matching NavPath in status bar

**Breadcrumb bar placement and styling**
- Positioned below TabBar, above Editor — matching Rider's breadcrumb bar location
- Full width of the editor area (between left and right panels)
- Height: 24px, matching status bar proportions
- Background: `--bg-panel` (#2B2D30), bottom border: `--border-default` (#393B40)
- Font: 12px Inter, `--text-secondary` for segments, `--text-primary` for last segment
- Non-interactive for now — display only, no click behavior

### Claude's Discretion
- Tooltip portal/positioning implementation (CSS absolute vs React portal)
- Exact heuristic for detecting `def` blocks (regex vs line scanning)
- Whether to debounce breadcrumb updates on rapid cursor movement
- Tooltip arrow/caret presence (Rider tooltips don't have arrows — likely skip)
- Edge case handling for deeply nested code in breadcrumb

### Deferred Ideas (OUT OF SCOPE)
- Functional keyboard shortcuts (wiring actual keybindings for Run/Debug/Stop)
- Breadcrumb click-to-navigate (clicking function name jumps to its definition)
- Tooltip for CodeMirror hover (e.g. variable type on hover) — requires language server
- Autocomplete/suggestion tooltip styling — separate feature
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| EDIT-01 | Breadcrumb navigation bar below tab bar showing cursor position in syntax tree | Zustand `cursorLine` already exists; `updateContent` syncs doc; line-scan heuristic is sufficient for StreamLanguage |
| PLSH-03 | Custom tooltip component with Rider dark styling replacing native browser title attributes | Pure CSS-absolute React component; all `title=` sites audited and listed below |
| PLSH-04 | Keyboard shortcut hints displayed in tooltip text (e.g., "Run (Shift+F10)") | Tooltip `content` prop accepts a hint string; ToolBtn shortcut prop passes through |
</phase_requirements>

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| React | 19.0 | Tooltip state (show/hide), breadcrumb component | Already in project |
| CSS Modules | Vite-native | Scoped tooltip and breadcrumb styling | Project-wide standard |
| Zustand | 5.0 | Breadcrumb reads `cursorLine`, `activeTabId`, `tabs` from existing store | Already in project |
| `@codemirror/view` `EditorView.updateListener` | 6.35 | Cursor position hook — already firing in `Editor.tsx` | Already wired |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| None new | — | No additional packages needed | — |

**Installation:**

No new packages required. All dependencies are already present.

---

## Architecture Patterns

### Recommended Project Structure

```
src/
├── primitives/
│   ├── Tooltip.tsx          # NEW: wraps trigger, manages show/hide delay
│   ├── Tooltip.module.css   # NEW: tooltip visual styles
│   ├── ToolBtn.tsx          # MODIFIED: replace title= with <Tooltip>
│   └── ToolBtn.module.css   # unchanged
├── components/
│   ├── BreadcrumbBar.tsx    # NEW: display-only breadcrumb bar
│   ├── BreadcrumbBar.module.css  # NEW
│   ├── Header.tsx           # MODIFIED: TrafficLight + SearchPill title= removed
│   ├── ToolStrip.tsx        # unchanged (uses ToolBtn which gets fixed)
│   ├── ScriptList.tsx       # unchanged (uses ToolBtn which gets fixed)
│   ├── DebugPanel.tsx       # MODIFIED: variable value title= removed
│   └── BottomTabStrip.tsx   # unchanged (uses ToolBtn which gets fixed)
├── state/
│   └── store.ts             # MODIFIED: add breadcrumbFunction string field + setter
└── App.tsx                  # MODIFIED: insert <BreadcrumbBar /> between TabBar and editorArea
```

### Pattern 1: Tooltip Primitive — CSS Absolute, No Portal

**What:** `<Tooltip>` wraps the trigger child. It renders a hidden `<div>` positioned `absolute` relative to the wrapper (which is `position: relative`). Show/hide is controlled by a `useRef`-based timer started on `mouseenter` and cancelled on `mouseleave`.

**When to use:** Single-document wry WebView — no multi-document clipping concern. Absolute positioning is simpler and performant.

**Example:**
```typescript
// src/primitives/Tooltip.tsx
import { useRef, useState } from 'react';
import styles from './Tooltip.module.css';

interface TooltipProps {
  content: string;        // full display string, e.g. "Run (Shift+F10)"
  children: React.ReactNode;
  disabled?: boolean;
}

export function Tooltip({ content, children, disabled }: TooltipProps) {
  const [visible, setVisible] = useState(false);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const show = () => {
    if (disabled) return;
    timerRef.current = setTimeout(() => setVisible(true), 800);
  };

  const hide = () => {
    if (timerRef.current) clearTimeout(timerRef.current);
    setVisible(false);
  };

  return (
    <div className={styles.wrapper} onMouseEnter={show} onMouseLeave={hide}>
      {children}
      {visible && <div className={styles.tooltip}>{content}</div>}
    </div>
  );
}
```

**CSS:**
```css
/* Tooltip.module.css */
.wrapper {
  position: relative;
  display: contents; /* or inline-flex — see pitfall below */
}

.tooltip {
  position: absolute;
  top: calc(100% + 4px);
  left: 50%;
  transform: translateX(-50%);
  background: #3C3F41;
  color: #BBBBBB;
  border: 1px solid #555555;
  border-radius: 4px;
  padding: 4px 8px;
  font-size: 12px;
  white-space: nowrap;
  pointer-events: none;
  z-index: 1000;
  opacity: 1;
  animation: fadeIn 100ms ease;
}

@keyframes fadeIn {
  from { opacity: 0; }
  to   { opacity: 1; }
}
```

**Viewport flip:** When `visible` becomes true, use a `useEffect` + `getBoundingClientRect()` on the tooltip ref to check if it clips `window.innerHeight`. If clipped, toggle a CSS class that flips `top` to `bottom: calc(100% + 4px)`.

### Pattern 2: ToolBtn with Tooltip

**What:** `ToolBtn` gains a `shortcut?: string` prop. If `shortcut` is provided, the tooltip content is `"title (shortcut)"`. The native `title` attribute on the `<button>` is removed entirely.

**Example:**
```typescript
// ToolBtn.tsx — modified signature
interface ToolBtnProps {
  title: string;
  shortcut?: string;    // NEW: e.g. "Shift+F10"
  // ... rest unchanged
}

// Inside return:
<Tooltip content={shortcut ? `${title} (${shortcut})` : title}>
  <button className={classes} onClick={onClick} disabled={disabled} style={...}>
    {children}
  </button>
</Tooltip>
```

The native `title={title}` attribute is removed from the `<button>` element.

### Pattern 3: Breadcrumb Heuristic

**What:** A pure function that scans from the cursor line backwards through the document to find the nearest preceding `def` line.

**When to use:** Always called from `BreadcrumbBar` when `cursorLine` or `activeTabId` changes.

**Example:**
```typescript
// src/components/BreadcrumbBar.tsx

// DEF_RE matches: "def function_name(" at any indentation
const DEF_RE = /^\s*def\s+([a-zA-Z_]\w*)\s*\(/;

function findEnclosingFunction(content: string, cursorLine: number): string | null {
  const lines = content.split('\n');
  // scan backward from cursorLine-1 (0-indexed) to 0
  for (let i = cursorLine - 1; i >= 0; i--) {
    const m = DEF_RE.exec(lines[i]);
    if (m) return m[1];
  }
  return null;
}
```

The component subscribes to Zustand: `cursorLine`, `activeTabId`, `tabs`. The active tab's `content` is already in the store (via `updateContent` calls in the editor listener). No additional IPC or CodeMirror API access is needed.

### Pattern 4: Breadcrumb Store State

**What:** The breadcrumb function name is derived reactively inside `BreadcrumbBar.tsx` using Zustand selectors — NOT stored in Zustand. Zustand already has `cursorLine` and the active tab's `content`. Computing breadcrumb inside the component is sufficient and avoids redundant state.

```typescript
// BreadcrumbBar.tsx
export function BreadcrumbBar() {
  const cursorLine = useStore((s) => s.cursorLine);
  const activeTabId = useStore((s) => s.activeTabId);
  const tabs = useStore((s) => s.tabs);

  const activeTab = tabs.find((t) => t.scriptId === activeTabId);
  const filename = activeTab?.name ?? null;
  const fnName = activeTab
    ? findEnclosingFunction(activeTab.content, cursorLine)
    : null;

  if (!filename) return null;

  return (
    <div className={styles.bar}>
      <span className={styles.segment}>{filename}</span>
      {fnName && (
        <>
          <span className={styles.chevron}> › </span>
          <span className={styles.segmentActive}>{fnName}</span>
        </>
      )}
    </div>
  );
}
```

### Pattern 5: App Layout Insertion

The breadcrumb bar goes inside `.center` in `App.tsx`, after `<TabBar />` and before the `<Group>` (vertical resizable group). It is a fixed-height element like the TabBar, so it needs `flex-shrink: 0` in its CSS (established pattern from Phase 6).

```tsx
// App.tsx — center panel
<div className={styles.center}>
  <TabBar />
  <BreadcrumbBar />          {/* NEW — inserted here */}
  <Group id="void-center-layout" ...>
    ...
  </Group>
</div>
```

### Pattern 6: TrafficLight and SearchPill Custom Tooltip

`TrafficLight` in `Header.tsx` is a plain `<div>`, not a `ToolBtn`. It needs the `<Tooltip>` wrapper applied directly. `SearchPill` is a `<button>` with its own inline `title=` — same approach.

```tsx
// TrafficLight — wrap with Tooltip
<Tooltip content={title}>
  <div onClick={onClick} className={styles.trafficLight} style={{ backgroundColor: color }}>
    {hoverSymbol}
  </div>
</Tooltip>
```

The `title` prop on `TrafficLight` stays in its signature but is forwarded to `<Tooltip content={title}>` instead of the DOM.

### Anti-Patterns to Avoid

- **`display: contents` on wrapper div:** `display: contents` removes the box from layout and breaks `position: relative` for the tooltip positioning. Use `display: inline-flex` or `display: inline-block` on `.wrapper` instead. For `ToolBtn` which is already `inline-flex` inside a flex container, wrapping with `display: inline-flex` wrapper is correct.
- **Storing tooltip visibility in Zustand:** It is transient UI state; `useState` inside the component is correct.
- **Using a React portal for tooltips:** Not needed in wry. Portals add complexity and `document.body` mounting makes z-index management harder.
- **Reading CodeMirror document via `viewRef` for breadcrumb:** The document content is already mirrored into Zustand `tab.content` on every change via `updateContent`. Reading from Zustand is correct; accessing `viewRef` from `BreadcrumbBar` would require lifting the ref unnecessarily.
- **Forgetting `flex-shrink: 0` on BreadcrumbBar:** All fixed-height siblings of `flex: 1` children must set `flex-shrink: 0` (established in Phase 6).

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Viewport flip detection | Complex custom layout engine | `getBoundingClientRect()` + conditional CSS class | One check on mount/update is sufficient; no library needed |
| Tooltip animation | CSS transitions on `visibility` | CSS `animation: fadeIn` on mount | `visibility` transition requires exit animation delay which is explicitly not wanted |
| Syntax tree cursor position | Lezer tree walking | Regex backward line scan | VoidScript uses StreamLanguage with no Lezer tree; tree walking would require a parser rewrite |

**Key insight:** All three deliverables are solved with existing project primitives. The tooltip requires zero new dependencies. The breadcrumb heuristic is 10 lines of string processing.

---

## Common Pitfalls

### Pitfall 1: `display: contents` Breaks Absolute Tooltip Positioning

**What goes wrong:** If `.wrapper` uses `display: contents`, it has no layout box. `position: absolute` on the tooltip child has no nearest positioned ancestor to anchor to, and it floats to a distant ancestor.

**Why it happens:** `display: contents` makes the element "vanish" from the box tree. The `position: relative` rule has no effect.

**How to avoid:** Use `display: inline-flex` (or `inline-block`) on `.wrapper`. Confirm in browser devtools that the wrapper has a layout box with the correct dimensions.

**Warning signs:** Tooltip appears far from the trigger, or at top-left of a panel.

### Pitfall 2: Multiple Tooltips Visible Simultaneously

**What goes wrong:** Moving quickly from button A to button B (both with 800ms delay started) shows both tooltips if the first timer fires before the mouse leaves.

**Why it happens:** Each `Tooltip` instance manages its own `useState`. Without a global singleton, both can be `visible: true` simultaneously.

**How to avoid:** The 800ms delay is long enough that fast mouse movement will cancel the first timer before it fires. For extra safety, the `mouseenter` handler calls `hide()` implicitly (clearing any pending timer) before starting a new one — the two calls are on different `Tooltip` instances so this requires a global signal OR relying on the delay being long enough. Given 800ms, simultaneous tooltips are practically impossible under normal use. No global singleton needed unless QA finds a regression.

**Warning signs:** Two tooltip boxes visible at the same time during automated testing.

### Pitfall 3: Native `title` Attribute Still Present Causes Double Tooltip

**What goes wrong:** A `title=` attribute left on a `<button>` causes the browser to show the native OS tooltip 1–2 seconds after hover, in addition to the custom one.

**Why it happens:** Browser tooltip behaviour cannot be suppressed without removing the `title` attribute from the DOM element entirely.

**How to avoid:** When `Tooltip` wraps a `<button>`, ensure the native `title` attribute is removed from `<button>`. In `ToolBtn`, the native `title={title}` prop on `<button>` must be deleted. Verify with devtools that no `title` attribute appears in the DOM on interactive elements.

**Warning signs:** After ~1s hover, a system tooltip box appears below the custom one.

### Pitfall 4: Breadcrumb Flicker on Every Keystroke

**What goes wrong:** `findEnclosingFunction` runs on every content change (every keypress), causing the breadcrumb to re-render constantly even when cursor line hasn't changed.

**Why it happens:** `BreadcrumbBar` subscribes to `tabs` (which changes on every keypress via `updateContent`).

**How to avoid:** Subscribe to `cursorLine` and `activeTabId` separately, then access the tab content via `useStore.getState().tabs.find(...)` inside a `useMemo` keyed on both. Or subscribe to a derived `activeTabContent` selector. The scan itself is O(n lines from cursor to start) and fast, but unnecessary re-renders should be avoided.

**Better pattern:**
```typescript
const cursorLine = useStore((s) => s.cursorLine);
const activeTabId = useStore((s) => s.activeTabId);
// Read content imperatively — not as a subscription
const activeTab = useStore((s) => s.tabs.find((t) => t.scriptId === s.activeTabId));
```
Zustand re-renders only when the returned value changes (by reference for objects). Using `.find()` inside the selector creates a new object reference on every render. Use `useMemo` or separate selectors to avoid excessive re-renders.

**Recommended pattern:**
```typescript
const cursorLine = useStore((s) => s.cursorLine);
const activeTabContent = useStore((s) => {
  const t = s.tabs.find((tab) => tab.scriptId === s.activeTabId);
  return t?.content ?? '';
});
const activeTabName = useStore((s) => {
  const t = s.tabs.find((tab) => tab.scriptId === s.activeTabId);
  return t?.name ?? null;
});
const fnName = useMemo(
  () => findEnclosingFunction(activeTabContent, cursorLine),
  [activeTabContent, cursorLine]
);
```

### Pitfall 5: Breadcrumb Bar Compresses Editor Height

**What goes wrong:** Adding a 24px `BreadcrumbBar` inside the `flex-direction: column` center layout causes the editor to shrink by 24px, but more critically causes overflow if the parent `editorArea` is already `flex: 1 min-height: 0`.

**Why it happens:** `flex-shrink` defaults to 1 on all flex children. If `BreadcrumbBar` doesn't declare `flex-shrink: 0`, it compresses with the layout.

**How to avoid:** Add `flex-shrink: 0` to `.bar` in `BreadcrumbBar.module.css`. The editor `Panel` inside the vertical `Group` will absorb the remaining space correctly.

**Warning signs:** Breadcrumb bar appears to collapse to 0 height, or editor is shorter than expected.

### Pitfall 6: wry Token Inline Requirement

**What goes wrong:** New CSS custom properties defined only in `tokens.css` are not available in wry production build.

**Why it happens:** wry WKWebView does not apply CSS `var()` from external stylesheets when using the custom protocol. All `:root` token definitions must be inlined in `index.html`.

**How to avoid:** If Phase 9 needs new tokens (it does not — `#3C3F41`, `#BBB`, `#555` are hardcoded per spec), add them to both `tokens.css` AND the inline `<style>` block in `index.html`. The Phase 9 tooltip colors are locked values (`#3C3F41`, `#BBB`, `#555`) and can be hardcoded directly in the CSS Module rather than using tokens — this avoids the wry sync requirement entirely.

---

## Code Examples

Verified patterns from existing codebase:

### Existing updateListener pattern (cursor position — already in Editor.tsx)

```typescript
// Source: editor-ui/src/components/Editor.tsx line 184-190
if (update.selectionSet) {
  const pos = update.state.selection.main.head;
  const line = update.state.doc.lineAt(pos);
  useStore.getState().setCursor(line.number, pos - line.from + 1);
}
```

The `cursorLine` store field is populated on every cursor move. BreadcrumbBar reads this directly — no changes to `Editor.tsx` needed.

### Existing ToolStrip shortcut title pattern (already in ToolStrip.tsx)

```typescript
// Source: editor-ui/src/components/ToolStrip.tsx line 28
title={`${item.label}${item.shortcut ? ` (${item.shortcut})` : ''}`}
```

ToolStrip already formats `"Label (Shortcut)"`. The ToolBtn tooltip should use the same format.

### Existing flex-shrink: 0 pattern (from Phase 6 StatusBar)

```css
/* Source: editor-ui/src/components/StatusBar.module.css */
.bar {
  flex-shrink: 0;
  height: var(--height-statusbar); /* 24px */
}
```

BreadcrumbBar.module.css should follow the same pattern.

### Backward line scan for def blocks

```typescript
// VoidScript def regex — matches Python-style function definitions
// Source: derived from voidscript-lang.ts keyword set (def is a keyword)
const DEF_RE = /^\s*def\s+([a-zA-Z_]\w*)\s*\(/;

function findEnclosingFunction(content: string, cursorLine: number): string | null {
  if (!content || cursorLine < 1) return null;
  const lines = content.split('\n');
  // cursorLine is 1-based; array is 0-based
  const startIdx = Math.min(cursorLine - 1, lines.length - 1);
  for (let i = startIdx; i >= 0; i--) {
    const m = DEF_RE.exec(lines[i]);
    if (m) return m[1];
  }
  return null;
}
```

---

## Complete `title=` Audit

Every site that must be migrated from native `title=` to `<Tooltip>`:

| File | Line | Current value | After migration |
|------|------|---------------|----------------|
| `Header.tsx` | 24 | `title="Menu"` | ToolBtn wraps Tooltip |
| `Header.tsx` | 35 | `title="Back"` | ToolBtn wraps Tooltip |
| `Header.tsx` | 38 | `title="Forward"` | ToolBtn wraps Tooltip |
| `Header.tsx` | 74 | `title="Run"` | ToolBtn wraps Tooltip, add shortcut="Shift+F10" |
| `Header.tsx` | 86 | `title="Debug"` | ToolBtn wraps Tooltip, add shortcut="Shift+F9" |
| `Header.tsx` | 107 | `title="Stop"` | ToolBtn wraps Tooltip, add shortcut="Ctrl+F2" |
| `Header.tsx` | 121 | `title="Resume"` | ToolBtn wraps Tooltip, add shortcut="F9" |
| `Header.tsx` | 129 | `title="Step Over"` | ToolBtn wraps Tooltip, add shortcut="F8" |
| `Header.tsx` | 132 | `title="Step Into"` | ToolBtn wraps Tooltip, add shortcut="F7" |
| `Header.tsx` | 135 | `title="Step Out"` | ToolBtn wraps Tooltip, add shortcut="Shift+F8" |
| `Header.tsx` | 145 | `title="Settings"` | ToolBtn wraps Tooltip |
| `Header.tsx` | 165 | `title="Close"` | TrafficLight → `<Tooltip content={title}>` |
| `Header.tsx` | 171 | `title="Minimize"` | TrafficLight → `<Tooltip content={title}>` |
| `Header.tsx` | 177 | `title="Maximize"` | TrafficLight → `<Tooltip content={title}>` |
| `Header.tsx` | 250 | `title="Search Everywhere (Shift Shift)"` | SearchPill → `<Tooltip content="Search Everywhere (Shift Shift)">`, remove native title |
| `ScriptList.tsx` | 23 | `title="Add Script"` | ToolBtn wraps Tooltip |
| `ScriptList.tsx` | 28 | `title="Hide"` | ToolBtn wraps Tooltip |
| `DebugPanel.tsx` | 17 | `title="Hide"` | ToolBtn wraps Tooltip |
| `DebugPanel.tsx` | 46 | `title={v.value}` | variable value — truncation tooltip; wrap span with `<Tooltip>` |
| `BottomTabStrip.tsx` | 29 | `title="Clear Console"` | ToolBtn wraps Tooltip |
| `BottomTabStrip.tsx` | 34 | `title="Close"` | ToolBtn wraps Tooltip |
| `ToolStrip.tsx` | 28 | `title={...}` | ToolBtn wraps Tooltip (automatically fixed by ToolBtn change) |

Note: `PanelHeader title=` prop is a section heading label, not a DOM `title` attribute — no migration needed there.

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Native browser `title` attribute | Custom CSS-positioned tooltip component | Phase 9 | Consistent Rider-dark styling; no OS-native tooltip appearance |
| No breadcrumb bar | BreadcrumbBar below TabBar | Phase 9 | EDIT-01 satisfied |

**Deprecated/outdated in this codebase:**
- Native `title` attribute on interactive elements: removed entirely across all components in this phase.

---

## Open Questions

1. **`display: contents` vs `display: inline-flex` for Tooltip wrapper**
   - What we know: `display: contents` breaks `position: relative` needed for tooltip anchor
   - What's unclear: Whether `display: inline-flex` on the wrapper disrupts ToolBtn's existing layout in the toolbar flex container
   - Recommendation: Use `display: inline-flex` with `position: relative`. If it causes layout issues (e.g. ToolBtn width changes), fall back to `display: block` or `display: contents` with a React portal approach. Test immediately in dev.

2. **Breadcrumb re-render frequency on fast typing**
   - What we know: `findEnclosingFunction` runs on content change AND cursor line change; content changes every keypress
   - What's unclear: Whether `useMemo` on `[activeTabContent, cursorLine]` is sufficient, or if the Zustand selector pattern creates unwanted re-renders
   - Recommendation: Use per-field selectors (`cursorLine`, `activeTabContent`, `activeTabName` as separate `useStore` calls) + `useMemo` for the derived function name. If still noisy, add a 50ms debounce on the content selector (acceptable since breadcrumb doesn't need keystroke-level update frequency).

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | None detected — no test config or test files in project |
| Config file | None — Wave 0 must create if tests are needed |
| Quick run command | N/A |
| Full suite command | N/A |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PLSH-03 | Custom tooltip shows on hover, native title absent | manual-only | — | ❌ no test infrastructure |
| PLSH-04 | Shortcut hint appears in tooltip text | manual-only | — | ❌ no test infrastructure |
| EDIT-01 | Breadcrumb updates on cursor move; `def` block detected | manual-only | — | ❌ no test infrastructure |

The `findEnclosingFunction` utility function is pure and easily unit-testable. However, since the project has no test infrastructure at all, and this is a visual polish phase, all validation is manual via the running wry application.

**Manual verification checklist (for verify-work phase):**
- Hover any toolbar button for 800ms — custom styled tooltip appears, no native OS tooltip shows
- Hover Run button — tooltip reads "Run (Shift+F10)"
- Move mouse quickly between buttons — only one tooltip visible at a time
- Open a script with `def` blocks, move cursor inside one — breadcrumb shows filename › function_name
- Move cursor to top level — breadcrumb shows just filename
- Resize window so tooltip would clip bottom — tooltip flips above trigger

### Wave 0 Gaps

None — no test infrastructure needed for this phase. All validation is visual/interactive.

---

## Sources

### Primary (HIGH confidence)

- Direct codebase read: `editor-ui/src/primitives/ToolBtn.tsx` — current title= usage pattern
- Direct codebase read: `editor-ui/src/components/Editor.tsx` — updateListener cursor tracking at line 184-190
- Direct codebase read: `editor-ui/src/components/Header.tsx` — complete title= audit
- Direct codebase read: `editor-ui/src/codemirror/voidscript-lang.ts` — StreamLanguage confirmation, `def` keyword confirmed
- Direct codebase read: `editor-ui/src/state/store.ts` — `cursorLine`, `updateContent`, tab structure
- Direct codebase read: `editor-ui/src/App.tsx` — layout slot for BreadcrumbBar confirmed
- Direct codebase read: `editor-ui/src/theme/tokens.css` — existing token values; `--bg-tooltip: #393B40` exists but Phase 9 uses `#3C3F41` per locked spec
- Direct codebase read: `editor-ui/src/components/NavPath.module.css` — segment/chevron styling to reuse in breadcrumb

### Secondary (MEDIUM confidence)

- `.planning/phases/09-polish-and-tooltips/09-CONTEXT.md` — locked design decisions (treated as HIGH for this project)
- `.planning/STATE.md` — confirmed wry token inline requirement (Phase 01-foundation P02 decision)

### Tertiary (LOW confidence)

None — all findings are directly from codebase inspection.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new packages, all existing
- Architecture: HIGH — derived directly from existing code patterns
- Pitfalls: HIGH — several derived from prior phase decisions (flex-shrink, wry tokens, display:contents)

**Research date:** 2026-03-15
**Valid until:** 2026-04-15 (stable React/CSS domain)
