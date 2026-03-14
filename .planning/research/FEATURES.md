# Feature Research

**Domain:** IDE-like code editor UI — JetBrains Rider New UI pixel-accurate recreation
**Researched:** 2026-03-14
**Confidence:** MEDIUM-HIGH (official JetBrains docs verified; pixel measurements unavailable from docs, inferred from code inspection)

---

## Context

The editor shell already works: tabs, panels, run/debug, scripts list, status bar, gutter. This milestone adds visual polish and missing UI elements to match Rider's New UI exactly. Features below are scoped to the _visual/UI layer only_ — game mechanics are explicitly out of scope.

Reference: https://www.jetbrains.com/help/rider/New_UI.html

---

## Feature Landscape

### Table Stakes (Users Expect These)

Features that any developer opening this editor will immediately notice are missing or wrong. Without these, the editor feels like a prototype and undermines the "professional IDE in a game" premise.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| **Title bar correct height and widget proportions** | Rider's title bar is ~40px with 26px tall widget buttons; current implementation exists but lacks pixel-perfect alignment (28px buttons, 40px bar height needs verification against Rider). Devs notice instantly. | LOW | Current: 40px bar, 28px buttons. Rider: ~26px widget buttons with more horizontal padding. Adjust `ToolBtn` and `ActionBtn` heights. |
| **Search Everywhere button in title bar** | Rider prominently places a magnifying-glass icon button in the toolbar as a key entry point. Missing from current implementation. | LOW | Render as a `ToolBtn` icon in the center-right of the title bar, before the run configuration selector. Triggers a modal/popup overlay on click. Keyboard: Shift+Shift or Ctrl+N. |
| **Settings gear icon in toolbar** | Every JetBrains product shows a gear icon (IDE settings) as a persistent toolbar element top-right. Currently absent. | LOW | Rightmost item after run controls. Opens a dropdown (or no-op for now). |
| **Breadcrumb bar below tab bar** | Rider shows a breadcrumb strip at the bottom of the editor area (default) or top, showing the structural path of the caret position (e.g. namespace > class > method). Critical for the "feels like a real IDE" perception. | MEDIUM | New React component `Breadcrumb.tsx` below `TabBar`. Requires CodeMirror integration to read current syntax node hierarchy via `syntaxTree()` from `@codemirror/language`. Shows static placeholder when no symbol context available. |
| **Tab bar with correct Rider sizing** | Rider New UI tabs are notably taller and more spacious than classic IDE tabs. Current tabs use `minHeight: 36px` and `fontSize: 13px`. Rider New UI: tabs are ~36–38px with padding `0 16px`, font-weight normal for inactive, no bold active state difference — the selection is communicated only by the 2px blue bottom border and background fill. | LOW | Tune `padding` from `0 14px` to `0 16px`, increase `minHeight` to 38px. Verify close-button is hidden on inactive tabs until hover. |
| **Close button hidden on inactive tabs until hover** | Rider hides the close `×` on inactive tabs; it appears only on hover (and always shows on active tab). Current implementation shows close button always. Prominent visual difference. | LOW | CSS hover state with conditional rendering or opacity transition on the close span. |
| **Gutter: breakpoints overlay line numbers (not beside)** | Rider New UI default: breakpoints render over the line number gutter, not in a separate column, saving horizontal space. Rider 2024.2 made this the default. Current implementation uses a separate `cm-breakpoint-gutter` column. | MEDIUM | Replace the separate breakpoint gutter with a combined line-number gutter that overlays breakpoint markers on top of line numbers via custom `GutterMarker` rendering. Requires reworking breakpoint gutter in `Editor.tsx`. |
| **Gutter: fold icons appear on hover only** | Rider default is hover-only fold arrows. Currently the `foldGutter()` extension shows icons always. | LOW | Pass `{ openText: '▾', closedText: '▸' }` to `foldGutter()` and apply CSS `.cm-foldGutter .cm-gutterElement { opacity: 0 }` with `:hover` reveal on the gutter element. CSS-only solution. |
| **Tool window strip icons at correct size** | Rider New UI uses ~22px icons in 40px strip buttons (larger than the old 16px/28px icons), with icons that visually describe the panel (e.g., a list icon for Scripts, a bug icon for Debug, terminal icon for Console). Current strip is 36px wide with 30px buttons — slightly narrow. | LOW | Expand `ToolStrip` width from 36px to 40px; button size from 30px to 36px; improve icon quality. |
| **Tool window header with panel title text** | Rider tool panels show a text header row at the top (e.g., "Scripts", "Debug", "Console") with actions pinned right. Current panels have no header row — content starts immediately. | MEDIUM | Add a `PanelHeader` component with title text and right-aligned action icons (e.g., close, settings gear, collapse). Rider style: 28px header height, 11px uppercase-ish label text, `#2B2D30` background, bottom `1px solid #1E1F22` separator. |
| **Status bar: navigation breadcrumb path** | Rider New UI relocated the navigation bar to the status bar left side. It shows the path from project root to current file/symbol. Current status bar left shows only VCS branch. The navigation bar should show file path segments clickable for quick navigation. | MEDIUM | Extend `StatusBar.tsx` left region: navigation segments (`VOID//SCRIPT > scripts > filename`) as clickable `StatusSegment` items. Static for now (no actual navigation popup), but must render. |
| **Status bar: problems icon with count** | Rider status bar shows an errors/warnings icon widget with total counts. Current implementation conditionally shows text. Should be icon + number, matching Rider's dedicated problem indicator widget. | LOW | Replace inline text with icon+count pattern (error circle icon + red count, warning triangle icon + yellow count). |
| **Hover states on all interactive elements** | Rider has consistent 150ms background transitions on all clickable elements. Some current components use inline `onMouseEnter/Leave` inconsistently (ToolStrip uses no transition, Header uses none). | LOW | Audit all components; extract shared hover CSS class or consistent inline style with `transition: background-color 0.15s`. |
| **Consistent border and separator colors** | Rider uses a 3-level color hierarchy: `#1E1F22` (deepest borders), `#393B40` (element separators), `#43454A` (subtle dividers). Current code mixes these somewhat correctly but `ToolStrip` border uses `#1E1F22` as separator which is the background — making it invisible against panel backgrounds. | LOW | Audit border colors across all components. `#393B40` for visible separators between panels, `#1E1F22` only for outer window edges. |
| **Inter font applied universally** | Rider New UI uses Inter for all UI elements across all OS. The base font stack needs explicit `Inter` loading. Currently relies on CSS `font-family: inherit` chain which may fall back to system sans-serif. | LOW | Load Inter via `@fontsource/inter` or Google Fonts in `index.html`. Apply `font-family: 'Inter', system-ui, sans-serif` to `:root`. |
| **Tooltip styling matching Rider** | Hover tooltips (title attributes rendered as browser native) should be styled to match Rider: dark background `#2B2D30`, rounded 4px corners, 11px Inter text, subtle `#43454A` border. Native browser tooltips cannot be styled — requires custom tooltip component. | MEDIUM | Replace `title=""` props with a custom `Tooltip` component using CSS positioning. Apply Rider dark tooltip colors. |
| **Bottom panel tabs matching Rider style** | Bottom panel (Console) tab strip matches the Rider bottom tool window tab style: text tabs with icon, active tab highlighted, 28px tab height. Current Console has no tab strip — it's a single panel. Need tab structure for future Console/Output/Problems tabs. | MEDIUM | Add a tab strip to the bottom panel region. Even with only "Console" tab, the chrome must match Rider. Uses same 2px bottom-border active indicator style as top tab bar but on a darker base. |

### Differentiators (Competitive Advantage)

Features beyond table stakes that would make the editor feel distinctly impressive within the game context — not strictly required for the Rider match, but elevate quality.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **Search everywhere modal with script search** | A working Search Everywhere popup that searches script names and symbols. Players don't need full IDE search, but finding scripts by name is immediately useful. Rider-accurate modal UI (dark overlay, centered modal, tabbed results). | HIGH | Requires fuzzy-search logic over script list (already in store). UI is a centered overlay with search input + result list. Non-trivial but high payoff. Defer to v1.x unless simple script-name-only filter can be done first. |
| **Compact mode toggle** | Rider offers a compact mode (reduced heights, smaller icons). For the game's cockpit/HUD context, a "compact" density that uses less vertical space may be desirable. Accessible via a settings dropdown. | MEDIUM | CSS custom property for density: `--ui-density: normal|compact`. In compact mode, title bar ~34px, tabs ~30px, status bar ~22px, strip buttons ~28px. Toggle stored in UI state. |
| **VCS-style gutter change indicators** | Rider shows colored left-border bars in the gutter for modified/added/deleted lines. For VoidScript, this could indicate "lines changed since last run" — a game-relevant semantic. | HIGH | Requires tracking content delta between last run and current state. Needs new store field and CodeMirror decoration layer. Genuinely useful UX but high complexity for a game editor. |
| **Animated status bar execution progress** | During script execution, Rider shows a progress indicator in the status bar. An animated spinner or progress bar in the status bar left region during run/debug state provides satisfying feedback. | LOW | Already have `isRunning`/`isDebugging` state. Add a CSS spinner or indeterminate progress bar to `StatusBar` left region when active. Low complexity, high polish impact. |
| **Keyboard shortcut hints in tooltips** | Rider shows shortcut keys in tooltips (e.g., "Run (Shift+F10)"). Adding shortcut hints to the Run/Debug/Stop button tooltips matches Rider exactly and helps players learn keyboard shortcuts. | LOW | Extend the `ActionBtn` title prop to include shortcut hint. Pure string change. |
| **Panel resize handles with hover feedback** | The splitter between the editor area and side panels should show a resize cursor and a highlighted 2px drag handle on hover, matching Rider's panel resize affordance. | MEDIUM | CSS cursor change on `resize` region; custom drag handle component. Currently panels may use browser-default behavior or none. |

### Anti-Features (Commonly Requested, Often Problematic)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| **Light theme** | "Can you add a light mode?" — common UI request | Doubles CSS token maintenance; game is set in space — dark theme is canonical; PROJECT.md explicitly excludes it for this milestone | Stay dark. Add a `--color-*` token layer that would _allow_ light theme later without doing it now. |
| **Full Search Everywhere (classes, files, actions, text)** | Rider has comprehensive search; seems obvious to replicate | VoidScript has no class/namespace system yet; indexing actions and settings is entire IDE infrastructure; massive scope creep | Implement only script-name search as a simplified modal. Label it "Go to Script" not "Search Everywhere" to set expectations. |
| **Tab overflow: multi-row tabs** | Users ask for this when too many tabs are open | Multi-row tabs double the tab bar height and push editor content down significantly; Rider itself discourages this in New UI (recommends scroll or squeeze instead) | Implement tab scrolling (horizontal scroll within the tab bar) with left/right scroll arrows appearing when overflowed. |
| **Floating tool windows** | Power users want detachable panels | Webview inside a game window cannot spawn child windows easily; wry/Bevy don't expose multi-window with shared state; massive architectural complexity for marginal benefit | Keep all panels docked. Allow panel width resize. Defer detach to a future milestone if ever. |
| **Plugin/extension system** | Developers want extensibility | This is a game editor, not a general IDE; plugin API surface is enormous; premature abstraction will slow down feature work | Build the UI features needed for the game first. Extract extension points only when a concrete second use-case appears. |
| **Real-time collaborative editing** | Multiplayer games suggest "co-edit the same script" | CRDTs, operational transforms, presence indicators — massive infrastructure for an edge case; game design shows PvP as a future milestone, not cooperative scripting | Implement save/load sync through the Rust backend when multiplayer is designed. Don't architect for it now. |
| **Minimap (code overview)** | VSCode users expect a minimap | Adds ~80px to the right side; VoidScript files are small (game scripts); minimap provides zero value at this code scale; clutters the minimal Rider aesthetic | Rider itself doesn't show minimap by default. Skip it entirely. Use CodeMirror's fold-gutter for structural overview. |

---

## Feature Dependencies

```
Search Everywhere modal
    └──requires──> Script list in store (already exists)
    └──requires──> Modal overlay component (new)

Breadcrumb bar
    └──requires──> CodeMirror syntaxTree() integration (new)
    └──requires──> Active tab content in store (already exists)
    └──enhances──> Status bar navigation path (shared context concept)

Breakpoint overlay (gutter refactor)
    └──requires──> Rework of cm-breakpoint-gutter in Editor.tsx
    └──conflicts──> Separate breakpoint gutter column (current impl)

Panel header component
    └──requires──> Consistent panel structure across ScriptList, DebugPanel, Console
    └──enhances──> Tool window strip icons (same visual system)

Tooltip component
    └──requires──> Custom Tooltip wrapper component
    └──enhances──> Keyboard shortcut hints in tooltips (same component)
    └──replaces──> Native browser title="" attributes (current impl)

Inter font loading
    └──required-by──> All other typography work (must land first)
    └──blocks──> None (can land independently)

Compact mode
    └──requires──> CSS custom property token system (--ui-density)
    └──enhances──> All size/spacing work (those sizes become the token values)
```

### Dependency Notes

- **Inter font must land before typography audits:** Spacing measurements look different with system sans-serif vs Inter. Load the font first, then tune spacings.
- **Breadcrumb requires CodeMirror syntaxTree:** The breadcrumb component is worthless without real cursor-position data. A static mock is acceptable in early iterations but the real integration is non-trivial.
- **Breakpoint overlay refactor conflicts with current gutter:** Cannot incrementally migrate — the gutter column separation must be removed and replaced in one change to `Editor.tsx`.
- **Panel header requires touching all three panel components:** `ScriptList.tsx`, `DebugPanel.tsx`, and the Console section all need consistent header chrome. Plan as a single coordinated change to avoid visual inconsistency mid-milestone.
- **Tooltip component replaces all `title=""` props:** This is a sweep across every component. Scope carefully — do it last after other components are stable to minimize re-work.

---

## MVP Definition

### Launch With (v1) — This Milestone's Must-Haves

Features that together make the editor indistinguishable from a real Rider window in a screenshot.

- [ ] **Inter font loading** — typography foundation; everything else depends on it
- [ ] **Title bar pixel-accurate sizing** — widget heights, font weights, separator positions; the most-scrutinized surface
- [ ] **Search Everywhere button** — icon in toolbar; prominent enough that its absence is noticed instantly
- [ ] **Settings gear icon** — top-right, static; two-line change but visually expected
- [ ] **Close button hidden on inactive tabs (hover-reveal)** — very visible Rider behavior; easy CSS fix
- [ ] **Tab bar sizing refinement** — height 38px, padding `0 16px`, correct font weight
- [ ] **Gutter fold icons on hover only** — CSS + foldGutter config change; low effort, high authenticity
- [ ] **Tool window strip width/button sizing** — 40px wide, 36px buttons, larger icons
- [ ] **Panel header strip** — title + actions row on ScriptList, DebugPanel, Console
- [ ] **Status bar: navigation path left segment** — static breadcrumb path, no popup
- [ ] **Status bar: icon+count diagnostics widget** — replace plain text with icon pattern
- [ ] **Consistent border/separator colors** — audit pass across all components
- [ ] **Hover state transitions** — 150ms ease on all interactive elements
- [ ] **Bottom panel tab strip** — even single "Console" tab must have Rider tab chrome

### Add After Validation (v1.x)

- [ ] **Breadcrumb bar** — add once Inter font and layout are stable; needs CodeMirror integration
- [ ] **Gutter breakpoint overlay** — medium refactor; add once other gutter work is done
- [ ] **Custom tooltip component** — add after all layout work stabilizes
- [ ] **Keyboard shortcut hints** — add with tooltip component
- [ ] **Animated status bar progress** — quick win, add after status bar layout is finalized
- [ ] **Panel resize handles** — add when panels are otherwise complete

### Future Consideration (v2+)

- [ ] **Search Everywhere modal with fuzzy search** — needs VoidScript symbol index; defer until scripting system matures
- [ ] **Compact mode toggle** — defer until game's HUD/cockpit context is designed
- [ ] **VCS-style gutter change indicators** — defer until runtime execution history is tracked in game

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Inter font loading | HIGH | LOW | P1 |
| Title bar sizing precision | HIGH | LOW | P1 |
| Close button hover-reveal on tabs | HIGH | LOW | P1 |
| Tab bar sizing refinement | HIGH | LOW | P1 |
| Search Everywhere button | HIGH | LOW | P1 |
| Settings gear icon | MEDIUM | LOW | P1 |
| Gutter fold icons on hover | MEDIUM | LOW | P1 |
| Tool window strip sizing | MEDIUM | LOW | P1 |
| Panel header row | HIGH | MEDIUM | P1 |
| Status bar navigation path | MEDIUM | MEDIUM | P1 |
| Status bar diagnostics icon widget | MEDIUM | LOW | P1 |
| Hover transitions audit | HIGH | LOW | P1 |
| Bottom panel tab strip | HIGH | MEDIUM | P1 |
| Border/separator color audit | MEDIUM | LOW | P1 |
| Breadcrumb bar | HIGH | MEDIUM | P2 |
| Gutter breakpoint overlay | MEDIUM | MEDIUM | P2 |
| Custom tooltip component | MEDIUM | MEDIUM | P2 |
| Animated status bar progress | LOW | LOW | P2 |
| Panel resize handles | MEDIUM | MEDIUM | P2 |
| Search Everywhere modal | HIGH | HIGH | P3 |
| Compact mode toggle | LOW | MEDIUM | P3 |
| VCS gutter change indicators | LOW | HIGH | P3 |

**Priority key:**
- P1: Must have for this milestone (pixel-accurate Rider match)
- P2: Should add once P1 is stable
- P3: Nice to have, future milestone

---

## Competitor Feature Analysis

The "competitor" here is JetBrains Rider itself — the reference implementation to match.

| UI Element | Rider New UI (Reference) | Current VOID//SCRIPT State | Delta |
|------------|--------------------------|---------------------------|-------|
| Title bar height | ~40px | 40px | Match — but widget sizes need tuning |
| Widget button height | ~26px, rounded 6px | 28px, rounded 6px | 2px difference |
| Tab bar height | ~38px | 36px | 2px short |
| Tab padding | `0 16px` | `0 14px` | 2px per side short |
| Close button | Hover-reveal on inactive tabs | Always visible | Missing behavior |
| Breadcrumb bar | Yes, below tabs, bottom of editor area | Absent | Missing component |
| Search Everywhere | Icon button in title bar center-right | Absent | Missing widget |
| Settings gear | Icon button at toolbar far right | Absent | Missing widget |
| Gutter fold icons | Hover-only by default | Always visible (foldGutter default) | Config change needed |
| Gutter breakpoints | Overlay on line numbers | Separate column | Gutter refactor needed |
| Tool strip width | ~40px wide, ~22px icons | 36px wide, icon-only | Width + icon size |
| Panel header | Text label + action icons row | None | Missing component |
| Status bar left | Navigation path (file > symbol) | VCS branch only | Segment missing |
| Status bar diagnostics | Icon + count widget | Text only | Visual style |
| Bottom panel | Tabbed with Rider tab chrome | Unstyled single panel | Tab strip missing |
| Tooltips | Custom dark styled tooltips | Native browser title= | Unstyled |
| Font | Inter (explicit load) | Inherited / system fallback | Font load missing |
| Hover transitions | 150ms ease on all elements | Inconsistent | Audit needed |

---

## Sources

- [JetBrains Rider New UI documentation](https://www.jetbrains.com/help/rider/New_UI.html)
- [JetBrains Rider UI overview](https://www.jetbrains.com/help/rider/Guided_Tour_Around_the_User_Interface.html)
- [Editor Breadcrumbs — Rider docs](https://www.jetbrains.com/help/rider/Editor_breadcrumbs.html)
- [Search Everywhere — Rider docs](https://www.jetbrains.com/help/rider/Searching_Everywhere.html)
- [Editor Gutter — Rider docs](https://www.jetbrains.com/help/rider/Editor-gutter.html)
- [Tool Windows — Rider docs](https://www.jetbrains.com/help/rider/Tool_Windows.html)
- [Appearance settings — Rider docs](https://www.jetbrains.com/help/rider/Settings_Appearance.html)
- [Rider 2021.3 Brand New Main Toolbar — JetBrains blog](https://blog.jetbrains.com/dotnet/2021/11/16/rider-2021-3-brand-new-main-toolbar/)
- [Rider 2024.2 release notes — What's New](https://www.jetbrains.com/rider/whatsnew/2024-2/)
- Code inspection of existing components: `Header.tsx`, `TabBar.tsx`, `StatusBar.tsx`, `ToolStrip.tsx`, `Editor.tsx`, `voidscript-theme.ts`

---

*Feature research for: IDE-like code editor UI, JetBrains Rider New UI recreation*
*Researched: 2026-03-14*
