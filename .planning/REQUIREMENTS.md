# Requirements: VOID//SCRIPT Editor — Rider New UI Restyle

**Defined:** 2026-03-14
**Core Value:** The code editor must look and feel like JetBrains Rider's New UI — professional, polished, and immediately familiar to developers.

## v1 Requirements

Requirements for the Rider New UI pixel-accurate restyle. Each maps to roadmap phases.

### Foundation

- [x] **FOUN-01**: Inter font self-hosted via Fontsource and applied universally to all UI elements
- [x] **FOUN-02**: JetBrains Mono font self-hosted via Fontsource for code editor and console
- [x] **FOUN-03**: CSS custom properties token system (tokens.css) replacing all hardcoded hex values across components
- [x] **FOUN-04**: CSS Modules migration replacing inline onMouseEnter/onMouseLeave hover patterns with CSS :hover pseudo-classes
- [x] **FOUN-05**: macOS font smoothing fix applied (-webkit-font-smoothing: antialiased, color-scheme: dark)

### Title Bar

- [x] **TBAR-01**: Widget buttons sized to 26px height matching Rider New UI proportions
- [x] **TBAR-02**: Correct spacing, separator positions, and font weights across all toolbar widgets
- [x] **TBAR-03**: Search Everywhere magnifying glass icon button in toolbar center-right area
- [x] **TBAR-04**: Settings gear icon at toolbar far-right position

### Tab Bar

- [x] **TABS-01**: Tab bar height increased to 38px with padding 0 16px matching Rider spacing
- [x] **TABS-02**: Close button hidden on inactive tabs, appearing on hover only (always visible on active tab)

### Editor

- [ ] **EDIT-01**: Breadcrumb navigation bar below tab bar showing cursor position in syntax tree
- [ ] **EDIT-02**: Fold gutter icons visible on hover only (hidden by default)
- [ ] **EDIT-03**: Breakpoint markers overlay line numbers in a single combined gutter (remove separate breakpoint column)

### Tool Strips & Panels

- [x] **PNLS-01**: Tool strip width expanded to 40px with 36px buttons and appropriately sized icons
- [x] **PNLS-02**: Panel header rows with title text + right-aligned action icons on ScriptList, DebugPanel, and Console panels
- [x] **PNLS-03**: Bottom panel tab strip with Rider-style chrome (2px active indicator, proper tab sizing)
- [x] **PNLS-04**: Resizable panels with drag handles using react-resizable-panels

### Status Bar

- [x] **STAT-01**: Navigation breadcrumb path segments in status bar left region (project > folder > file)
- [x] **STAT-02**: Diagnostics widget with icon + count pattern replacing plain text (error icon + red count, warning icon + yellow count)

### Polish

- [x] **PLSH-01**: Consistent 150ms ease hover transitions on all interactive elements
- [x] **PLSH-02**: Correct border/separator 3-level color hierarchy (#1E1F22 outer, #393B40 separators, #43454A subtle dividers)
- [ ] **PLSH-03**: Custom tooltip component with Rider dark styling replacing native browser title attributes
- [ ] **PLSH-04**: Keyboard shortcut hints displayed in tooltip text (e.g., "Run (Shift+F10)")

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Search

- **SRCH-01**: Working Search Everywhere modal with fuzzy script-name search
- **SRCH-02**: Search results with tabbed categories (scripts, symbols, actions)

### Density

- **DENS-01**: Compact mode toggle reducing heights, spacing, and icon sizes
- **DENS-02**: CSS custom property density system (--ui-density: normal|compact)

### Advanced Editor

- **ADVD-01**: VCS-style gutter change indicators (lines changed since last run)
- **ADVD-02**: Animated status bar execution progress spinner during run/debug
- **ADVD-03**: EditorState preservation across tab switches (undo history, scroll, folds)

## Out of Scope

| Feature | Reason |
|---------|--------|
| Light theme | Dark theme matches space game aesthetic; doubles CSS maintenance |
| Multi-row tabs | Pushes editor content down; Rider discourages in New UI |
| Floating/detachable tool windows | wry/Bevy can't spawn child windows with shared state |
| Plugin/extension system | Game editor, not general IDE; premature abstraction |
| Real-time collaborative editing | Massive CRDT infrastructure for edge case |
| Minimap | VoidScript files are small; clutters minimal Rider aesthetic |
| Full class/symbol indexing | VoidScript has no namespace system yet |
| Game mechanics (mothership, resources, combat) | Future milestones per GDD |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| FOUN-01 | Phase 1 | Complete |
| FOUN-02 | Phase 1 | Complete |
| FOUN-03 | Phase 1 | Complete |
| FOUN-04 | Phase 2 | Complete |
| FOUN-05 | Phase 1 | Complete |
| TBAR-01 | Phase 3 | Complete |
| TBAR-02 | Phase 3 | Complete |
| TBAR-03 | Phase 3 | Complete |
| TBAR-04 | Phase 3 | Complete |
| TABS-01 | Phase 4 | Complete |
| TABS-02 | Phase 4 | Complete |
| EDIT-01 | Phase 9 | Pending |
| EDIT-02 | Phase 8 | Pending |
| EDIT-03 | Phase 8 | Pending |
| PNLS-01 | Phase 5 | Complete |
| PNLS-02 | Phase 5 | Complete |
| PNLS-03 | Phase 5 | Complete |
| PNLS-04 | Phase 7 | Complete |
| STAT-01 | Phase 6 | Complete |
| STAT-02 | Phase 6 | Complete |
| PLSH-01 | Phase 2 | Complete |
| PLSH-02 | Phase 2 | Complete |
| PLSH-03 | Phase 9 | Pending |
| PLSH-04 | Phase 9 | Pending |

**Coverage:**
- v1 requirements: 24 total
- Mapped to phases: 24
- Unmapped: 0

---
*Requirements defined: 2026-03-14*
*Last updated: 2026-03-14 after roadmap creation — traceability confirmed*
