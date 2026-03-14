# VOID//SCRIPT Editor

## What This Is

VOID//SCRIPT is a space automation game where players write code in a custom scripting language to control ships, factories, and fleets. The in-game code editor is the primary interface — players spend most of their time here. This milestone focuses on making the editor UI a pixel-accurate recreation of JetBrains Rider's New UI, so it feels like a real professional IDE embedded in a game.

## Core Value

The code editor must look and feel like JetBrains Rider's New UI — professional, polished, and immediately familiar to developers. If the editor feels like a toy, the game fails.

## Requirements

### Validated

- ✓ Frameless window with custom title bar — existing
- ✓ CodeMirror-based editor with VoidScript syntax highlighting — existing
- ✓ Tab bar with close/modified indicators — existing
- ✓ Left tool strip + scripts panel — existing
- ✓ Right tool strip + debug panel — existing
- ✓ Bottom panel with console output — existing
- ✓ Status bar with cursor position, diagnostics, encoding — existing
- ✓ Run/Debug/Stop controls in title bar — existing
- ✓ Breakpoint gutter with toggle — existing
- ✓ IPC bridge between webview (JS) and Rust/Bevy backend — existing
- ✓ VoidScript interpreter with basic execution — existing

### Active

- [ ] Pixel-accurate Rider New UI title bar (widget spacing, heights, icon sizing)
- [ ] Correct Inter font usage across all OS
- [ ] Refined color palette matching Rider dark theme exactly
- [ ] Improved contrast ratios throughout (Rider's updated dark theme)
- [ ] Breadcrumb navigation below tabs
- [ ] Search everywhere widget in title bar
- [ ] Settings gear icon in toolbar
- [ ] Tool window headers matching Rider (larger icons, proper spacing)
- [ ] Tab bar with Rider's larger font, increased spacing
- [ ] Status bar with navigation bar integration (Rider-style)
- [ ] Panel header refinements (bottom panel tabs matching Rider)
- [ ] Gutter refinements (breakpoints overlay line numbers, fold icons on hover)
- [ ] Tooltip and autocomplete styling matching Rider
- [ ] Proper hover states and transitions throughout
- [ ] Consistent border and separator styling

### Out of Scope

- Game mechanics (mothership, resources, combat, R&D) — future milestones per GDD
- PvP system — future milestone
- Campaign/story mode — future milestone
- Audio design — future milestone
- Light theme — dark theme only for now
- OAuth/authentication — single-player editor for now
- Plugin system — not needed yet

## Context

The editor is built as a Tauri-like architecture: Rust/Bevy manages the game window, wry embeds a webview for the editor UI. The frontend is React + TypeScript + Vite with CodeMirror 6 for the text editor. The current UI already uses Rider's color tokens (`#1E1F22` background, `#2B2D30` panels, `#393B40` borders) but needs refinement in spacing, typography, widget layout, and missing UI elements to truly match Rider's New UI.

The GDD (VOIDSCRIPT_GDD_v1.2.md) describes the full game vision including scripting system, mothership, resources, R&D tree, campaign, PvP, and combat. This milestone focuses exclusively on the editor shell.

## Constraints

- **Tech stack**: React + TypeScript + Vite frontend, Rust/Bevy + wry backend — established, not changing
- **Font**: Inter for UI, JetBrains Mono for code — matching Rider
- **Theme**: Dark theme only (Rider dark) — `#1E1F22` base
- **Window**: Frameless (`decorations: false` in Bevy) — custom title bar with macOS traffic lights
- **Target reference**: JetBrains Rider New UI (https://www.jetbrains.com/help/rider/New_UI.html)

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Editor-first milestone | Players spend 90% of time in the editor; it must feel professional before adding game features | — Pending |
| Rider New UI as reference | Most recognizable modern IDE UI; familiar to target audience (programmers) | — Pending |
| Dark theme only | Matches space game aesthetic; Rider dark is the most used theme | — Pending |
| Frameless window | Enables custom title bar matching Rider exactly | ✓ Good |

---
*Last updated: 2026-03-14 after initialization*
