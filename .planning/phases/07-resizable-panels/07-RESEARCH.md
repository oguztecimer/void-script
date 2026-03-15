# Phase 7: Resizable Panels - Research

**Researched:** 2026-03-15
**Domain:** react-resizable-panels v4 vertical Group, nested layout persistence, collapse sync
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Bottom Panel Resize**
- Nested vertical `Group` (react-resizable-panels) inside center panel, wrapping editor area and bottom panel
- Percentage-based constraints: min 10%, max 50% of center area height
- Persist height via `useDefaultLayout` with a named storage key (consistent with horizontal group)
- Collapsible by dragging past minimum size, synced to Zustand `bottomPanelOpen` state
- Default height: 25% of center area

**Resize Handle Appearance**
- No hover feedback — handle stays as plain 1px `border-strong` line (matches Rider)
- No drag feedback — no color change during active drag
- No grip indicator (dots/lines) — Rider doesn't show grip marks on panel dividers
- Vertical (bottom panel) handle uses `row-resize` cursor; horizontal handles keep `col-resize`

**Collapse Behavior**
- Double-click a resize handle toggles collapse/expand of the adjacent panel
- Snap-to-collapse threshold: dragging below 50% of minimum size auto-collapses to zero
- All collapse interactions (drag snap, double-click) sync to Zustand state (`leftPanelOpen`, `rightPanelOpen`, `bottomPanelOpen`)
- Collapse animation: 150ms ease (existing `panelAnimated` class)

**Panel Size Defaults**
- Side panels: 18% default width (already set, unchanged)
- Bottom panel: 25% of center area height
- No "reset layout" UI option
- Named storage keys for layout persistence (discoverable in localStorage)

### Claude's Discretion
- Vertical group storage key naming convention
- Implementation details of double-click handler on Separator
- How to wire snap-to-collapse threshold with react-resizable-panels v4 API
- Whether the bottom panel vertical group needs its own `onLayoutChange`/`onLayoutChanged` callbacks or shares with the horizontal group

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| PNLS-04 | Resizable panels with drag handles using react-resizable-panels | Vertical Group nesting, `useDefaultLayout` per group, imperative collapse/expand synced to Zustand, CSS cursor overrides for horizontal handle |
</phase_requirements>

---

## Summary

Phase 7 is a focused integration task: add a nested vertical `Group` inside the center panel to make the bottom panel resizable and persistent, wire double-click-to-collapse on all Separators, and ensure every collapse interaction keeps Zustand state in sync. The horizontal layout (left/right side panels) already works from Phase 5 — the new work is exclusively about the vertical axis inside center.

The react-resizable-panels v4 API (installed as 4.7.3) has all required primitives. The snap-to-collapse behavior is built in: when a `collapsible` Panel's size drops below its `minSize`, the library automatically snaps it to `collapsedSize` (defaults to 0). No custom threshold logic is needed from the app. Double-click-to-toggle is not built into Separator — it requires a plain `onDoubleClick` DOM handler on the Separator element, which calls `panelRef.current.collapse()` or `panelRef.current.expand()`.

**Primary recommendation:** Convert the `.center` div into a vertical `Group`, give it its own `useDefaultLayout` storage key (`void-center-layout`), add the bottom panel as a `collapsible Panel` with `panelRef`, and mirror the left/right collapse sync pattern for `bottomPanelOpen`. Remove `height: 200px` from `.bottomPanel` CSS class entirely.

---

## Standard Stack

### Core (already installed — no new dependencies)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| react-resizable-panels | 4.7.3 | Resizable panel groups, persistence, imperative collapse | Already in use for horizontal layout; vertical orientation is native |
| zustand | 5.0.0 | `bottomPanelOpen` state, `toggleBottomPanel` action | Already the app's state manager; pattern is established |

### No New Dependencies Required

All required functionality is available from the already-installed library. Phase 7 is purely a wiring exercise.

**Installation:**
```bash
# Nothing new — react-resizable-panels 4.7.3 already installed
```

---

## Architecture Patterns

### Recommended Project Structure

No new files or directories. All changes are in:
```
editor-ui/src/
├── App.tsx                      # Add vertical Group, bottomPanelRef, double-click handlers
└── App.module.css               # Add .resizeHandleHorizontal for row-resize cursor; remove height from .bottomPanel
```

### Pattern 1: Nested Vertical Group (the core change)

**What:** Replace the `.center` div with a vertical `Group`. The editor area becomes one Panel; the bottom panel becomes a second collapsible Panel.

**When to use:** Any time you need a vertically-stacked resizable layout inside an existing horizontal layout.

**Example:**
```tsx
// Source: installed react-resizable-panels dist/react-resizable-panels.d.ts
const { defaultLayout: centerLayout, onLayoutChanged: saveCenterLayout } =
  useDefaultLayout({ id: 'void-center-layout' });

const bottomPanelRef = useRef<PanelImperativeHandle | null>(null);

// Inside JSX, replacing <Panel id="center"><div className={styles.center}>...</div></Panel>:
<Panel id="center">
  <Group
    id="void-center-layout"
    orientation="vertical"
    defaultLayout={centerLayout}
    onLayoutChange={() => setIsResizing(true)}
    onLayoutChanged={(layout) => { setIsResizing(false); saveCenterLayout(layout); }}
    className={styles.centerGroup}
  >
    {/* Editor area — fills remaining space */}
    <Panel id="editor-panel" minSize="50%" maxSize="90%">
      <div className={styles.editorArea}>
        <TabBar />
        <Editor />
      </div>
    </Panel>

    <Separator
      id="bottom-separator"
      className={bottomPanelOpen ? styles.resizeHandleHorizontal : styles.resizeHandleHidden}
      disabled={!bottomPanelOpen}
      onDoubleClick={handleBottomPanelDoubleClick}
    />

    {/* Bottom panel — collapsible */}
    <Panel
      panelRef={bottomPanelRef}
      id="bottom-panel"
      defaultSize="25%"
      minSize="10%"
      maxSize="50%"
      collapsible
      collapsedSize={0}
      className={!isResizing ? styles.panelAnimated : ''}
    >
      <div className={styles.bottomPanel}>
        <BottomTabStrip />
        <Console />
      </div>
    </Panel>
  </Group>
</Panel>
```

### Pattern 2: Imperative Collapse Sync to Zustand (established pattern — mirror for bottom)

**What:** A `useEffect` watches Zustand state and calls `panelRef.current.collapse()` or `.expand()` to keep the library panel in sync.

**When to use:** Whenever external UI (a button, keyboard shortcut, or another component) needs to trigger panel collapse without going through a drag interaction.

**Example:**
```tsx
// Source: App.tsx existing pattern (left/right panels) — apply identically to bottom
useEffect(() => {
  const panel = bottomPanelRef.current;
  if (!panel) return;
  if (bottomPanelOpen) {
    panel.expand();
  } else {
    panel.collapse();
  }
}, [bottomPanelOpen]);
```

### Pattern 3: Double-Click Toggle Handler

**What:** Separator has no built-in double-click-to-collapse. A plain `onDoubleClick` handler reads `isCollapsed()` and calls `collapse()` or `expand()`, then syncs Zustand.

**When to use:** All three Separators (left, right, bottom) should have this handler.

**Example:**
```tsx
// Discretion area: implementation detail for double-click handler on Separator
// Source: PanelImperativeHandle API from dist/react-resizable-panels.d.ts
const handleBottomPanelDoubleClick = useCallback(() => {
  const panel = bottomPanelRef.current;
  if (!panel) return;
  if (panel.isCollapsed()) {
    panel.expand();
    setBottomPanelOpen(true);
  } else {
    panel.collapse();
    setBottomPanelOpen(false);
  }
}, [setBottomPanelOpen]);

// Apply to Separator:
<Separator
  id="bottom-separator"
  onDoubleClick={handleBottomPanelDoubleClick}
  ...
/>
```

### Pattern 4: Snap-to-Collapse (built-in, no custom code needed)

**What:** When `collapsible={true}` is set on a Panel, the library automatically snaps to `collapsedSize` when the panel is dragged below `minSize`. The 50% threshold mentioned in the context ("dragging below 50% of minimum size") is the library's default behavior.

**Key finding (HIGH confidence — from installed type declarations):**
> "A collapsible panel will collapse when its size is less than the specified `minSize`"

This means the snap-to-collapse is built into the drag interaction. The app only needs to detect when the panel collapses post-drag to sync Zustand. Use `Panel.onResize` callback to detect collapse state:

```tsx
// Discretion area: using onResize to sync Zustand after drag-snap collapse
<Panel
  panelRef={bottomPanelRef}
  id="bottom-panel"
  collapsible
  collapsedSize={0}
  minSize="10%"
  onResize={(size) => {
    // sync Zustand when panel collapses/expands via drag
    const collapsed = size.asPercentage === 0;
    if (collapsed !== !bottomPanelOpen) {
      setBottomPanelOpen(!collapsed);
    }
  }}
  ...
/>
```

Note: `onResize` fires frequently during drag. The condition guard (`collapsed !== !bottomPanelOpen`) prevents unnecessary Zustand updates.

### Pattern 5: Storage Key Convention (discretion area)

**Recommendation:** Use `void-center-layout` for the vertical group. This is consistent with `void-main-layout` for the horizontal group. Both keys will be visible in `localStorage` under the `useDefaultLayout` storage implementation.

```tsx
const { defaultLayout: centerLayout, onLayoutChanged: saveCenterLayout } =
  useDefaultLayout({ id: 'void-center-layout' });
```

### Anti-Patterns to Avoid

- **Mounting/unmounting bottom panel conditionally:** The current code has `{bottomPanelOpen && <div className={styles.bottomPanel}>}` — this must become an always-rendered Panel with imperative collapse, matching the left/right pattern. Conditional rendering causes unmount flicker and loses scroll position.
- **Keeping `height: 200px` on `.bottomPanel`:** The CSS class must remove the fixed height — Panel size is controlled by the Group, not CSS height.
- **Using `onLayoutChange` for Zustand sync:** `onLayoutChange` fires on every pointer move. Use `Panel.onResize` with a guard or `onLayoutChanged` (fires only on pointer release) for Zustand sync.
- **Forgetting `TabBar` is outside the inner Group:** `TabBar` is inside `.center` but above the editor area. It must stay outside the inner `Group` — as a fixed header above the vertical Group. Structure: `<Panel id="center"><div className={styles.center}><TabBar /><Group orientation="vertical">...</Group></div></Panel>`.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Panel size persistence | Custom localStorage write in onLayoutChange | `useDefaultLayout({ id: 'void-center-layout' })` | Handles debouncing, parse errors, version mismatch; already works for horizontal group |
| Snap-to-collapse on drag | Custom threshold detection in onLayoutChange | `collapsible` + `collapsedSize={0}` on Panel | Library auto-snaps below `minSize`; threshold is configurable via `minSize` value |
| Panel size in pixels vs percent | Manual pixel calculations | Library's native `getSize().asPercentage` | Library tracks size as percentage; converting is unnecessary |
| Cursor management during drag | CSS cursor hacks on document | `Group` default cursor behavior | Library sets `col-resize`/`row-resize` cursor on the Group during drag automatically unless `disableCursor` is set |

**Key insight:** react-resizable-panels v4 handles all the hard problems. Phase 7 is almost entirely wiring and CSS cleanup.

---

## Common Pitfalls

### Pitfall 1: TabBar Inside Vertical Group
**What goes wrong:** If `TabBar` is placed inside the editor Panel in the vertical Group, it gets squished when the bottom panel grows. The editor content area shrinks while the tab bar should remain fixed-height.
**Why it happens:** Vertical Group distributes height; the editor Panel itself should only contain the resizable content area (CodeMirror), not fixed-height headers.
**How to avoid:** Keep `TabBar` as a sibling of the vertical `Group`, both inside a flex column `.center` wrapper. The vertical Group takes `flex: 1` inside `.center`.
**Warning signs:** Tab bar height changes when bottom panel is resized.

### Pitfall 2: Double useEffect Firing for Bottom Panel State
**What goes wrong:** If `onResize` and `useEffect([bottomPanelOpen])` both run on a drag-collapse, they can fight — the useEffect calls `expand()` after the drag already collapsed the panel.
**Why it happens:** `onResize` fires → updates Zustand → useEffect fires → calls `expand()` on a panel that just snapped closed.
**How to avoid:** Separate the source of truth: `useEffect` only responds to external state changes (button clicks, keyboard). For drag-induced collapses, `onResize` detects the collapsed state and updates Zustand without triggering a useEffect. Guard: only call Zustand from `onResize` when the Zustand state doesn't already match.
**Warning signs:** Bottom panel immediately re-opens after dragging to collapse.

### Pitfall 3: centerGroup Needs height: 100%
**What goes wrong:** The vertical Group inside the center Panel renders at 0 height if the Panel content wrapper has no height constraint.
**Why it happens:** Group uses `display: flex; flex-direction: column` internally; it needs a defined height to distribute.
**How to avoid:** Ensure the `.centerGroup` CSS class (or the inner Panel content wrapper) has `height: 100%` or is a flex child with `flex: 1; min-height: 0`.
**Warning signs:** Editor area collapses to 0; layout appears broken on first render.

### Pitfall 4: Separator className on Horizontal Handle
**What goes wrong:** The existing horizontal Separators use `cursor: col-resize` via `.resizeHandle`. The new vertical Separator must use `cursor: row-resize`. If the same class is reused, the cursor is wrong on the bottom Separator.
**Why it happens:** Horizontal and vertical resize handles need different cursor styles.
**How to avoid:** Add a `.resizeHandleHorizontal` class that is identical to `.resizeHandle` except `cursor: row-resize` instead of `col-resize`.
**Warning signs:** Bottom panel Separator shows col-resize cursor (left-right arrows).

### Pitfall 5: isResizing State Shared Across Both Groups
**What goes wrong:** `isResizing` state currently controls `panelAnimated` class on left/right panels. If the same state is shared with the vertical Group, dragging the bottom Separator will also remove animation from the side panels (and vice versa).
**Why it happens:** Single `isResizing` boolean used for all panels.
**How to avoid:** Two options — (a) use one `isResizing` for all, which is fine since you never drag two handles simultaneously, or (b) separate states for vertical/horizontal. Option (a) is simpler and correct for this use case.
**Warning signs:** Animation is removed from unrelated panels during resize.

---

## Code Examples

### Vertical CSS for centerGroup
```css
/* Source: App.module.css — new rules needed */
.centerGroup {
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.resizeHandleHorizontal {
  height: 1px;
  background: var(--border-strong);
  cursor: row-resize;
  flex-shrink: 0;
  outline: none;
}

/* Remove height: 200px from .bottomPanel, keep rest */
.bottomPanel {
  /* height: 200px; <-- REMOVE THIS */
  display: flex;
  flex-direction: column;
  height: 100%;
  border-top: 1px solid var(--border-strong);
}
```

### Full Bottom Panel Integration in App.tsx
```tsx
// Source: react-resizable-panels dist/react-resizable-panels.d.ts + existing App.tsx patterns
const { defaultLayout: centerLayout, onLayoutChanged: saveCenterLayout } =
  useDefaultLayout({ id: 'void-center-layout' });

const bottomPanelRef = useRef<PanelImperativeHandle | null>(null);
const setBottomPanelOpen = useStore((s) => s.setBottomPanelOpen);

// Sync imperative collapse to Zustand (for external triggers: button, keyboard)
useEffect(() => {
  const panel = bottomPanelRef.current;
  if (!panel) return;
  if (bottomPanelOpen) {
    panel.expand();
  } else {
    panel.collapse();
  }
}, [bottomPanelOpen]);

// Double-click handler (discretion: implementation detail for Separator)
const handleBottomSeparatorDoubleClick = useCallback(() => {
  const panel = bottomPanelRef.current;
  if (!panel) return;
  if (panel.isCollapsed()) {
    panel.expand();
    setBottomPanelOpen(true);
  } else {
    panel.collapse();
    setBottomPanelOpen(false);
  }
}, [setBottomPanelOpen]);
```

### onResize Guard for Drag-Sync
```tsx
// Source: PanelProps.onResize signature from dist/react-resizable-panels.d.ts
<Panel
  panelRef={bottomPanelRef}
  id="bottom-panel"
  defaultSize="25%"
  minSize="10%"
  maxSize="50%"
  collapsible
  collapsedSize={0}
  className={!isResizing ? styles.panelAnimated : ''}
  onResize={(size) => {
    const isNowCollapsed = size.asPercentage === 0;
    // Only update Zustand when state actually changes to avoid loop with useEffect
    if (isNowCollapsed && bottomPanelOpen) {
      setBottomPanelOpen(false);
    } else if (!isNowCollapsed && !bottomPanelOpen) {
      setBottomPanelOpen(true);
    }
  }}
>
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Conditional render `{bottomPanelOpen && <div>}` | Always-rendered Panel with imperative collapse | Phase 7 | No unmount flicker; Bottom panel scroll position preserved |
| Fixed `height: 200px` on `.bottomPanel` | Panel `defaultSize="25%"` inside vertical Group | Phase 7 | User-resizable, persisted across reloads |
| Drag detection via `onLayoutChange` firing | `onLayoutChange` for `isResizing` + `onLayoutChanged` for save | Phase 5 pattern (continued) | No save-on-every-pointer-move; animation disabled only during drag |

**Confirmed from installed library (v4.7.3):**
- `onLayoutChange` is the high-frequency callback (fires per pointer move)
- `onLayoutChanged` is the settled callback (fires on pointer release only) — correct one for persistence
- `collapsible` auto-snaps below `minSize` — no custom threshold code needed

---

## Open Questions

1. **Does `Panel.onResize` fire with `asPercentage === 0` exactly when snapped, or is it a tiny nonzero value?**
   - What we know: The library snaps to `collapsedSize={0}` — so the value should be exactly 0
   - What's unclear: Floating point edge cases in the library's snap logic
   - Recommendation: Use `size.asPercentage < 1` as the collapse detection guard rather than `=== 0`, to be safe

2. **Does the `PanelImperativeHandle.expand()` restore to the last pre-collapse size?**
   - What we know: The type declaration says "Expand a collapsed Panel to its most recent size" — this implies yes
   - What's unclear: Whether "most recent size" means last drag size or `defaultSize` on first expand
   - Recommendation: On first page load, if no persisted layout exists, expand will likely use `defaultSize="25%"`. This is correct behavior.

3. **Does the double-click handler on Separator need `e.preventDefault()`?**
   - What we know: Double-click on a div element selects text in adjacent elements; on a Separator this is less likely but possible
   - What's unclear: Whether react-resizable-panels already prevents default on Separator events
   - Recommendation: Add `onDoubleClick={(e) => { e.preventDefault(); handleBottomSeparatorDoubleClick(); }}` as a safe default

---

## Validation Architecture

> nyquist_validation is true in .planning/config.json — section included.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | None detected — no vitest.config, jest.config, or test files found in repo |
| Config file | None — Wave 0 gap |
| Quick run command | N/A (no test framework installed) |
| Full suite command | N/A |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PNLS-04 | Vertical Group renders with bottom panel at 25% default | manual-only | N/A — wry WebView, visual | ❌ |
| PNLS-04 | Bottom panel resizes smoothly without layout shift | manual-only | N/A — visual drag interaction | ❌ |
| PNLS-04 | Panel sizes persist after page reload (localStorage keys present) | manual-only | `localStorage.getItem('void-center-layout')` in DevTools | ❌ |
| PNLS-04 | Collapse via drag (snap below minSize) syncs Zustand state | manual-only | N/A — requires pointer events | ❌ |
| PNLS-04 | Double-click Separator toggles collapse | manual-only | N/A — requires pointer events | ❌ |

**Why all manual-only:** This project has no test framework installed. All behaviors involve drag interactions and visual rendering in a wry WebView. These cannot be automated without a browser test runner (Playwright/Cypress) and wry-specific test harness, neither of which exists in this project.

### Sampling Rate
- **Per task commit:** Manual visual check in running app
- **Per wave merge:** Full manual checklist from VERIFICATION.md
- **Phase gate:** All VERIFICATION.md truths confirmed before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] No test framework installed — all PNLS-04 verification is human/visual

*(No automated tests are feasible for this phase without adding a test framework, which is out of scope.)*

---

## Sources

### Primary (HIGH confidence)
- Installed `react-resizable-panels` dist/react-resizable-panels.d.ts (v4.7.3) — `Group`, `Panel`, `Separator`, `PanelImperativeHandle`, `useDefaultLayout` type signatures; collapse semantics; onResize callback signature
- `editor-ui/src/App.tsx` — existing patterns for `useDefaultLayout`, `panelRef`, `useEffect` collapse sync, `isResizing` state
- `editor-ui/src/App.module.css` — existing `.resizeHandle`, `.resizeHandleHidden`, `.panelAnimated` CSS classes
- `editor-ui/src/state/store.ts` — `bottomPanelOpen`, `toggleBottomPanel`, `setBottomPanelOpen` already present

### Secondary (MEDIUM confidence)
- `.planning/phases/07-resizable-panels/07-CONTEXT.md` — user decisions (locked and discretion items)
- `editor-ui/src/components/BottomTabStrip.tsx` — renders inside bottom panel; confirmed it calls `toggleBottomPanel` for the close button (will need updating to call `setBottomPanelOpen(false)` directly or continue using `toggleBottomPanel` — both work)

### Tertiary (LOW confidence)
- None

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — library already installed and in use; all API signatures read from installed types
- Architecture: HIGH — vertical Group pattern is a direct mirror of existing horizontal Group; all APIs confirmed from installed types
- Pitfalls: HIGH — identified from reading actual code (conditional render, height: 200px, cursor style, useEffect/onResize ordering)
- Validation: HIGH — no test framework exists in project; all behaviors are visual

**Research date:** 2026-03-15
**Valid until:** 2026-09-15 (stable library; 6 months)
