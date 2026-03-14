---
phase: 5
slug: tool-strips-and-panels
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-14
---

# Phase 5 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | None (visual-only phase, TypeScript compilation as proxy) |
| **Config file** | none |
| **Quick run command** | `cd editor-ui && npx tsc --noEmit 2>&1 \| head -20` |
| **Full suite command** | `cd editor-ui && npm run build` |
| **Estimated runtime** | ~5 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cd editor-ui && npx tsc --noEmit 2>&1 | head -20`
- **After every plan wave:** Run `cd editor-ui && npm run build`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 5 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 05-01-01 | 01 | 1 | PNLS-01 | manual + tsc | `npx tsc --noEmit` | N/A | ⬜ pending |
| 05-01-02 | 01 | 1 | PNLS-02 | manual + tsc | `npx tsc --noEmit` | N/A | ⬜ pending |
| 05-02-01 | 02 | 2 | PNLS-02, PNLS-03 | manual + tsc | `npx tsc --noEmit` | N/A | ⬜ pending |
| 05-02-02 | 02 | 2 | PNLS-04 | manual + tsc | `npx tsc --noEmit` | N/A | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. No test framework needed — all requirements are visual/CSS with TypeScript compilation as the automated safety net.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Tool strip 40px width, 36px SVG buttons | PNLS-01 | CSS pixel dimension | DevTools: measure `.strip` width and `.btn` dimensions |
| Panel headers with title + action icons | PNLS-02 | Visual layout verification | Inspect ScriptList, DebugPanel, Console (BottomTabStrip) headers in browser |
| Bottom tab strip with 2px blue indicator | PNLS-03 | CSS visual styling | Inspect `.active` tab border-bottom in DevTools |
| Drag-resize handles on side panels | PNLS-04 | Interactive drag behavior | Drag panel divider, verify resize works, close/reopen editor |
| Collapse animation ~150ms ease | PNLS-04 | Visual animation timing | Close panel, confirm smooth ~150ms transition (not snap) |
| Reopen restores last width | PNLS-04 | Interactive state persistence | Drag panel to custom width, close, reopen — should restore |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 5s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
