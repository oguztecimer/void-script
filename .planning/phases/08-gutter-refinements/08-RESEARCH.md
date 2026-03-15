# Phase 8: Gutter Refinements - Research

**Researched:** 2026-03-15
**Domain:** CodeMirror 6 gutter API — GutterMarker, lineNumberMarkers facet, foldGutter configuration
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **Breakpoint overlay appearance:** Filled red circle (~12px diameter), completely replaces the line number — number vanishes, red circle sits in its place. Uses `--accent-breakpoint` CSS token. No line number reveal on hover; adjacent numbers make position clear.
- **Fold icon hover behavior:** Fold icons hidden by default (`opacity: 0`), appear on hover anywhere on the gutter row. Fixed-width fold gutter column (~14px) always reserves space to prevent width jumping.
- **Fold icon style:** Rider-style filled triangles — `▶` (right-pointing) for collapsed, `▼` (down-pointing) for expanded. Color: `--text-tertiary`, brightens on hover.
- **Gutter click targets:** Clicking anywhere in the line-number gutter column toggles a breakpoint. Faint/translucent red circle preview appears on hover to indicate interactivity.
- **Breakpoint animation:** 150ms fade (consistent with established `--transition-hover` app-wide pattern).
- **Active line + breakpoint:** Highlight background shows, red circle layered on top, line number hidden. Breakpoint always wins.
- **All `.cm-*` style overrides:** Must live inside `EditorView.theme()` in `voidscript-theme.ts`. No `.cm-*` rules in any external CSS file.

### Claude's Discretion

- Exact implementation approach for combining breakpoint + line number into a single custom gutter (custom GutterMarker subclass vs `lineNumbers()` configuration with `lineNumberMarkers` facet)
- How to wire gutter row hover detection for fold icon visibility (CSS `:hover` on gutter row vs `EditorView.domEventHandlers`)
- Internal CodeMirror extension composition (ordering of gutter extensions, StateField design)
- Exact opacity values for the faint breakpoint hover preview

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| EDIT-02 | Fold gutter icons visible on hover only (hidden by default) | `foldGutter({ markerDOM })` + CSS `opacity: 0` with `.cm-foldGutter:hover span` targeting inside `EditorView.theme()` |
| EDIT-03 | Breakpoint markers overlay line numbers in a single combined gutter (remove separate breakpoint column) | `lineNumberMarkers` facet injects a custom GutterMarker with `toDOM`; the built-in `lineMarker` callback returns `null` when any marker with `toDOM` is present — suppressing the line number automatically |
</phase_requirements>

---

## Summary

Phase 8 requires three distinct gutter changes inside CodeMirror 6: (1) remove the separate breakpoint gutter column and merge breakpoints into the line-number column via the `lineNumberMarkers` facet, (2) configure `foldGutter()` to show custom triangle markers that are hidden by default and appear on gutter-row hover via CSS, and (3) ensure all `.cm-*` styling lives exclusively in `EditorView.theme()`.

The CodeMirror 6 source (v6.40.0, confirmed from installed `node_modules`) reveals the exact mechanism for breakpoint overlay: `lineNumberGutter.lineMarker` already returns `null` when `others.some(m => m.toDOM)` is true. This means any `GutterMarker` with a `toDOM` implementation provided via the `lineNumberMarkers` facet will automatically suppress the default line number for that row — no custom gutter needed.

For fold icon hover, the `foldGutter({ markerDOM })` callback creates a `<span>` element; that element can be targeted by CSS inside `EditorView.theme()` using `.cm-foldGutter .cm-gutterElement:hover` or the parent `.cm-gutters:hover .cm-foldGutter .cm-gutterElement`. The fixed-width column reservation prevents gutter width jumping on hover via `opacity: 0` / `opacity: 1` rather than `display: none`.

**Primary recommendation:** Use `lineNumberMarkers` facet (not a second custom `gutter()`) for breakpoint overlay; use `foldGutter({ markerDOM })` with CSS opacity transitions for fold icon hover. Remove `createBreakpointGutter()` and its separate `gutter()` call entirely.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `@codemirror/view` | 6.40.0 (installed) | `GutterMarker`, `lineNumbers`, `lineNumberMarkers`, `gutter` | Official CM6 view package; all gutter primitives live here |
| `@codemirror/language` | 6.12.2 (installed) | `foldGutter`, `FoldGutterConfig`, `markerDOM` callback | Official CM6 language package; fold gutter extension |
| `@codemirror/state` | 6.5.x (installed) | `StateField`, `StateEffect`, `RangeSet`, `RangeSetBuilder` | Official CM6 state package; breakpoint state management |

### Supporting

No new packages needed. All required APIs are already installed.

**Installation:**
```bash
# No new dependencies — all existing
```

---

## Architecture Patterns

### Recommended Project Structure

No structural changes needed. All changes are confined to:

```
editor-ui/src/
├── codemirror/
│   └── voidscript-theme.ts   # Add gutter hover CSS rules inside EditorView.theme()
└── components/
    └── Editor.tsx             # Replace createBreakpointGutter() + lineNumbers() with
                               # lineNumberMarkers facet approach; configure foldGutter()
```

### Pattern 1: Breakpoint Overlay via `lineNumberMarkers` Facet

**What:** The built-in `lineNumberGutter` suppresses the default `NumberMarker` whenever any external marker with a `toDOM` method is present for that line. Injecting a breakpoint marker via `lineNumberMarkers` facet achieves the overlay with zero additional gutter columns.

**When to use:** Whenever you want custom content to replace (not augment) a line number.

**How it works (from source, `@codemirror/view` v6.40.0):**
```typescript
// Source: node_modules/@codemirror/view/dist/index.js line 11602
lineMarker(view, line, others) {
    if (others.some(m => m.toDOM))
        return null;   // <-- suppresses NumberMarker when any marker has toDOM
    return new NumberMarker(formatNumber(...));
}
```

**Implementation pattern:**
```typescript
// Source: @codemirror/view lineNumberMarkers facet (declared at index.d.ts:2346)
import { lineNumberMarkers } from '@codemirror/view';

class BreakpointOverlayMarker extends GutterMarker {
  toDOM(): HTMLElement {
    const el = document.createElement('div');
    el.className = 'cm-breakpoint-circle'; // styled in EditorView.theme()
    return el;
  }
  eq(other: GutterMarker): boolean {
    return other instanceof BreakpointOverlayMarker;
  }
}

// Compute a RangeSet from breakpointState field, feed via lineNumberMarkers facet
const breakpointOverlayExtension = EditorView.decorations.from(
  // OR: use a ViewPlugin that reads breakpointState and provides lineNumberMarkers
);
```

The cleanest approach: a `ViewPlugin` that reads `breakpointState` and returns a `RangeSet<GutterMarker>` through `lineNumberMarkers.of(...)`:

```typescript
// Derived extension — reads breakpointState, provides overlay markers to line-number gutter
const breakpointOverlay = ViewPlugin.fromClass(
  class {
    markers: RangeSet<GutterMarker>;
    constructor(view: EditorView) {
      this.markers = view.state.field(breakpointState);
    }
    update(update: ViewUpdate) {
      if (update.docChanged || update.transactions.some(t =>
        t.effects.some(e => e.is(toggleBreakpointEffect))
      )) {
        // Map old RangeSet to new markers using the BreakpointOverlayMarker
        const set = update.state.field(breakpointState);
        const builder = new RangeSetBuilder<GutterMarker>();
        const iter = set.iter();
        while (iter.value) {
          builder.add(iter.from, iter.from, new BreakpointOverlayMarker());
          iter.next();
        }
        this.markers = builder.finish();
      }
    }
  },
  { provide: plugin => lineNumberMarkers.of(view => view.plugin(plugin)?.markers ?? RangeSet.empty) }
);
```

**Simpler alternative** — because `breakpointState` already holds a `RangeSet<GutterMarker>`, a static `lineNumberMarkers.of(view => view.state.field(breakpointState))` would work if `BreakpointOverlayMarker` extends `GutterMarker` with `toDOM` (the existing `BreakpointMarker` already has `toDOM`). The key is that the `RangeSet` provided to `lineNumberMarkers` must contain markers with `toDOM` methods.

**Simplest implementation:**

```typescript
// breakpointState already contains GutterMarkers with toDOM.
// lineNumberMarkers.of() feeds them into the line-number gutter.
// The line-number gutter suppresses NumberMarker when others.some(m => m.toDOM).
const breakpointLineNumberMarkers = lineNumberMarkers.of(
  (view: EditorView) => view.state.field(breakpointState)
);
```

This is the recommended approach — minimal code, uses existing `breakpointState` directly.

### Pattern 2: `foldGutter` with Custom `markerDOM` and CSS Hover

**What:** Replace default text markers with custom DOM elements; hide them by default via CSS `opacity: 0`; reveal on gutter-row hover with `opacity: 1` and `transition`.

**Fold gutter config API (from `@codemirror/language` v6.12.2 index.d.ts:796-822):**
```typescript
interface FoldGutterConfig {
  markerDOM?: ((open: boolean) => HTMLElement) | null;
  openText?: string;      // default "⌄"
  closedText?: string;    // default "›"
  domEventHandlers?: Handlers;
  foldingChanged?: (update: ViewUpdate) => boolean;
}
```

**Implementation pattern:**
```typescript
foldGutter({
  markerDOM(open: boolean): HTMLElement {
    const span = document.createElement('span');
    span.className = 'cm-fold-marker';
    span.textContent = open ? '▼' : '▶';
    return span;
  },
})
```

**CSS inside `EditorView.theme()` for hover reveal (fixed-width column, opacity transition):**
```typescript
EditorView.theme({
  // ... existing rules ...
  '.cm-foldGutter': {
    width: '14px',
    minWidth: '14px',
  },
  '.cm-foldGutter .cm-gutterElement': {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
  },
  '.cm-foldGutter .cm-fold-marker': {
    opacity: '0',
    color: 'var(--text-tertiary)',
    fontSize: '10px',
    cursor: 'pointer',
    transition: 'opacity 150ms ease, color 150ms ease',
  },
  // Hover on the gutter row (cm-gutterElement) reveals the icon
  '.cm-foldGutter .cm-gutterElement:hover .cm-fold-marker': {
    opacity: '1',
  },
  // Brighter on direct icon hover
  '.cm-foldGutter .cm-gutterElement:hover .cm-fold-marker:hover': {
    color: 'var(--text-secondary)',
  },
})
```

**Note on hover scope decision (Claude's discretion):** CSS `:hover` on `.cm-gutterElement` (the individual gutter cell div) triggers when the mouse is anywhere in that cell row for the fold gutter column. This is simpler than wiring `EditorView.domEventHandlers` and has zero JS overhead. The CONTEXT.md says "hover anywhere on the gutter row" — for the fold gutter column itself, `.cm-gutterElement:hover` achieves this. CSS `:hover` is the correct approach.

### Pattern 3: Breakpoint Hover Preview (Faint Circle on Hover)

The line-number gutter `domEventHandlers` option in `lineNumbers()` allows mouseenter/mouseleave events. A simpler approach: use CSS `:hover` on `.cm-lineNumbers .cm-gutterElement` to show a pseudo-element circle.

```typescript
// In EditorView.theme():
'.cm-lineNumbers .cm-gutterElement': {
  position: 'relative',
  cursor: 'pointer',
  // Remove default padding/text-align overrides only if needed
},
'.cm-lineNumbers .cm-gutterElement::after': {
  content: '""',
  position: 'absolute',
  inset: '0',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  // Hidden by default
  opacity: '0',
  transition: 'opacity 150ms ease',
},
'.cm-lineNumbers .cm-gutterElement:hover::after': {
  opacity: '1',
  // Faint circle background via radial-gradient or border-radius
  background: 'radial-gradient(circle, var(--accent-breakpoint) 40%, transparent 41%)',
  // Sized to match breakpoint circle ~12px in center
},
```

**Alternative:** Add `mouseenter`/`mouseleave` handlers on the line-number gutter's `domEventHandlers` to apply/remove a CSS class to the gutter element, then style that class in `EditorView.theme()`. This is more controllable but requires JS event management. Given EDIT-03 (all `.cm-*` rules in `EditorView.theme()`), CSS pseudo-element approach keeps all styling in one place and is recommended.

### Pattern 4: Breakpoint Circle Styling

```typescript
// In EditorView.theme():
'.cm-breakpoint-circle': {
  width: '12px',
  height: '12px',
  borderRadius: '50%',
  backgroundColor: 'var(--accent-breakpoint)',
  margin: 'auto',
  opacity: '0',
  transition: 'opacity 150ms ease',
},
'.cm-breakpoint-circle.cm-bp-active': {
  opacity: '1',
},
```

**OR** — if the `BreakpointOverlayMarker.toDOM()` directly creates a styled div, the CSS can be in `EditorView.theme()` targeting that class.

### Anti-Patterns to Avoid

- **Separate breakpoint gutter column:** The old `createBreakpointGutter()` + `gutter({ class: 'cm-breakpoint-gutter' })` creates a separate visual column. Remove entirely in this phase.
- **External CSS `.cm-*` rules:** EDIT-03 requires zero `.cm-*` rules in `.css`/`.module.css` files. Confirmed: currently none exist — maintain this invariant.
- **`display: none` for fold icons:** Causes gutter width to collapse on rows without fold markers (no icon present = no width). Use `opacity: 0` on an element that always exists in the DOM.
- **`initialSpacer` omission on fixed-width gutters:** Without `initialSpacer`, the fold gutter has 0 width when no foldable lines are in view. The `foldGutter()` already provides `initialSpacer`; the 14px fixed width via CSS is the reliable approach.
- **Mutating gutter DOM directly:** Never mutate `.cm-gutterElement` children outside of `GutterMarker.toDOM()`/`GutterMarker.destroy()`. CodeMirror owns that DOM.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Line number suppression for breakpoint rows | Custom gutter replacing `lineNumbers()` entirely | `lineNumberMarkers` facet with a `GutterMarker` that has `toDOM` | CM6 built-in: `lineMarker` returns `null` automatically when `others.some(m => m.toDOM)` |
| Fold icon state (foldable vs folded) | Manual syntax tree traversal | `foldGutter()` with `markerDOM` config | Built-in already tracks foldable/folded state per viewport line |
| Gutter row hover detection | `EditorView.domEventHandlers` + JS state | CSS `:hover` on `.cm-gutterElement` | Zero JS; works perfectly for opacity transitions |
| RangeSet management | Custom Map/Set of breakpoint positions | `breakpointState` `StateField` (already implemented) | Already handles doc change mapping correctly |

**Key insight:** CodeMirror 6's gutter system has built-in priority and suppression logic. The `lineNumberMarkers` facet is the designed extension point for injecting markers into the line-number column without replacing the whole gutter.

---

## Common Pitfalls

### Pitfall 1: `lineNumberMarkers` Provides Markers But `lineMarker` Still Runs

**What goes wrong:** A `GutterMarker` is provided via `lineNumberMarkers` but it does NOT define `toDOM` (only `elementClass`). The `lineMarker` callback checks `others.some(m => m.toDOM)` — if that's false, the line number is NOT suppressed. Both the marker and the number appear.

**Why it happens:** Markers without `toDOM` are used only for CSS class application (via `elementClass`). They don't count as "replacing" the line number.

**How to avoid:** Ensure `BreakpointOverlayMarker.toDOM()` is defined (not inherited from `GutterMarker` as abstract/undefined). The existing `BreakpointMarker` already implements `toDOM()` — preserve this.

**Warning signs:** Two elements visible in the line-number cell — the `●` character AND the line number. Check DOM in devtools.

### Pitfall 2: Fold Gutter Column Width Collapse

**What goes wrong:** `foldGutter()` with no `initialSpacer` override AND no explicit CSS width on `.cm-foldGutter` — when the viewport contains no foldable lines, the gutter renders no elements, has no spacer, collapses to 0px wide. The gutter then jumps in width when scrolling into foldable regions.

**Why it happens:** CM6 gutters take their width from the widest rendered element. If nothing is rendered (no foldable lines in viewport), width = 0.

**How to avoid:** Set explicit `width: '14px'` (or `min-width: '14px'`) on `.cm-foldGutter` inside `EditorView.theme()`. The `foldGutter()` built-in already provides an `initialSpacer` so width should be stable, but explicit CSS is safer.

**Warning signs:** Gutter flickers or shifts width when scrolling.

### Pitfall 3: CSS `:hover` Selector Specificity in `EditorView.theme()`

**What goes wrong:** Writing `.cm-foldGutter:hover .cm-fold-marker` — hovering the gutter column div instead of the individual row cell. The `.cm-foldGutter` element is the entire column; hovering anywhere in the editor would trigger it.

**Why it happens:** Confusing `.cm-foldGutter` (entire column div) with `.cm-gutterElement` (individual row cell).

**How to avoid:** Target `.cm-foldGutter .cm-gutterElement:hover .cm-fold-marker` — this scopes hover to the specific row cell within the fold gutter column.

**Warning signs:** All fold icons appear when hovering anywhere in the gutter column, not just the hovered row.

### Pitfall 4: `EditorView.theme()` Selector Scoping

**What goes wrong:** CSS rule `.cm-lineNumbers .cm-gutterElement:hover::after` doesn't fire. The `EditorView.theme()` scoping prepends `&` (the editor root) to selectors that don't start with `.`. Class selectors (`.cm-*`) are used as-is within the editor root scope.

**Why it happens:** `EditorView.theme()` uses CSS Modules-style scoping. Selectors work as expected for class-based rules.

**How to avoid:** Test in browser devtools: inspect the rendered `.cm-gutterElement` to confirm hover styles apply. Verify the `EditorView.theme()` object key matches exactly (no leading `&` for class selectors).

**Warning signs:** Hover styles never appear despite correct-looking selectors.

### Pitfall 5: Removing `createBreakpointGutter` Without Removing Its `initialSpacer`

**What goes wrong:** The old `createBreakpointGutter()` used `initialSpacer: () => breakpointMarker` to size the column. If `createBreakpointGutter` is removed but `breakpointState` is kept (it is — reused as-is), the `lineNumberMarkers` facet approach doesn't need a spacer since it rides on the existing line-number gutter.

**How to avoid:** Remove both `createBreakpointGutter(scriptId)` from `buildExtensions()` and the `gutter()` call. Keep `breakpointState` and `toggleBreakpointEffect`. Update `BreakpointMarker.toDOM()` to render the circle div (not the `●` character).

---

## Code Examples

Verified patterns from source inspection of `@codemirror/view` v6.40.0 and `@codemirror/language` v6.12.2:

### Complete Breakpoint Overlay Extension

```typescript
// Source: @codemirror/view lineNumberMarkers facet + lineMarker suppression logic
import {
  GutterMarker,
  lineNumberMarkers,
} from '@codemirror/view';
import { RangeSet } from '@codemirror/state';

class BreakpointOverlayMarker extends GutterMarker {
  // eq() is critical — prevents unnecessary re-renders
  eq(other: GutterMarker): boolean {
    return other instanceof BreakpointOverlayMarker;
  }
  toDOM(): HTMLElement {
    const el = document.createElement('div');
    el.className = 'cm-bp-circle';
    return el;
  }
}

const breakpointOverlayMarker = new BreakpointOverlayMarker();

// Re-map breakpointState RangeSet to use BreakpointOverlayMarker (has toDOM)
// breakpointState already stores GutterMarker — but we need BreakpointOverlayMarker specifically
// Simplest: store BreakpointOverlayMarker in breakpointState directly (change breakpointMarker const)

// OR: provide via lineNumberMarkers facet using existing breakpointState
// This works because BreakpointOverlayMarker has toDOM, suppressing NumberMarker
const bpLineMarkers = lineNumberMarkers.of(
  (view) => {
    // Convert breakpointState (which has BreakpointMarker instances) to BreakpointOverlayMarker instances
    // If breakpointState uses BreakpointOverlayMarker directly, no conversion needed
    return view.state.field(breakpointState);
  }
);
```

### `foldGutter` Configuration with Custom Markers

```typescript
// Source: @codemirror/language FoldGutterConfig.markerDOM (index.d.ts:802)
foldGutter({
  markerDOM(open: boolean): HTMLElement {
    const span = document.createElement('span');
    span.className = 'cm-fold-marker';
    // Rider-style: ▼ for expanded (can fold), ▶ for collapsed (can unfold)
    span.textContent = open ? '▼' : '▶';
    return span;
  },
})
```

### `EditorView.theme()` Additions for Gutter Styles

```typescript
// Source: existing voidscript-theme.ts — extend EditorView.theme() object
EditorView.theme({
  // ... existing rules preserved ...

  // ── Fold gutter ──────────────────────────────────────
  '.cm-foldGutter': {
    width: '14px',
    minWidth: '14px',
  },
  '.cm-foldGutter .cm-gutterElement': {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    padding: '0',
  },
  '.cm-foldGutter .cm-fold-marker': {
    opacity: '0',
    color: 'var(--text-tertiary)',
    fontSize: '10px',
    lineHeight: '1',
    cursor: 'pointer',
    transition: 'opacity var(--transition-hover), color var(--transition-hover)',
    userSelect: 'none',
  },
  '.cm-foldGutter .cm-gutterElement:hover .cm-fold-marker': {
    opacity: '1',
  },
  '.cm-foldGutter .cm-gutterElement:hover .cm-fold-marker:hover': {
    color: 'var(--text-secondary)',
  },

  // ── Breakpoint overlay circle ─────────────────────────
  '.cm-lineNumbers .cm-gutterElement': {
    position: 'relative',
    cursor: 'pointer',
    // Retain existing padding from base theme: "0 3px 0 5px"
    // Override needed only if centering the circle requires it
  },
  '.cm-bp-circle': {
    width: '12px',
    height: '12px',
    borderRadius: '50%',
    backgroundColor: 'var(--accent-breakpoint)',
    margin: '0 auto',
    // 150ms fade matches app-wide --transition-hover
    opacity: '1',  // visible when element exists (marker only added when breakpoint set)
    // If using CSS transition on add/remove: opacity starts at 0, JS adds class
    // Simpler: GutterMarker presence = visible; opacity:1 on element itself
  },

  // ── Breakpoint hover preview (faint circle) ───────────
  // Shows a translucent red circle on hover to indicate clickability
  '.cm-lineNumbers .cm-gutterElement:hover': {
    // Slight tint to indicate interactivity — exact opacity is Claude's discretion
  },
}, { dark: true })
```

**Note on breakpoint transition:** CodeMirror adds/removes the `BreakpointOverlayMarker` by calling `GutterMarker.destroy(dom)` and `GutterMarker.toDOM()`. CSS `transition` on the circle element won't fire on element creation/removal (no CSS animation triggers). To achieve 150ms fade on toggle, use CSS `@keyframes` fade-in on the `.cm-bp-circle` element, or use `opacity` + a separate `StateField`-driven CSS class approach. The simplest production-grade solution: CSS `animation: bp-appear 150ms ease` applied to `.cm-bp-circle`.

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Separate breakpoint gutter column (`gutter()` call) | `lineNumberMarkers` facet injection into existing line-number gutter | This phase | Removes extra column; breakpoint overlays line number |
| Default fold text markers (`⌄` / `›`) | Custom `markerDOM` triangle markers | This phase | Rider-style triangles, opacity-based hover reveal |
| Visible fold icons always | `opacity: 0` default, `opacity: 1` on hover | This phase | Hover-only visibility matches Rider |

**Deprecated in this phase:**
- `createBreakpointGutter()` function: replaced by `lineNumberMarkers` facet
- `class: 'cm-breakpoint-gutter'` gutter extension: removed from `buildExtensions()`
- `BreakpointMarker` rendering `●` character: replaced by `BreakpointOverlayMarker` rendering a styled `div.cm-bp-circle`

---

## Open Questions

1. **Breakpoint fade animation on toggle**
   - What we know: CSS `transition` doesn't fire on element creation (toDOM) or removal (destroy). The 150ms fade-on-toggle requirement conflicts with how `GutterMarker` DOM is managed.
   - What's unclear: Whether `GutterMarker.destroy(dom)` is called synchronously or with a delay that could be exploited.
   - Recommendation: Use CSS `@keyframes` fade-in for appearance. For fade-out (disappearance), either accept instant removal OR implement a `StateField`-driven pending-removal state that keeps the circle visible for 150ms before removing the marker from the `RangeSet`. The former is simpler and acceptable for v1.

2. **Hover preview circle for breakpoints**
   - What we know: CSS `::before`/`::after` pseudo-elements on `.cm-lineNumbers .cm-gutterElement` can show a faint circle on hover.
   - What's unclear: The exact opacity/color for "faint but visible" preview — user said this is Claude's discretion.
   - Recommendation: Use `rgba(248, 81, 73, 0.25)` (25% opacity of `--accent-breakpoint: #f85149`) as the hover preview color. This provides clear affordance without competing with set breakpoints.

3. **`lineNumbers()` `domEventHandlers` vs separate `mousedown` handler**
   - What we know: The existing breakpoint toggle uses `gutter({ domEventHandlers: { mousedown } })`. Moving to `lineNumberMarkers` facet, click handling must move to `lineNumbers({ domEventHandlers: { mousedown } })`.
   - What's unclear: Whether `lineNumbers()` `domEventHandlers.mousedown` receives `line: BlockInfo` with the same shape as the old `gutter()` handler.
   - Recommendation: From `@codemirror/view` source, `LineNumberConfig.domEventHandlers` has signature `(view: EditorView, line: BlockInfo, event: MouseEvent) => boolean`. Same shape. Direct migration is safe.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | None installed |
| Config file | None — Wave 0 gap |
| Quick run command | `npm run build` (TypeScript compilation as proxy for correctness) |
| Full suite command | Manual visual inspection in running app |

**No automated test infrastructure exists in this project.** TypeScript compilation (`tsc && vite build`) serves as the primary automated correctness check. Visual regression is manual.

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| EDIT-02 | Fold icons hidden by default, visible on hover | manual-only | `npm run build` (compile check) | N/A |
| EDIT-03 | Breakpoints overlay line numbers, no separate column | manual-only | `npm run build` (compile check) | N/A |
| EDIT-03 | No `.cm-*` rules in external CSS files | automated | `grep -r "\.cm-" src --include="*.css"` (should return empty) | N/A |

**Manual-only justification:** Gutter rendering requires a live CodeMirror view in a browser DOM. No jsdom/happy-dom setup exists; CM6 gutter extensions require a real browser layout engine to verify column width, hover behavior, and DOM element placement.

### Sampling Rate

- **Per task commit:** `npm run build` in `editor-ui/` (TypeScript + Vite compile)
- **Per wave merge:** Manual visual check in running app (breakpoint toggle, fold hover)
- **Phase gate:** Manual verification against the three success criteria before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] No test framework — manual verification only; this is consistent with all prior phases in this project

*(No new infrastructure gaps specific to this phase — pattern matches established project approach)*

---

## Sources

### Primary (HIGH confidence)

- `node_modules/@codemirror/view/dist/index.js` (v6.40.0) — `lineNumberMarkers` facet declaration, `lineNumberGutter.lineMarker` suppression logic (line 11602), `GutterElement.setMarkers` rendering (line 11513)
- `node_modules/@codemirror/view/dist/index.d.ts` — `LineNumberConfig`, `GutterConfig`, `GutterMarker`, `lineNumberMarkers` TypeScript interfaces
- `node_modules/@codemirror/language/dist/index.js` (v6.12.2) — `FoldGutterConfig`, `foldGutter()` implementation (line 1583), `FoldMarker.toDOM()` with `markerDOM` callback path (line 1570)
- `node_modules/@codemirror/language/dist/index.d.ts` — `FoldGutterConfig` interface (line 796)
- `editor-ui/src/components/Editor.tsx` — existing `BreakpointMarker`, `breakpointState`, `toggleBreakpointEffect`, `createBreakpointGutter`, `buildExtensions`
- `editor-ui/src/codemirror/voidscript-theme.ts` — existing `EditorView.theme()` structure, current gutter CSS rules
- `editor-ui/src/theme/tokens.css` — `--accent-breakpoint`, `--text-tertiary`, `--text-secondary`, `--transition-hover` token values

### Secondary (MEDIUM confidence)

- None needed — all findings verified directly from installed source

### Tertiary (LOW confidence)

- None

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all packages already installed; APIs verified from source
- Architecture: HIGH — `lineNumberMarkers` suppression logic confirmed line-by-line in source; `foldGutter({ markerDOM })` API confirmed from type declarations
- Pitfalls: HIGH — derived from actual source code logic, not guesswork
- Breakpoint fade animation: MEDIUM — CM6 destroy/create timing not tested empirically; `@keyframes` recommendation is standard CSS workaround

**Research date:** 2026-03-15
**Valid until:** 2026-06-15 (CodeMirror 6 is stable; APIs change infrequently)
