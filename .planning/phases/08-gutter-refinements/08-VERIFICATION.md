---
phase: 08-gutter-refinements
verified: 2026-03-15T00:00:00Z
status: passed
score: 6/6 must-haves verified
re_verification: false
---

# Phase 8: Gutter Refinements Verification Report

**Phase Goal:** Refine the editor gutter to match JetBrains Rider style — merge breakpoints into the line-number column, configure fold icons to appear on hover with Rider-style triangles, and add a breakpoint hover preview.
**Verified:** 2026-03-15
**Status:** passed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Fold/unfold icons are invisible by default; they appear on hover | VERIFIED | `.cm-foldGutter .cm-fold-marker` has `opacity: '0'`; `.cm-foldGutter .cm-gutterElement:hover .cm-fold-marker` sets `opacity: '1'` — voidscript-theme.ts lines 81–91 |
| 2 | Breakpoint markers share the line-number gutter column — no separate breakpoint column exists | VERIFIED | `createBreakpointGutter` fully removed (grep returns 0); `breakpointLineNumberMarkers` via `lineNumberMarkers.computeN` feeds markers into the line-number gutter — Editor.tsx lines 65–68, 118 |
| 3 | Clicking anywhere in the line-number gutter toggles a breakpoint on that line | VERIFIED | `lineNumbers({ domEventHandlers: { mousedown: handleBreakpointClick } })` — Editor.tsx line 120; handler dispatches `toggleBreakpointEffect`, calls `store.toggleBreakpoint`, sends IPC `toggle_breakpoint` |
| 4 | A red circle replaces the line number when a breakpoint is set | VERIFIED | `BreakpointOverlayMarker.toDOM()` creates `div.cm-bp-circle` (Editor.tsx lines 31–35); `.cm-bp-circle` styled with `borderRadius: '50%'`, `backgroundColor: 'var(--accent-breakpoint)'` (theme lines 102–108); `toDOM` presence causes CodeMirror to suppress the line number for those rows |
| 5 | A faint red circle preview appears on hover over any line-number gutter cell | VERIFIED | `.cm-lineNumbers .cm-gutterElement::after` uses `opacity: '0'` with `backgroundColor: 'var(--accent-breakpoint)'`; `.cm-lineNumbers .cm-gutterElement:hover::after` raises to `opacity: '0.25'` — theme lines 111–127 |
| 6 | All .cm-* style overrides live inside EditorView.theme() — no .cm-* rules in external CSS files | VERIFIED | grep for `.cm-` in all `*.css` and `*.module.css` under `editor-ui/src` returns no matches |

**Score:** 6/6 truths verified

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `editor-ui/src/components/Editor.tsx` | Breakpoint overlay via lineNumberMarkers facet, foldGutter with custom markerDOM | VERIFIED | Contains `BreakpointOverlayMarker`, `lineNumberMarkers.computeN`, `foldGutter({ markerDOM })`, `lineNumbers({ domEventHandlers })`. No `createBreakpointGutter` present. |
| `editor-ui/src/codemirror/voidscript-theme.ts` | All gutter CSS rules for fold hover, breakpoint circle, and hover preview | VERIFIED | Contains fold gutter rules (opacity 0/1 on hover), `.cm-bp-circle` breakpoint circle, and `::after` hover preview — all inside `EditorView.theme()` |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `Editor.tsx` | `voidscript-theme.ts` | `BreakpointOverlayMarker.toDOM()` creates `div.cm-bp-circle` styled by `EditorView.theme()` | WIRED | `el.className = 'cm-bp-circle'` in Editor.tsx line 33; `.cm-bp-circle` rule present in theme line 102 |
| `Editor.tsx` | `voidscript-theme.ts` | `foldGutter markerDOM` creates `span.cm-fold-marker` styled by `EditorView.theme()` | WIRED | `span.className = 'cm-fold-marker'` in Editor.tsx line 130; `.cm-foldGutter .cm-fold-marker` rule present in theme line 81 |
| `Editor.tsx` | `@codemirror/view lineNumberMarkers facet` | `breakpointState` RangeSet feeds `lineNumberMarkers.computeN()` to suppress line numbers on breakpoint rows | WIRED | `lineNumberMarkers.computeN([breakpointState], state => [state.field(breakpointState)])` — Editor.tsx lines 65–68; included in extensions array at line 118 |

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| EDIT-02 | 08-01-PLAN.md | Fold gutter icons visible on hover only (hidden by default) | SATISFIED | `.cm-fold-marker` opacity 0 default, opacity 1 on `.cm-gutterElement:hover` in EditorView.theme(); `foldGutter({ markerDOM })` with `cm-fold-marker` class in Editor.tsx |
| EDIT-03 | 08-01-PLAN.md | Breakpoint markers overlay line numbers in a single combined gutter (remove separate breakpoint column) | SATISFIED | `lineNumberMarkers.computeN` feeds `BreakpointOverlayMarker` (with `toDOM`) into line-number gutter; `createBreakpointGutter` fully removed; `.cm-bp-circle` styled in EditorView.theme() |

No orphaned requirements — REQUIREMENTS.md maps only EDIT-02 and EDIT-03 to Phase 8, both are accounted for. REQUIREMENTS.md traceability table confirms both marked complete.

---

### Anti-Patterns Found

None detected.

- `TODO`/`FIXME`/placeholder comments: none in modified files
- Empty implementations: `BreakpointOverlayMarker.toDOM()` is substantive (creates real DOM element)
- `handleBreakpointClick` is substantive: dispatches state effect, updates store, sends IPC message
- TypeScript compiles with zero errors (confirmed: empty `tsc --noEmit` output)
- Commit hashes 293c5d6 and 23490cb both exist in git history

---

### Human Verification Required

The following behaviors require visual confirmation in the running app (cannot be verified by static analysis):

**1. Fold icon hover reveal**

Test: Open a script with foldable syntax (e.g. a function body). Hover over the fold gutter column on a foldable line.
Expected: Triangle icon (▼ or ▶) appears only while the cursor is over that row; invisible on all other rows.
Why human: CSS `:hover` pseudo-class behavior on individual gutter rows cannot be verified statically.

**2. Breakpoint circle visual**

Test: Click a line number in the gutter. The line number should disappear and a red circle should appear in its place.
Expected: Red filled circle (~12px) centered in the gutter cell, line number text gone.
Why human: CodeMirror's `lineMarker` suppression of the `NumberMarker` (when `toDOM` is present on a `lineNumberMarkers` entry) is runtime behavior.

**3. Breakpoint hover preview**

Test: Hover over a line-number gutter cell that does NOT have a breakpoint set.
Expected: A faint red circle (25% opacity) appears briefly as affordance while hovering.
Why human: CSS `::after` pseudo-element opacity transition requires visual inspection.

**4. No visible separate breakpoint column**

Test: Inspect the gutter visually in a running instance.
Expected: Only two gutter columns visible: fold gutter (14px) and line numbers. No third column from the old `gutter()` call.
Why human: DOM layout cannot be confirmed without rendering.

---

### Gaps Summary

No gaps. All 6 observable truths are verified, all 2 artifacts are substantive and wired, all 3 key links are connected, and both EDIT-02 and EDIT-03 requirements are satisfied by the implementation.

The one deviation from the plan (using `lineNumberMarkers.computeN` instead of `.of` with a function) was a necessary type-safe correction — the semantics are equivalent and the implementation matches the plan intent.

Human verification items are recommended but do not block the goal: the static code evidence fully supports all required behaviors.

---

_Verified: 2026-03-15_
_Verifier: Claude (gsd-verifier)_
