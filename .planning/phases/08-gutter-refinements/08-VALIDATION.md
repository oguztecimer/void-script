---
phase: 8
slug: gutter-refinements
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-15
---

# Phase 8 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | None — manual visual verification (consistent with prior phases) |
| **Config file** | None |
| **Quick run command** | `cd editor-ui && npm run build` |
| **Full suite command** | `cd editor-ui && npm run build && grep -r "\.cm-" src --include="*.css" --include="*.module.css"` |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cd editor-ui && npm run build`
- **After every plan wave:** Run full suite command + manual visual check in running app
- **Before `/gsd:verify-work`:** Full suite must be green + visual verification of all 3 success criteria
- **Max feedback latency:** 10 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 08-01-01 | 01 | 1 | EDIT-03 | compile | `cd editor-ui && npm run build` | N/A | ⬜ pending |
| 08-01-02 | 01 | 1 | EDIT-03 | automated | `grep -r "\.cm-" editor-ui/src --include="*.css" --include="*.module.css"` | N/A | ⬜ pending |
| 08-01-03 | 01 | 1 | EDIT-02 | manual | Visual: fold icons hidden, appear on hover | N/A | ⬜ pending |
| 08-01-04 | 01 | 1 | EDIT-03 | manual | Visual: breakpoint circle replaces line number | N/A | ⬜ pending |
| 08-01-05 | 01 | 1 | EDIT-03 | manual | Visual: hover preview faint circle on gutter | N/A | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. No test framework needed — this phase is entirely visual/CSS changes verified by TypeScript compilation and manual inspection.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Fold icons hidden by default, visible on hover | EDIT-02 | Requires live browser DOM with CSS `:hover` | 1. Open editor with foldable code 2. Verify no fold icons visible 3. Hover gutter row — triangles appear 4. Move away — triangles fade |
| Breakpoint circle overlays line number | EDIT-03 | Requires live CodeMirror view rendering | 1. Click gutter to set breakpoint 2. Verify red circle replaces line number 3. Verify no separate breakpoint column |
| Hover preview circle | EDIT-03 | CSS pseudo-element rendering | 1. Hover over line-number gutter 2. Verify faint red circle preview appears 3. Click — full breakpoint set |
| No `.cm-*` rules in external CSS | EDIT-03 | Automated grep verification | Run: `grep -r "\.cm-" editor-ui/src --include="*.css" --include="*.module.css"` — expect empty output |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 10s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
