---
status: diagnosed
trigger: "Investigate why the status bar is not visible/rendering in the void-script editor UI"
created: 2026-03-15T00:00:00Z
updated: 2026-03-15T00:00:00Z
---

## Current Focus

hypothesis: The status bar IS rendered in the DOM, but the flex layout causes .main to consume all remaining space, leaving zero height for StatusBar. The .main div has `flex: 1` but no `min-height: 0`, and more critically, the StatusBar has no `flex-shrink: 0` to protect its height from being squeezed out.
test: Analyzed CSS layout chain from .app down to StatusBar
expecting: StatusBar height is collapsed to 0 by flex layout
next_action: Report diagnosis

## Symptoms

expected: Status bar visible at bottom of the window with NavPath breadcrumb, diagnostics widget, cursor position, and file info
actual: Status bar is completely invisible - not rendered or height collapsed to zero
errors: None (TypeScript compiles clean)
reproduction: Load the editor UI - status bar is missing from the bottom
started: Unknown - may have been introduced when react-resizable-panels layout was added

## Eliminated

- hypothesis: StatusBar not imported/rendered in App.tsx
  evidence: Line 16 imports StatusBar, line 183 renders it inside the .app div after .main
  timestamp: 2026-03-15

- hypothesis: TypeScript compilation errors preventing render
  evidence: `npx tsc --noEmit` passes with zero errors
  timestamp: 2026-03-15

- hypothesis: CSS display:none or visibility:hidden hiding the bar
  evidence: StatusBar.module.css .bar has `display: flex`, no hiding rules anywhere
  timestamp: 2026-03-15

- hypothesis: Missing CSS module file
  evidence: StatusBar.module.css exists at editor-ui/src/components/StatusBar.module.css
  timestamp: 2026-03-15

- hypothesis: Missing CSS custom properties (wry issue)
  evidence: --height-statusbar, --bg-panel, --font-size-status, --text-secondary, --border-strong all defined in index.html inline styles
  timestamp: 2026-03-15

## Evidence

- timestamp: 2026-03-15
  checked: App.tsx layout structure
  found: The .app div is a flex column (height: 100vh) with three children: Header, .main div, StatusBar. The .main div has `flex: 1` which makes it grow to fill all available space. StatusBar has `height: var(--height-statusbar)` (24px) but NO `flex-shrink: 0` protection.
  implication: In a flex column layout, `flex: 1` on .main means it will try to take all remaining space. If the content inside .main (the react-resizable-panels Group) takes more than the available space, .main can push StatusBar off-screen or squeeze it to 0.

- timestamp: 2026-03-15
  checked: App.module.css .main styles
  found: `.main { display: flex; flex: 1; overflow: hidden; }` - the overflow:hidden means any overflow is clipped. The .main div itself has no min-height:0 constraint, and its children (ToolStrips + Group) could cause it to exceed available space.
  implication: The `overflow: hidden` on .main clips any overflow but does not prevent .main from pushing StatusBar out of view.

- timestamp: 2026-03-15
  checked: StatusBar.module.css
  found: `.bar { height: var(--height-statusbar); }` uses var() for height (24px) but has no `flex-shrink: 0` to prevent flex compression.
  implication: Without flex-shrink: 0, the StatusBar can be compressed to 0 height by flex layout when .main takes up all available space.

- timestamp: 2026-03-15
  checked: Header.module.css
  found: `.toolbar { height: var(--height-titlebar); }` - Header also has no flex-shrink: 0, but since it is the FIRST child and has fixed content, it likely renders fine. StatusBar as the LAST child gets squeezed.
  implication: Both Header and StatusBar should have flex-shrink: 0 for robustness, but StatusBar is the one visibly affected.

- timestamp: 2026-03-15
  checked: react-resizable-panels Group container
  found: `.panelGroup { flex: 1; overflow: hidden; }` - the Group also has flex: 1 inside .main, competing for space.
  implication: The nested flex: 1 chain (.app -> .main -> .panelGroup) with no flex-shrink protection on fixed-height elements means the StatusBar gets zero remaining space.

## Resolution

root_cause: |
  The StatusBar is rendered in App.tsx but gets zero height due to CSS flex layout compression.

  The layout chain is:
  1. `.app` - flex column, height: 100vh (contains Header + .main + StatusBar)
  2. `.main` - flex: 1, overflow: hidden (contains ToolStrips + Group)
  3. `.panelGroup` (Group) - flex: 1, overflow: hidden

  The `.main` div has `flex: 1` which tells it to consume ALL remaining space after Header.
  Since StatusBar has `height: var(--height-statusbar)` (24px) but NO `flex-shrink: 0`,
  the flex algorithm compresses StatusBar to 0 height as .main takes everything.

  The Header survives because it renders first in the flex flow and has enough intrinsic
  content to claim its 40px, but StatusBar as the last child gets the leftover: nothing.

fix: (not applied - diagnosis only)
verification: (not applied - diagnosis only)
files_changed: []
