---
status: diagnosed
trigger: "Tooltip component clips off the left and right edges of the viewport when hovering over elements near the sides"
created: 2026-03-15T00:00:00Z
updated: 2026-03-15T00:00:00Z
---

## Current Focus

hypothesis: Tooltip uses CSS `left: 50%; transform: translateX(-50%)` for horizontal centering with no JS clamping, causing overflow when the anchor element is near viewport edges
test: Read Tooltip.tsx and Tooltip.module.css to confirm positioning logic and JS effect
expecting: No horizontal clamping code exists in the useEffect or anywhere else
next_action: COMPLETE — root cause confirmed

## Symptoms

expected: Tooltip stays fully visible within viewport, even when anchor is near the left or right edge
actual: Tooltip extends beyond the visible area when anchor element is near the left or right edge of the viewport
errors: (none — visual clipping, no JS error)
reproduction: Hover over a toolbar button or tab at the far left or far right of the editor UI; tooltip overflows that edge
started: Always — horizontal clamping was never implemented

## Eliminated

- hypothesis: CSS overflow hidden on a parent container clips the tooltip
  evidence: `.tooltip` has `position: absolute` on `.wrapper` which is `position: relative; display: inline-flex` — no overflow:hidden in the chain, the tooltip renders in the stacking context of the wrapper, not clipped by a container
  timestamp: 2026-03-15

- hypothesis: The existing useEffect handles horizontal as well as vertical
  evidence: Lines 16-28 of Tooltip.tsx — the effect only checks `rect.bottom > window.innerHeight` (vertical bottom edge). No check for `rect.left < 0` or `rect.right > window.innerWidth`. Horizontal clamping is entirely absent.
  timestamp: 2026-03-15

## Evidence

- timestamp: 2026-03-15
  checked: Tooltip.module.css lines 6-22
  found: `.tooltip` is positioned with `position: absolute; left: 50%; transform: translateX(-50%)`
  implication: The tooltip is centred relative to its `.wrapper` parent in document flow. The `left: 50%` is relative to the wrapper width, and `translateX(-50%)` shifts the tooltip left by half its own width. This correctly centres it horizontally over the anchor — but only within the parent's coordinate space. No viewport boundary is consulted at the CSS level.

- timestamp: 2026-03-15
  checked: Tooltip.tsx useEffect (lines 16-28)
  found: The effect fires when `visible` changes. It reads `tooltipRef.current.getBoundingClientRect()` and checks `rect.bottom > window.innerHeight` to set `flipped: true`. There is NO equivalent check for `rect.left < 0` or `rect.right > window.innerWidth`. No `translateX` override or `left`/`right` inline style is ever set.
  implication: Horizontal position is 100% governed by the static CSS rule. There is no runtime correction for left or right overflow.

- timestamp: 2026-03-15
  checked: Tooltip.tsx render output (lines 55-62)
  found: The tooltip div receives only a className (`styles.tooltip` + optional `styles.flipped`). No `style` prop is set, so there is no way to pass a runtime horizontal offset.
  implication: Even if a horizontal offset were computed in the useEffect, there is currently no mechanism to apply it to the element.

- timestamp: 2026-03-15
  checked: Tooltip.tsx state (lines 11-12)
  found: Only two state variables: `visible: boolean` and `flipped: boolean`. No `offsetX` or similar state for horizontal correction.
  implication: Horizontal clamping would require a third state variable (e.g. `offsetX: number`) or a `ref`-driven inline style.

## Resolution

root_cause: >
  The tooltip's horizontal position is set entirely in CSS as `left: 50%; transform: translateX(-50%)`.
  The `useEffect` in Tooltip.tsx only checks the bottom-edge overflow and sets a `flipped` boolean —
  it never inspects `rect.left` or `rect.right` against the viewport width.
  There is no state variable, no inline style prop, and no CSS rule that corrects horizontal overflow.
  When the anchor element is within ~(tooltipWidth/2) pixels of the left or right viewport edge,
  the centred tooltip overflows that edge with no correction.

fix: (not applied — diagnose-only mode)
verification: (not applied)
files_changed: []
