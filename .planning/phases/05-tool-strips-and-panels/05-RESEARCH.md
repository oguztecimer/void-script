# Phase 5: Tool Strips and Panels - Research

**Researched:** 2026-03-14
**Domain:** React layout primitives, CSS Modules, react-resizable-panels DOM prep, SVG icon patterns
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- ScriptList header: close button + add-script (plus icon) button
- Side panel widths must have min/max constraints (e.g. min 150px, max 50% viewport)
- Collapsing a panel animates the width transition (~150ms ease, matching established hover transitions)
- Reopening a collapsed panel restores its last dragged width, not a default
- Replace emoji text icons with simple monochrome SVG icons matching Rider's style
- Icon color: tertiary (`--text-tertiary`) by default, brighten on hover
- Active tool strip button: 2px colored border indicator on the inner edge (left edge for left strip, right edge for right strip)

### Claude's Discretion
- DebugPanel header structure and action icons
- Console panel header approach (tab strip row as header vs separate PanelHeader inside)
- Action icon hover states in panel headers
- Bottom panel tab selection, visibility, click behavior, and sizing
- Panel resize implementation strategy (Phase 7 dependency consideration)
- Whether tool strip buttons show rotated text labels

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| PNLS-01 | Tool strip width expanded to 40px with 36px buttons and appropriately sized icons | Tokens `--width-toolstrip: 40px` and `--size-toolstrip-btn: 36px` already defined; ToolStrip.module.css already uses them; need SVG icon swap from emoji text |
| PNLS-02 | Panel header rows with title text + right-aligned action icons on ScriptList, DebugPanel, and Console panels | PanelHeader primitive is fully wired; ScriptList and DebugPanel already use it; action icon buttons use existing `ToolBtn size="small"` pattern |
| PNLS-03 | Bottom panel tab strip with Rider-style chrome (2px active indicator, proper tab sizing) | `BottomTab` inline component in App.tsx already has the 2px `--accent-blue` border-bottom pattern; needs extraction to `BottomTabStrip` component + Zustand `setBottomPanelTab` wired to click |
| PNLS-04 | Resizable panels with drag handles using react-resizable-panels | Per ROADMAP: Phase 5 installs library + wraps DOM in PanelGroup/Panel; Phase 7 makes handles live. This phase does DOM prep only. |
</phase_requirements>

---

## Summary

Phase 5 reworks three visual sub-systems: tool strip icons, panel header chrome, and bottom panel tab strip. It also installs react-resizable-panels and wraps the layout in PanelGroup/Panel containers so Phase 7 can wire up live resize handles without re-touching the DOM structure.

The codebase is already well-prepared. `--width-toolstrip: 40px` and `--size-toolstrip-btn: 36px` tokens are defined and consumed by `ToolStrip.module.css`. `PanelHeader` is a working primitive with a `title + actions` API used by both `ScriptList` and `DebugPanel`. The `BottomTab` component in `App.tsx` already applies the `--accent-blue` 2px border-bottom active pattern — it just needs to be extracted and wired to real tab state.

The main new work is: (1) swap emoji text icons in ToolStrip for inline SVG, (2) add the active-edge indicator CSS to ToolStrip buttons, (3) add the add-script button to ScriptList's header, (4) upgrade DebugPanel header, (5) extract BottomTabStrip and wire Zustand `bottomPanelTab`, (6) install react-resizable-panels and wrap the App layout shell in PanelGroup/Panel/PanelResizeHandle stubs.

**Primary recommendation:** Work in four incremental plans: icons/toolstrip, panel headers, bottom tab strip, then react-resizable-panels DOM prep.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| react-resizable-panels | ^2.0.19 | Resizable split-pane layout | Required by PNLS-04; v2.x API stable; v4 has breaking renames |
| zustand | ^5.0.0 (already installed) | Panel width persistence via store | Already the state layer; add `leftPanelWidth`, `rightPanelWidth` |
| CSS Modules | built-in (Vite) | Scoped styles for all new components | Project convention; no exceptions |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| localStorage (browser API) | native | Persist panel widths across sessions | Cheaper than a Zustand middleware; read on mount, write on drag-end |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| react-resizable-panels | allotment | allotment is a heavier VSCode-origin library; REQUIREMENTS.md names react-resizable-panels explicitly |
| localStorage direct | Zustand persist middleware | Middleware adds boilerplate; direct localStorage is simpler for two numeric values |

**Installation:**
```bash
cd editor-ui && npm install react-resizable-panels
```

---

## Architecture Patterns

### Recommended Project Structure

New files this phase creates:

```
editor-ui/src/
├── components/
│   ├── BottomTabStrip.tsx         # extracted from App.tsx inline BottomTab
│   ├── BottomTabStrip.module.css
│   ├── ToolStrip.tsx              # updated: SVG icons, active-edge indicator
│   ├── ToolStrip.module.css       # updated: .activeLeft / .activeRight classes
│   ├── ScriptList.tsx             # updated: add-script icon in PanelHeader actions
│   ├── DebugPanel.tsx             # updated: consolidated header + action icons
│   └── Console.tsx                # updated (optional): PanelHeader or tab-strip-as-header
├── primitives/
│   └── (no changes needed)
└── App.tsx                        # updated: PanelGroup/Panel/PanelResizeHandle wrapping
```

### Pattern 1: SVG Icons in ToolStrip

**What:** Replace `item.icon` string (currently `'S'`, `'D'`) with inline SVG ReactNodes. Pass SVG as the children of `ToolBtn`. Inline SVGs are self-contained and need no icon library import.

**When to use:** Any tool strip button needing a monochrome, size-controlled icon.

**Example:**
```tsx
// ToolStrip.tsx — SVG as ToolBtn children
const ICONS: Record<string, React.ReactNode> = {
  scripts: (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <path d="M3 2h10v12H3V2zm2 3h6M5 8h6M5 11h4"
            stroke="currentColor" strokeWidth="1.3" strokeLinecap="round"/>
    </svg>
  ),
  debug: (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <path d="M8 3a3 3 0 100 6 3 3 0 000-6zM8 9v4M6 13h4"
            stroke="currentColor" strokeWidth="1.3" strokeLinecap="round"/>
    </svg>
  ),
};
```

The `ToolBtn` CSS already sets `color: var(--text-secondary)` on `.btn` and `color: white` on `.active`. Add `color: var(--text-tertiary)` default for strip items with a specific CSS override class (see Pattern 2).

### Pattern 2: Active Edge Indicator

**What:** A 2px colored bar on the inner edge of an active tool strip button (left edge for right strip, right edge for left strip). Implemented in CSS via `box-shadow` inset or a `::before` pseudo-element — neither requires DOM changes.

**When to use:** Any ToolStrip button that has `active={true}`.

**Example (CSS Module):**
```css
/* ToolStrip.module.css */

/* Override ToolBtn's .active blue fill — strip active is edge indicator only */
.strip :global(.active) {
  background-color: transparent;
  color: var(--text-primary);
}

/* Inner edge indicator — left strip: right edge; right strip: left edge */
.left .activeBtn {
  box-shadow: inset -2px 0 0 var(--accent-blue);
}
.right .activeBtn {
  box-shadow: inset 2px 0 0 var(--accent-blue);
}
```

Pass a `className` prop to `ToolBtn` for the `activeBtn` class when `isActive` is true. `ToolBtn` already accepts an optional `className` prop.

**Alternative:** Use a `::before`/`::after` pseudo-element with `position: absolute`, which gives more control over height (e.g., 60% of button height). Prefer `box-shadow` for simplicity unless Rider shows a partial-height bar.

### Pattern 3: BottomTabStrip Extraction

**What:** Extract the inline `BottomTab` function from `App.tsx` into a standalone `BottomTabStrip` component. Wire it to `useStore` so clicking a tab calls `setBottomPanelTab`.

**When to use:** Bottom panel header in `App.tsx`.

**Example:**
```tsx
// BottomTabStrip.tsx
import { useStore } from '../state/store';
import styles from './BottomTabStrip.module.css';

const BOTTOM_TABS = [
  { id: 'console', label: 'Console' },
  // Room to add 'Run', 'Problems' etc. at Claude's discretion
];

export function BottomTabStrip({ onClose }: { onClose: () => void }) {
  const activeTab = useStore((s) => s.bottomPanelTab);
  const setTab = useStore((s) => s.setBottomPanelTab);

  return (
    <div className={styles.strip}>
      <div className={styles.tabs}>
        {BOTTOM_TABS.map((tab) => (
          <button
            key={tab.id}
            className={`${styles.tab} ${activeTab === tab.id ? styles.active : ''}`}
            onClick={() => setTab(tab.id)}
          >
            {tab.label}
          </button>
        ))}
      </div>
      <div className={styles.actions}>
        {/* ToolBtn clear console + ToolBtn close */}
      </div>
    </div>
  );
}
```

```css
/* BottomTabStrip.module.css */
.strip { display: flex; align-items: center; justify-content: space-between;
         background: var(--bg-panel); border-bottom: 1px solid var(--border-default);
         min-height: 30px; padding: 0 4px; }
.tabs { display: flex; }
.tab { padding: 4px 12px; font-size: 12px; cursor: pointer;
       border: none; background: transparent; border-bottom: 2px solid transparent;
       color: var(--text-tertiary); transition: color var(--transition-hover); }
.tab:hover { color: var(--text-primary); }
.active { color: var(--text-primary); border-bottom-color: var(--accent-blue); }
.actions { display: flex; gap: 2px; padding: 0 4px; }
```

### Pattern 4: react-resizable-panels DOM Prep

**What:** Wrap the App layout shell so it is structurally ready for Phase 7's live resize. In Phase 5, `PanelResizeHandle` components are added but styled as zero-width invisible handles (no drag UI yet). This avoids a DOM restructure in Phase 7.

**When to use:** The `.main` flex row in `App.tsx`.

**Example (Phase 5 stub — handles invisible):**
```tsx
// App.tsx — horizontal layout
import { PanelGroup, Panel, PanelResizeHandle } from 'react-resizable-panels';

<PanelGroup direction="horizontal" autoSaveId="void-main-layout">
  {/* Left tool strip — fixed, not in a Panel */}
  <ToolStrip side="left" ... />

  {/* Left panel */}
  <Panel
    id="left-panel"
    order={1}
    defaultSize={18}
    minSize={12}
    maxSize={40}
    collapsible
  >
    {leftPanelOpen && <ScriptList />}
  </Panel>

  <PanelResizeHandle className={styles.resizeHandle} />

  {/* Center */}
  <Panel id="center" order={2} defaultSize={64}>
    {/* center content */}
  </Panel>

  <PanelResizeHandle className={styles.resizeHandle} />

  {/* Right panel */}
  <Panel
    id="right-panel"
    order={3}
    defaultSize={18}
    minSize={12}
    maxSize={40}
    collapsible
  >
    {rightPanelOpen && isDebugging && <DebugPanel />}
  </Panel>

  <ToolStrip side="right" ... />
</PanelGroup>
```

```css
/* App.module.css — Phase 5: invisible handle stub */
.resizeHandle {
  width: 1px;
  background: var(--border-strong);
  cursor: col-resize;
  /* Phase 7: add visible drag indicator */
}
```

**Key decisions for Phase 5 stub:**
- Use `autoSaveId="void-main-layout"` so Phase 7 gets persistence for free
- Tool strips are NOT wrapped in Panel — they stay fixed-width outside PanelGroup columns
- `collapsible` on side panels so Phase 7 can call `panel.collapse()` from toggle buttons
- `order` prop required because panels are conditionally rendered (`leftPanelOpen`)

### Pattern 5: Collapse Animation Strategy

**What:** The CONTEXT.md requires ~150ms ease transition on panel collapse. However, react-resizable-panels' maintainer explicitly states CSS transitions on Panel width break drag-to-resize (issue #310, discussion #376).

**The solution:** Conditionally apply the CSS transition only when NOT dragging, using the `onDragging` callback on `PanelResizeHandle`:

```tsx
const [isResizing, setIsResizing] = useState(false);
// ...
<PanelResizeHandle onDragging={setIsResizing} className={styles.resizeHandle} />
<Panel className={isResizing ? '' : styles.panelAnimated} ...>
```

```css
.panelAnimated {
  transition: flex-basis 150ms ease;
}
```

This applies the transition only during programmatic collapse/expand (called from toggle buttons in ToolStrip), not during drag. Phase 5 lays this groundwork; Phase 7 implements the toggle buttons.

### Anti-Patterns to Avoid

- **Wrapping ToolStrip in a Panel:** Tool strips are fixed-width UI chrome, not resizable content panes. Keep them outside PanelGroup.
- **Conditional panel rendering with `{open && <Panel>}`:** When a Panel is conditionally rendered without `id` + `order` props, react-resizable-panels loses layout state. Always render Panel but conditionally render its content child.
- **Using `ToolBtn.active` blue-fill for strip buttons:** The Rider active indicator is an edge bar, not a solid blue button background. Override `.active` inside `.strip` scope.
- **Adding CSS `width` transition directly to Panel elements:** Breaks drag-to-resize. Use the conditional `isResizing` pattern above.
- **Inline styles for token values:** Project rule — no inline styles except CSS custom property injection (`--_btn-bg` pattern). All layout/colors go through CSS Modules + tokens.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Resizable split panels | Custom mousedown drag listener + pixel calculation | react-resizable-panels | Edge cases: RTL, keyboard accessibility, touch, iframe pointer events |
| Panel size persistence | Custom localStorage read/write | `autoSaveId` prop on PanelGroup | One prop handles serialize/deserialize; handles race conditions |
| Icon library | Import entire icon package (lucide, heroicons) | Inline SVG per icon | Only 2 icons needed; adding a package for 2 icons is wasteful; SVG inherits `currentColor` for free |

**Key insight:** react-resizable-panels handles the hardest resize cases (pointer capture, touch, keyboard step resize, screen reader announcements) that a hand-rolled solution would miss.

---

## Common Pitfalls

### Pitfall 1: Active indicator fighting ToolBtn's `.active` class

**What goes wrong:** `ToolBtn` has `.active { background-color: var(--accent-blue); color: white; }`. Using `active={true}` on a strip button gives it a solid blue background, not the Rider edge-bar style.

**Why it happens:** ToolBtn's active style was designed for toolbar action buttons (like a toggled Run Config), not for tool strip panel toggles.

**How to avoid:** In `ToolStrip.module.css`, override the active style for buttons inside `.strip` using `:global` or by passing a `className` prop that applies `box-shadow: inset -2px 0 0 var(--accent-blue)` and resets `background-color: transparent`.

**Warning signs:** Active tool strip button shows solid blue fill instead of an edge indicator.

### Pitfall 2: Conditionally rendered Panel loses size state

**What goes wrong:** `{leftPanelOpen && <Panel>...</Panel>}` — when the panel is toggled off, react-resizable-panels loses the Panel from its layout model. On re-open, it reverts to `defaultSize`.

**Why it happens:** react-resizable-panels tracks panels by `id`. If the Panel unmounts, its size record is dropped.

**How to avoid:** Always render `<Panel>` in the tree. Hide content with `{leftPanelOpen && <ScriptList />}` inside the Panel, or use Panel's `collapsible` prop with `collapse()`/`expand()` imperative API.

**Warning signs:** Panel width resets to default after close/reopen cycle.

### Pitfall 3: PanelGroup swallows `.main` flex sizing

**What goes wrong:** `PanelGroup` renders a `div` with `display: flex`. If placed inside `.main` (also `display: flex`), the PanelGroup div may not stretch to fill available space.

**Why it happens:** The outer flex container controls item sizing; PanelGroup needs `flex: 1` or explicit `width: 100%` to fill its flex slot.

**How to avoid:** Apply `style={{ flex: 1 }}` or a CSS Module class with `flex: 1` to the PanelGroup, or replace `.main` with the PanelGroup (making PanelGroup the direct layout root).

**Warning signs:** Layout collapses horizontally, editor area shrinks unexpectedly.

### Pitfall 4: SVG icon inheriting wrong color

**What goes wrong:** SVG icons in inactive strip buttons use `--text-secondary` (ToolBtn default) rather than `--text-tertiary` as required by the locked decision.

**Why it happens:** ToolBtn's `.btn` sets `color: var(--text-secondary)`.

**How to avoid:** Add a `.stripBtn` modifier class in `ToolStrip.module.css` that overrides color to `--text-tertiary`. Pass this class via the `className` prop on `ToolBtn`.

**Warning signs:** Inactive strip icons appear too bright (secondary vs tertiary).

### Pitfall 5: New CSS tokens added to tokens.css but not inlined in index.html

**What goes wrong:** Any new CSS custom property added to `tokens.css` is invisible to wry's WKWebView (custom protocol limitation documented in STATE.md).

**Why it happens:** Established project pattern: wry custom protocol does not apply `:root` blocks from external stylesheets.

**How to avoid:** If this phase needs new tokens (e.g., `--width-panel-handle: 1px`), add them to BOTH `tokens.css` AND the inline `<style>` block in `index.html`.

**Warning signs:** CSS variable resolves as empty string in production build but works in Vite dev server.

---

## Code Examples

Verified patterns from official sources:

### react-resizable-panels: Horizontal layout with autoSaveId
```tsx
// Source: https://app.unpkg.com/react-resizable-panels@2.0.19/files/README.md
import { Panel, PanelGroup, PanelResizeHandle } from 'react-resizable-panels';

<PanelGroup autoSaveId="example" direction="horizontal">
  <Panel defaultSize={25} minSize={15} collapsible>
    <SourcesExplorer />
  </Panel>
  <PanelResizeHandle />
  <Panel>
    <SourceViewer />
  </Panel>
</PanelGroup>
```

### react-resizable-panels: Conditional animation (no drag conflict)
```tsx
// Source: github.com/bvaughn/react-resizable-panels/issues/310
const [isResizing, setIsResizing] = useState(false);

<PanelResizeHandle onDragging={setIsResizing} />
<Panel className={!isResizing ? styles.panelAnimated : ''}>
```

### react-resizable-panels: PanelResizeHandle data-attribute styling
```css
/* Source: github.com/bvaughn/react-resizable-panels issues */
.resizeHandle[data-resize-handle-active='pointer'] {
  background: var(--accent-blue);
}
```

### CSS Modules: box-shadow inset as edge indicator
```css
/* Active edge bar — inner edge of left strip button */
.left .activeBtn {
  background-color: transparent;
  color: var(--text-primary);
  box-shadow: inset -2px 0 0 var(--accent-blue);
}
```

### opacity:0 + pointer-events:none hover-reveal (existing project pattern)
```css
/* Established in Phase 4 for tab close buttons — reuse for panel header actions */
.actionBtn {
  opacity: 0;
  pointer-events: none;
  transition: opacity var(--transition-hover);
}
.header:hover .actionBtn {
  opacity: 1;
  pointer-events: auto;
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Fixed panel widths in CSS | react-resizable-panels PanelGroup | Phase 5 (prep) → Phase 7 (live) | No more hardcoded `width: 220px` in ScriptList.module.css |
| Emoji text icon labels ('S', 'D') | Inline SVG with `currentColor` | Phase 5 | Icons scale correctly and adopt CSS color |
| Inline BottomTab in App.tsx | Standalone BottomTabStrip component | Phase 5 | Testable, re-usable, Zustand-connected |

**Deprecated/outdated after this phase:**
- `width: 220px` hardcode in `ScriptList.module.css` — replaced by Panel defaultSize percentage
- `width: 250px` hardcode in `DebugPanel.module.css` — same
- `item.icon: string` in `LEFT_ITEMS`/`RIGHT_ITEMS` arrays — replaced by `icon: ReactNode`
- Inline `BottomTab` function in `App.tsx` — moved to `BottomTabStrip`

---

## Open Questions

1. **Tool strip icon designs**
   - What we know: Icons should be simple, monochrome, 16x16px SVG. The CONTEXT says "matching Rider's style."
   - What's unclear: Exact path data for the specific icons (files/scripts icon, debug/bug icon).
   - Recommendation: Use simple, universally recognizable shapes. For "scripts": a document with lines. For "debug": a bug or play-with-steps icon. Keep paths minimal (1-2 paths per icon). Planner should define final SVG path data in the task.

2. **DebugPanel header structure**
   - What we know: DebugPanel currently has TWO PanelHeader rows (Frames + Variables). The close button is in the first one.
   - What's unclear: Should PNLS-02 add a single top-level "Debug" PanelHeader, or polish the two existing sections?
   - Recommendation: Keep the two-section structure (it mirrors Rider's debug panel). Add a single top-level "Debug" PanelHeader above "Frames" with the close action, matching ScriptList's pattern.

3. **Console header approach (Claude's Discretion)**
   - What we know: The bottom panel header row (with the tab strip) effectively IS the Console's header.
   - Recommendation: Do NOT add a separate PanelHeader inside Console.tsx. Instead, the `BottomTabStrip` serves as the Console panel header. Add "Console" tab label with clear action in the tab strip's actions area.

4. **Panel percentage defaults vs pixel widths**
   - What we know: react-resizable-panels v2 works only in percentages. Current panel widths are 220px (left) and 250px (right).
   - What's unclear: What percentage should they default to given variable screen widths?
   - Recommendation: Use `defaultSize={18}` for side panels, `minSize={12}`, `maxSize={40}`. These roughly correspond to 220px on a 1200px wide window.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | None installed (no vitest.config, jest.config, or test files found) |
| Config file | none — see Wave 0 |
| Quick run command | `cd editor-ui && npm run build` (TypeScript compile as proxy) |
| Full suite command | `cd editor-ui && npm run build` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PNLS-01 | Tool strip renders 40px wide with 36px SVG buttons | manual-only | `npm run build` (TS errors only) | ❌ Wave 0 |
| PNLS-02 | Panel headers show title + action icons on all three panels | manual-only | `npm run build` (TS errors only) | ❌ Wave 0 |
| PNLS-03 | Bottom tab strip renders active tab with 2px blue indicator | manual-only | `npm run build` (TS errors only) | ❌ Wave 0 |
| PNLS-04 | PanelGroup/Panel DOM structure wraps layout (prep only) | manual-only | `npm run build` (TS errors only) | ❌ Wave 0 |

**Manual-only justification:** All requirements are pure CSS/visual — pixel dimensions, colors, and layout proportions cannot be meaningfully verified with unit tests. The project has no test framework installed. TypeScript compilation catches interface/prop errors.

### Sampling Rate
- **Per task commit:** `cd editor-ui && npm run build`
- **Per wave merge:** `cd editor-ui && npm run build`
- **Phase gate:** Build green + visual inspection before `/gsd:verify-work`

### Wave 0 Gaps
- No test framework to install (visual-only phase, build check is sufficient)
- [ ] Consider adding vitest in a future infrastructure phase

---

## Sources

### Primary (HIGH confidence)
- `react-resizable-panels` v2.0.19 README — https://app.unpkg.com/react-resizable-panels@2.0.19/files/README.md
  - PanelGroup props, Panel props, PanelResizeHandle props, autoSaveId, collapsible, imperative methods
- Existing project source — direct file reads of App.tsx, ToolStrip.tsx, ToolBtn.tsx, PanelHeader.tsx, store.ts, tokens.css, index.html

### Secondary (MEDIUM confidence)
- react-resizable-panels GitHub discussion #376 — CSS transition/animation limitation + conditional isResizing workaround
- react-resizable-panels GitHub issue #310 — onDragging-based transition toggle pattern
- react-resizable-panels deepwiki examples — component usage patterns

### Tertiary (LOW confidence)
- JetBrains Rider New UI docs — official docs confirm New UI exists and tool window strip concept; pixel dimensions not published officially (verified from project tokens which were set by the user)

---

## Metadata

**Confidence breakdown:**
- Standard stack (react-resizable-panels v2): HIGH — verified against official README on unpkg
- Architecture (DOM prep pattern, conditional animation): HIGH — verified from library maintainer responses
- Pitfalls: HIGH — ToolBtn active override and conditional rendering pitfalls are verified from library docs; wry token pitfall from STATE.md decision log
- SVG icon designs: LOW — specific path data is discretionary; shapes are obvious but not validated against Rider screenshots

**Research date:** 2026-03-14
**Valid until:** 2026-04-14 (react-resizable-panels is actively maintained; v2 API is stable)
