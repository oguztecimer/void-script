# Phase 3: Title Bar - Research

**Researched:** 2026-03-14
**Domain:** React CSS Modules, macOS wry drag-region hover, Rider New UI toolbar layout
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- Search Everywhere widget: Rider-style search pill, fixed width ~200-240px, content: magnifying glass icon + "Search" text + "⇧⇧" shortcut hint (right-aligned, muted). Non-functional — visual shell only.
- Toolbar widget arrangement: `[traffic lights] | [hamburger] | [◀ ▶] | [project ▾] [VCS] | ---spacer--- | [run config ▾] [▶] [🪲] | [search pill] | [⚙]`
- Non-functional widgets (hamburger, back/forward) kept for visual completeness.
- Separators only between groups — no separator between project and VCS, no separator between run config and action buttons.
- Settings gear icon at far-right position.
- Debug controls arrangement when debugging and paused: `[⬛ Stop] | [▶ Resume] [⤵ Step Over] [↓ Step Into] [↑ Step Out]`. Stop separated from stepping buttons.
- Drop the "Running..."/"Debugging..."/"Paused" status text — state communicated through visible buttons only.
- Reuse ToolBtn primitive with `variant="filled"` for Run/Debug/Stop/Resume buttons.
- RunConfigSelector stays as header-specific component, migrated to CSS Module.
- All toolbar buttons uniformly 26px tall.
- Full CSS Module migration — Header.tsx gets `Header.module.css`, all inline styles move to CSS classes.
- Replace local ToolBtn and ActionBtn with ToolBtn primitive from `src/primitives/`.
- Replace local Separator with Separator primitive from `src/primitives/`.
- Keep WindowControls, TrafficLight, HeaderWidget, RunConfigSelector as header-specific local components.
- macOS drag region hover split: JS hover (`onMouseEnter`/`onMouseLeave`) for buttons directly in the drag region, CSS `:hover` for buttons inside `titlebar-no-drag` containers.
- Traffic lights migrate to CSS Module with `:hover` (they are in a no-drag zone).

### Claude's Discretion

- Exact pixel widths for search pill (within ~200-240px range)
- Icon sizing and SVG details for Search Everywhere magnifying glass and Settings gear
- Exact gap/margin values between widget groups (match Rider reference)
- Border radius on search pill and widget buttons
- HeaderWidget internal styling (icon + label + chevron compound layout)
- How to structure the CSS Module (class naming, grouping)

### Deferred Ideas (OUT OF SCOPE)

- Search Everywhere actual functionality (fuzzy search, modal, tabbed results) — v2 requirements SRCH-01/SRCH-02
- Settings menu/panel behind the gear icon — future phase
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| TBAR-01 | Widget buttons sized to 26px height matching Rider New UI proportions | `--height-widget-btn: 26px` token exists; ToolBtn `size="small"` maps to `var(--height-widget-btn)` — direct reuse, no new sizing needed |
| TBAR-02 | Correct spacing, separator positions, and font weights across all toolbar widgets | Separator primitive supports `variant="line"` with `level="subtle"` — matches current local Separator; group gap pattern established by Phase 2 |
| TBAR-03 | Search Everywhere magnifying glass icon button in toolbar center-right area | New SearchPill component: rounded-rect button, `titlebar-no-drag`, CSS `:hover` safe inside no-drag container |
| TBAR-04 | Settings gear icon at toolbar far-right position | ToolBtn primitive `size="small"` with SVG gear icon; rightmost element after search pill |
</phase_requirements>

---

## Summary

Phase 3 is a component-restructure and CSS migration, not a new feature build. The primary technical work is: (1) migrate `Header.tsx`'s 7 inline-style sub-components to CSS Modules, (2) replace local primitives (ToolBtn, ActionBtn, Separator) with the project's established primitives from `src/primitives/`, (3) add two new widgets (SearchPill and SettingsBtn), and (4) apply the correct Rider widget arrangement with proper group separators.

The codebase already has everything needed. `--height-widget-btn: 26px` is a live token. `ToolBtn` primitive's `size="small"` maps directly to `var(--height-widget-btn)`. `Separator` primitive covers the group dividers. The only genuinely new components are `SearchPill` (a styled static button) and `SettingsBtn` (a `ToolBtn` `size="small"` with a gear SVG).

The macOS drag-region hover issue is real but already solved by the project's `titlebar-no-drag` pattern. Buttons that live inside a `titlebar-no-drag` wrapper use CSS `:hover` safely. Buttons that must sit in the drag region require JS `onMouseEnter`/`onMouseLeave`. The CONTEXT.md decision is clear: traffic lights move to CSS `:hover` (they are already in `titlebar-no-drag`), while HeaderWidget and other direct drag-region children retain JS hover.

**Primary recommendation:** Structure the work as two tasks — (1) migrate Header layout + replace local primitives + fix widget arrangement/sizing, (2) add SearchPill + SettingsBtn + apply final Rider visual polish. Each task is independently shippable.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| React (CSS Modules) | ^19.0.0 | Component scoping | Established in Phase 2; zero runtime cost |
| Zustand | ^5.0.0 | State (`isRunning`, `isDebugging`, `isPaused`) | Already wired into Header.tsx |
| ToolBtn primitive | project | All 26px icon buttons and filled action buttons | Phase 2 deliverable; handles ghost/filled variants |
| Separator primitive | project | Group dividers | Phase 2 deliverable; handles line/gap/level variants |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `titlebar-drag` / `titlebar-no-drag` CSS classes | index.html | wry drag region control | Every interactive element must be in `titlebar-no-drag` |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| CSS `:hover` in no-drag zones | JS `onMouseEnter`/`onMouseLeave` everywhere | JS hover has no desync risk but is more verbose; CSS `:hover` is cleaner inside no-drag containers |
| Flat single CSS Module | Multiple per-component CSS Modules | Phase decision is single `Header.module.css` co-located with `Header.tsx` |

**Installation:** No new packages required.

---

## Architecture Patterns

### Recommended File Structure

```
src/components/
├── Header.tsx              # Restructured — imports Header.module.css
├── Header.module.css       # NEW — all header styles
src/primitives/
├── ToolBtn.tsx             # Already exists — reused as-is
├── Separator.tsx           # Already exists — reused as-is
```

All 7 sub-components (WindowControls, TrafficLight, HeaderWidget, RunConfigSelector, SearchPill, SettingsBtn, and the debug step group) live inside `Header.tsx` as local function components. No separate files.

### Pattern 1: 26px Button Uniformity

**What:** Every interactive button in the toolbar uses `ToolBtn` with `size="small"` or a component sized to `var(--height-widget-btn)`.

**When to use:** Every toolbar button — icon buttons (hamburger, back, forward, settings), action buttons (run, debug, stop, resume, step), and compound widgets (HeaderWidget, RunConfigSelector, SearchPill).

**Example:**
```tsx
// Ghost icon button — 26px
<ToolBtn size="small" title="Settings">
  <svg width="14" height="14" viewBox="0 0 16 16">...</svg>
</ToolBtn>

// Filled action button — 26px, uses CSS custom property for per-instance color
<ToolBtn
  size="small"
  variant="filled"
  title="Run"
  bgColor="var(--bg-btn-run)"
  hoverBgColor="var(--bg-btn-run-hover)"
  iconColor="var(--icon-run)"
  onClick={...}
  disabled={!activeTabId}
>
  <svg width="10" height="10" viewBox="0 0 16 16">
    <path d="M4 2l10 6-10 6V2z" fill="currentColor"/>
  </svg>
</ToolBtn>
```

### Pattern 2: CSS Module for Compound Widgets

**What:** Components with internal layout (HeaderWidget, RunConfigSelector, SearchPill) get CSS Module classes for their internal flex layout rather than inline styles.

**When to use:** Any component with multiple child elements (icon + label + chevron, icon + text + shortcut hint).

**Example:**
```tsx
// Header.module.css
.widget {
  display: flex;
  align-items: center;
  gap: 6px;
  height: var(--height-widget-btn);
  padding: 0 8px;
  background: none;
  border: none;
  border-radius: 6px;
  color: var(--text-primary);
  cursor: pointer;
  font-size: var(--font-size-ui);
  font-family: inherit;
  font-weight: 600;
  transition: background-color var(--transition-hover);
}
.widget:hover {
  background-color: var(--bg-hover);
}
.widgetMuted {
  color: var(--text-secondary);
  font-weight: 400;
}
.widgetIcon {
  display: flex;
  align-items: center;
  color: var(--text-tertiary);
}
.widgetChevron {
  color: var(--text-tertiary);
}
```

### Pattern 3: Toolbar Layout with Flex Spacer

**What:** The drag-region spacer (`flex: 1`) separates left group from right group. Left group is the draggable region; right group is `titlebar-no-drag`.

**When to use:** The outer header container must be `titlebar-drag`; the right interactive cluster wraps in `titlebar-no-drag`.

**Example:**
```tsx
// Header.module.css
.toolbar {
  display: flex;
  align-items: center;
  height: var(--height-titlebar);
  background-color: var(--bg-toolbar);
  padding: 0 8px;
  user-select: none;
  border-bottom: 1px solid var(--border-strong);
  font-size: var(--font-size-ui);
}
.spacer {
  flex: 1;
}
.rightGroup {
  display: flex;
  align-items: center;
  gap: 4px;
}
```

```tsx
// Header.tsx
<div className={`titlebar-drag ${styles.toolbar}`}>
  <WindowControls />
  <Separator variant="line" level="subtle" />
  {/* left widgets */}
  <div className={styles.spacer} />
  <div className={`titlebar-no-drag ${styles.rightGroup}`}>
    {/* run config, action buttons, search pill, settings */}
  </div>
</div>
```

### Pattern 4: SearchPill Component

**What:** A `titlebar-no-drag` button styled as a wide rounded-rectangle with icon + text + shortcut hint. Safe to use CSS `:hover` since it is inside the no-drag right group.

**When to use:** Only for the Search Everywhere visual shell.

**Example:**
```tsx
// Header.module.css
.searchPill {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 220px;
  height: var(--height-widget-btn);
  padding: 0 10px;
  background-color: var(--bg-input);
  border: 1px solid var(--border-default);
  border-radius: 6px;
  color: var(--text-secondary);
  font-size: var(--font-size-ui);
  font-family: inherit;
  cursor: pointer;
  transition: border-color var(--transition-hover), background-color var(--transition-hover);
}
.searchPill:hover {
  border-color: var(--border-subtle);
  background-color: var(--bg-hover);
}
.searchShortcut {
  margin-left: auto;
  color: var(--text-tertiary);
  font-size: 11px;
}
```

```tsx
function SearchPill() {
  return (
    <button className={styles.searchPill} title="Search Everywhere (Shift Shift)">
      <svg width="12" height="12" viewBox="0 0 16 16">
        <circle cx="6.5" cy="6.5" r="4.5" stroke="currentColor" strokeWidth="1.5" fill="none"/>
        <line x1="10" y1="10" x2="14" y2="14" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"/>
      </svg>
      <span>Search</span>
      <span className={styles.searchShortcut}>⇧⇧</span>
    </button>
  );
}
```

### Pattern 5: TrafficLight CSS Module Migration

**What:** Replace JS hover (`useState` + `onMouseEnter`/`onMouseLeave`) on TrafficLight with CSS `:hover` since `WindowControls` is wrapped in `titlebar-no-drag`.

**When to use:** Any element that sits inside an explicit `titlebar-no-drag` container.

**Example:**
```tsx
// Header.module.css
.trafficLight {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  font-size: 9px;
  font-weight: 700;
  line-height: 1;
  color: transparent;
  transition: color var(--transition-hover);
}
.trafficLight:hover {
  color: rgba(0, 0, 0, 0.6);
}
```

This removes the `useState(false)` + event handlers entirely from `TrafficLight`.

### Pattern 6: JS Hover for Drag-Region Widgets

**What:** HeaderWidget buttons (hamburger, back/forward, project, VCS) sit directly in `titlebar-drag` and cannot use CSS `:hover` safely on macOS — WKWebView does not fire `mouseleave` after a window drag, leaving `:hover` stuck.

**When to use:** Any interactive element that is a direct child of a `titlebar-drag` container (not wrapped in `titlebar-no-drag`).

**Correct approach:** Use `onMouseEnter`/`onMouseLeave` on these specific elements. The CSS Module sets the base style; JS handlers mutate `style.backgroundColor` on hover.

**Alternative approach (also valid):** Wrap each left-group cluster in its own `titlebar-no-drag` span/div. This allows CSS `:hover` and removes JS event noise. The CONTEXT.md decision chose the pragmatic split approach — keep JS hover for drag-zone buttons — so this alternative is noted but not used.

### Anti-Patterns to Avoid

- **28px height on toolbar buttons:** The current `Header.tsx` uses `width/height: 28px` on both local `ToolBtn` and `ActionBtn`. Phase 3 changes this uniformly to `26px` via `var(--height-widget-btn)`.
- **Inline styles for layout:** After migration, zero inline `style={{}}` props should remain for static layout values. Dynamic CSS custom properties (`--_btn-bg`, `--_btn-hover-bg`) passed via `style` prop to `ToolBtn` are the sole exception.
- **Local ToolBtn/ActionBtn/Separator:** These local copies are deleted. The `src/primitives/` versions are the only source of truth.
- **Status text ("Running...", "Debugging...", "Paused"):** Explicitly dropped per CONTEXT.md. State is communicated by which buttons are visible.
- **Separator between run config and action buttons:** No separator here. Separator is only between groups.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| 26px button sizing | Custom button component | `ToolBtn size="small"` | Token `--height-widget-btn: 26px` already wires this |
| Filled action button hover | Inline `onMouseEnter`/`onMouseLeave` with bgColor | `ToolBtn variant="filled" hoverBgColor=...` | CSS custom property `--_btn-hover-bg` drives hover from CSS, no JS needed |
| Group divider | Local `<div style={{width:'1px'...}}>` | `<Separator variant="line" level="subtle" />` | Phase 2 primitive, already correct color and sizing |
| Traffic light hover state | `useState(false)` per circle | CSS `.trafficLight:hover { color: ... }` in CSS Module | Simpler; no hook; valid because WindowControls is `titlebar-no-drag` |

**Key insight:** Everything needed exists. The work is deletion of local copies and wiring to existing primitives, not construction of new infrastructure.

---

## Common Pitfalls

### Pitfall 1: CSS `:hover` Desync in macOS Drag Region

**What goes wrong:** After the user drags the window, WKWebView (the wry WebView backend on macOS) does not fire `mouseleave`. A button that was hovered before the drag stays in the `:hover` state visually until the mouse moves again. This looks broken.

**Why it happens:** `-webkit-app-region: drag` intercepts pointer events. The browser never sees the `mouseleave` that would clear `:hover`.

**How to avoid:** Use CSS `:hover` ONLY on elements inside `titlebar-no-drag` containers. Use JS `onMouseEnter`/`onMouseLeave` for any button that sits directly in a `titlebar-drag` container.

**Warning signs:** After dragging the window, a toolbar button remains highlighted. This is the symptom.

**Scope for Phase 3:** The right group (run config, action buttons, search pill, settings gear) all sit inside `titlebar-no-drag` — CSS `:hover` is safe here. The left group (hamburger, back/forward, HeaderWidgets) sits in `titlebar-drag` — JS hover required.

### Pitfall 2: `titlebar-no-drag` Inheritance Gap

**What goes wrong:** Setting `titlebar-no-drag` on a parent container is not sufficient if child elements have their own `-webkit-app-region` styles. Also, omitting `titlebar-no-drag` from a button's ancestor means clicks are swallowed by the drag handler.

**Why it happens:** `-webkit-app-region` is inherited but can be overridden. wry drag capture happens at the OS level before React click events.

**How to avoid:** Every interactive element (button, input) must have a `titlebar-no-drag` ancestor. The safest pattern is wrapping interactive clusters in `<div className="titlebar-no-drag">`.

**Warning signs:** Button renders visually but `onClick` never fires.

### Pitfall 3: Height Token Mismatch (28px vs 26px)

**What goes wrong:** The current Header.tsx uses `height: 28px` on local ToolBtn and ActionBtn, while the design requirement and token `--height-widget-btn` specify 26px. If any button retains 28px after migration, the toolbar looks uneven.

**Why it happens:** Local components were built before the primitives with the correct sizing token.

**How to avoid:** After replacing all local buttons with `ToolBtn size="small"`, verify in DevTools that computed height is 26px on every toolbar button.

**Warning signs:** TBAR-01 verification step (DevTools measurement) returns 28px on any button.

### Pitfall 4: CSS Module Class Import for Sub-Components

**What goes wrong:** Sub-components defined in `Header.tsx` (TrafficLight, SearchPill, etc.) need access to `Header.module.css` classes. If styles are imported at the top of the file, all sub-functions in the same module share the same `styles` object — this works correctly, but only if each sub-component uses the shared `styles` reference, not a local re-import.

**Why it happens:** Confusion about whether sub-functions need their own CSS Module imports.

**How to avoid:** Single `import styles from './Header.module.css'` at the top of `Header.tsx`. All local sub-components in the same file reference `styles.className` directly. No re-imports needed.

**Warning signs:** TypeScript error on `styles.someClass` inside a sub-function.

### Pitfall 5: Debug Controls Separator Placement

**What goes wrong:** The current Header.tsx puts a `<Separator />` after the stop button in the paused debug state. The CONTEXT.md decision removes the separator between Stop and Resume/stepping buttons — the separator appears BEFORE the debug group (after run config), not between Stop and Resume.

**Why it happens:** Copying the old structure without applying the re-grouping decision.

**How to avoid:** When debugging and paused, render: `[run config ▾]` | `[⬛ Stop]` | `[▶ Resume] [⤵ Step Over] [↓ Step Into] [↑ Step Out]`. The separator between Stop and Resume group uses `<Separator variant="line" level="subtle" />`. The "Running..."/"Debugging..."/"Paused" `<span>` is removed entirely.

---

## Code Examples

### Full Toolbar Layout Skeleton

```tsx
// Header.tsx — after migration
export function Header() {
  const activeTabId = useStore((s) => s.activeTabId);
  const tabs = useStore((s) => s.tabs);
  const activeTab = tabs.find((t) => t.scriptId === activeTabId);
  const isRunning = useStore((s) => s.isRunning);
  const isDebugging = useStore((s) => s.isDebugging);
  const isPaused = useStore((s) => s.isPaused);
  const active = isRunning || isDebugging;

  return (
    <div className={`titlebar-drag ${styles.toolbar}`}>
      {/* macOS traffic lights — in no-drag zone, CSS :hover safe */}
      <WindowControls />

      <Separator variant="line" level="subtle" />

      {/* Hamburger — in drag zone, JS hover */}
      <ToolBtn size="small" title="Menu" className={styles.dragZoneBtn}
        onMouseEnter={...} onMouseLeave={...}>
        {/* hamburger SVG */}
      </ToolBtn>

      <Separator variant="line" level="subtle" />

      {/* Back/Forward — in drag zone, JS hover */}
      <ToolBtn size="small" title="Back" className={styles.dragZoneBtn} ...>...</ToolBtn>
      <ToolBtn size="small" title="Forward" className={styles.dragZoneBtn} ...>...</ToolBtn>

      <Separator variant="line" level="subtle" />

      {/* Project + VCS — no separator between them, both JS hover */}
      <HeaderWidget icon={...} label="VOID//SCRIPT" hasDropdown />
      <HeaderWidget icon={...} label="main" muted />

      {/* Flex spacer — draggable */}
      <div className={styles.spacer} />

      {/* Right interactive group — all inside no-drag, CSS :hover safe */}
      <div className={`titlebar-no-drag ${styles.rightGroup}`}>
        <RunConfigSelector label={activeTab ? `${activeTab.name}.vs` : 'No configuration'} />

        <Separator variant="line" level="subtle" />

        {!active ? (
          <>
            <ToolBtn size="small" variant="filled" title="Run"
              bgColor="var(--bg-btn-run)" hoverBgColor="var(--bg-btn-run-hover)"
              iconColor="var(--icon-run)"
              onClick={() => activeTabId && sendToRust({ type: 'run_script', script_id: activeTabId })}
              disabled={!activeTabId}>
              {/* run SVG */}
            </ToolBtn>
            <ToolBtn size="small" variant="filled" title="Debug"
              bgColor="var(--bg-btn-debug)" hoverBgColor="var(--bg-btn-debug-hover)"
              iconColor="var(--icon-debug)"
              onClick={() => activeTabId && sendToRust({ type: 'debug_start', script_id: activeTabId })}
              disabled={!activeTabId}>
              {/* debug SVG */}
            </ToolBtn>
          </>
        ) : (
          <>
            <ToolBtn size="small" variant="filled" title="Stop"
              bgColor="var(--bg-btn-stop)" hoverBgColor="var(--bg-btn-stop-hover)"
              iconColor="var(--icon-stop)"
              onClick={() => activeTabId && sendToRust({ type: 'stop_script', script_id: activeTabId })}>
              {/* stop SVG */}
            </ToolBtn>
            {isDebugging && isPaused && (
              <>
                <Separator variant="line" level="subtle" />
                <ToolBtn size="small" variant="filled" title="Resume"
                  bgColor="var(--bg-btn-run)" hoverBgColor="var(--bg-btn-run-hover)"
                  iconColor="var(--icon-run)"
                  onClick={() => activeTabId && sendToRust({ type: 'debug_continue', script_id: activeTabId })}>
                  {/* resume SVG */}
                </ToolBtn>
                <ToolBtn size="small" title="Step Over"
                  onClick={() => activeTabId && sendToRust({ type: 'debug_step_over', script_id: activeTabId })}>
                  {/* step over SVG */}
                </ToolBtn>
                <ToolBtn size="small" title="Step Into"
                  onClick={() => activeTabId && sendToRust({ type: 'debug_step_into', script_id: activeTabId })}>
                  {/* step into SVG */}
                </ToolBtn>
                <ToolBtn size="small" title="Step Out"
                  onClick={() => activeTabId && sendToRust({ type: 'debug_step_out', script_id: activeTabId })}>
                  {/* step out SVG */}
                </ToolBtn>
              </>
            )}
          </>
        )}

        <Separator variant="line" level="subtle" />

        <SearchPill />

        <ToolBtn size="small" title="Settings">
          {/* gear SVG */}
        </ToolBtn>
      </div>
    </div>
  );
}
```

### ToolBtn Primitive: `variant="filled"` Contract

```tsx
// ToolBtn handles filled hover via CSS custom property — no JS handlers needed
<ToolBtn
  size="small"           // → width/height: var(--height-widget-btn) = 26px
  variant="filled"       // → .filled class, reads --_btn-bg and --_btn-hover-bg
  bgColor="var(--bg-btn-run)"          // → sets --_btn-bg via style prop
  hoverBgColor="var(--bg-btn-run-hover)" // → sets --_btn-hover-bg via style prop
  iconColor="var(--icon-run)"          // → sets color via style prop
  title="Run"
  disabled={!activeTabId}
  onClick={...}
>
  {children}
</ToolBtn>
```

Note: `ToolBtn` does NOT accept `onMouseEnter`/`onMouseLeave` props. Hover for `filled` variant is CSS-driven via `--_btn-hover-bg`. Ghost hover is `var(--bg-hover)` in `.btn:hover:not(:disabled)`.

### Header.module.css Skeleton

```css
/* Header.module.css */

/* Outer container */
.toolbar {
  display: flex;
  align-items: center;
  height: var(--height-titlebar);
  background-color: var(--bg-toolbar);
  padding: 0 8px;
  user-select: none;
  border-bottom: 1px solid var(--border-strong);
  font-size: var(--font-size-ui);
}

/* Flex spacer — draggable gap */
.spacer {
  flex: 1;
}

/* Right interactive cluster */
.rightGroup {
  display: flex;
  align-items: center;
  gap: 4px;
}

/* Traffic lights container */
.trafficLights {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 4px;
}

/* Individual traffic light circle */
.trafficLight {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  font-size: 9px;
  font-weight: 700;
  line-height: 1;
  color: transparent;
  transition: color var(--transition-hover);
}
.trafficLight:hover {
  color: rgba(0, 0, 0, 0.6);
}

/* Compound header widget (project, VCS branch) */
.widget {
  display: flex;
  align-items: center;
  gap: 6px;
  height: var(--height-widget-btn);
  padding: 0 8px;
  background: none;
  border: none;
  border-radius: 6px;
  color: var(--text-primary);
  cursor: pointer;
  font-size: var(--font-size-ui);
  font-family: inherit;
  font-weight: 600;
  transition: background-color var(--transition-hover);
}
.widgetMuted {
  color: var(--text-secondary);
  font-weight: 400;
}
.widgetIcon {
  display: flex;
  align-items: center;
  color: var(--text-tertiary);
}
.widgetChevron {
  color: var(--text-tertiary);
}

/* Run configuration selector */
.runConfig {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 10px;
  background-color: var(--bg-run-config);
  border: none;
  border-radius: 6px;
  color: var(--text-primary);
  height: var(--height-widget-btn);
  cursor: pointer;
  font-size: var(--font-size-ui);
  font-family: inherit;
  transition: background-color var(--transition-hover);
}
.runConfig:hover {
  background-color: var(--border-subtle);
}
.runConfigIcon {
  color: var(--text-secondary);
  display: flex;
  align-items: center;
}
.runConfigChevron {
  color: var(--text-tertiary);
  display: flex;
  align-items: center;
}

/* Search Everywhere pill */
.searchPill {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 220px;
  height: var(--height-widget-btn);
  padding: 0 10px;
  background-color: var(--bg-input);
  border: 1px solid var(--border-default);
  border-radius: 6px;
  color: var(--text-secondary);
  font-size: var(--font-size-ui);
  font-family: inherit;
  cursor: pointer;
  transition: border-color var(--transition-hover), background-color var(--transition-hover);
}
.searchPill:hover {
  border-color: var(--border-subtle);
  background-color: var(--bg-hover);
}
.searchShortcut {
  margin-left: auto;
  color: var(--text-tertiary);
  font-size: 11px;
}
```

### JS Hover for Drag-Zone Buttons

For buttons that sit directly in `titlebar-drag` (hamburger, back/forward, HeaderWidget):

```tsx
// Pattern: JS hover targeting style.backgroundColor
// The button has no CSS :hover rule — hover bg is applied by JS only
<button
  className={`titlebar-no-drag ${styles.widget}`}  // base layout from CSS Module
  onMouseEnter={(e) => { e.currentTarget.style.backgroundColor = 'var(--bg-hover)'; }}
  onMouseLeave={(e) => { e.currentTarget.style.backgroundColor = ''; }}
  ...
>
```

Note: `titlebar-no-drag` class on HeaderWidget buttons makes them clickable even though the parent container is `titlebar-drag`. The JS hover handlers are used instead of CSS `:hover` to avoid the WKWebView desync.

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `ActionBtn` local component with inline hover handlers | `ToolBtn variant="filled"` with `--_btn-hover-bg` CSS custom property | Phase 2 | No JS hover needed for filled buttons |
| Local `Separator` with inline styles | `Separator` primitive with `variant`/`level` props | Phase 2 | Consistent separator sizing and color |
| `useState(false)` in TrafficLight for hover | CSS `:hover` in CSS Module | Phase 3 (this phase) | Removes hook, cleaner code |
| 28px button heights in local components | `var(--height-widget-btn)` = 26px via `ToolBtn size="small"` | Phase 3 (this phase) | TBAR-01 compliance |

**Deprecated/outdated in this phase:**
- Local `ToolBtn` function in `Header.tsx`: replaced by `import { ToolBtn } from '../primitives/ToolBtn'`
- Local `ActionBtn` function in `Header.tsx`: deleted, functionality absorbed by `ToolBtn variant="filled"`
- Local `Separator` function in `Header.tsx`: replaced by `import { Separator } from '../primitives/Separator'`
- `useState(false)` + `onMouseEnter`/`onMouseLeave` in `TrafficLight`: replaced by CSS `:hover`
- Status text span ("Running..."/"Debugging..."/"Paused"): removed per CONTEXT.md decision

---

## Open Questions

1. **HeaderWidget hover: JS on element vs. wrapping in titlebar-no-drag**
   - What we know: CONTEXT.md locks in JS hover for drag-zone buttons. Both approaches are technically valid.
   - What's unclear: Whether to put `titlebar-no-drag` on each individual `HeaderWidget` button (making CSS `:hover` safe) or keep them in `titlebar-drag` and use JS handlers.
   - Recommendation: Use `titlebar-no-drag` on each `HeaderWidget` button directly (the class is already on the `<button>` element in the current code). This makes the button clickable AND allows CSS `:hover`. The parent `div` stays `titlebar-drag` for drag behavior. This is slightly cleaner than pure JS hover.

2. **Search pill width: 200px vs 220px vs 240px**
   - What we know: CONTEXT.md says ~200-240px range, with 220px matching Rider's proportions well.
   - Recommendation: 220px. This fits "Search" + "⇧⇧" hint without clipping at 13px font size.

---

## Validation Architecture

> `nyquist_validation` is `true` in `.planning/config.json` — this section is included.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | None detected — no jest.config, vitest.config, or test files in project |
| Config file | None — see Wave 0 |
| Quick run command | N/A — no test runner installed |
| Full suite command | N/A — no test runner installed |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| TBAR-01 | Toolbar buttons render at 26px height | visual/manual | DevTools computed height check | ❌ Wave 0 |
| TBAR-02 | Separators appear between correct groups, font weights correct | visual/manual | DevTools inspection | ❌ Wave 0 |
| TBAR-03 | Search pill visible, shows magnifying glass + "Search" + "⇧⇧" | visual/manual | Visual inspection in app | ❌ Wave 0 |
| TBAR-04 | Settings gear visible at far-right | visual/manual | Visual inspection in app | ❌ Wave 0 |

**Note:** All four TBAR requirements are visual/layout requirements verifiable only through browser DevTools or visual inspection. The success criteria explicitly call out DevTools measurement. No unit test framework is installed and no test would add meaningful coverage for pure visual/CSS work. Wave 0 should document this as manual-only with DevTools steps.

### Sampling Rate

- **Per task commit:** `npm run build` in `editor-ui/` — TypeScript + Vite build clean
- **Per wave merge:** Full visual inspection against Rider reference screenshots
- **Phase gate:** All 4 TBAR requirements visually verified before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] No test framework installed — all TBAR verifications are manual DevTools + visual
- [ ] Consider adding vitest if future phases need component unit tests (out of scope for Phase 3)

---

## Sources

### Primary (HIGH confidence)

- Direct code inspection of `/editor-ui/src/components/Header.tsx` — current state of all 7 sub-components
- Direct code inspection of `/editor-ui/src/primitives/ToolBtn.tsx` + `ToolBtn.module.css` — confirmed `size="small"` maps to `var(--height-widget-btn)`, `variant="filled"` uses `--_btn-hover-bg`
- Direct code inspection of `/editor-ui/src/primitives/Separator.tsx` + `Separator.module.css` — confirmed `variant`, `level`, `orientation` props
- Direct code inspection of `/editor-ui/index.html` — confirmed `--height-widget-btn: 26px` token, `titlebar-drag`/`titlebar-no-drag` CSS classes
- Direct code inspection of `/editor-ui/src/state/store.ts` — confirmed `isRunning`, `isDebugging`, `isPaused`, `activeTabId` available
- `/Users/dakmor/.claude/projects/memory/MEMORY.md` — confirmed wry/WKWebView requires CSS tokens inlined in index.html (already done)

### Secondary (MEDIUM confidence)

- CONTEXT.md decisions — locked by user in `/gsd:discuss-phase` session

### Tertiary (LOW confidence)

- WKWebView `:hover` desync behavior described in CONTEXT.md — noted as "known WKWebView desync where `:hover` gets stuck after a window drag"; not independently re-verified against wry docs in this research session but consistent with known WebKit behavior

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all primitives directly inspected; no external packages needed
- Architecture: HIGH — CSS Module patterns established in Phase 2; layout skeleton derived from existing Header.tsx
- Pitfalls: HIGH — drag-region hover issue documented in project CONTEXT.md and MEMORY.md; height mismatch (28→26px) confirmed by reading current source

**Research date:** 2026-03-14
**Valid until:** 2026-04-13 (30 days — stable CSS/React domain)
