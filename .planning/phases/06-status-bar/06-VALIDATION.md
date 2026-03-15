---
phase: 6
slug: status-bar
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-15
---

# Phase 6 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | None — no test runner in project |
| **Config file** | None — Wave 0 gap |
| **Quick run command** | `cd editor-ui && npx tsc --noEmit` |
| **Full suite command** | `cd editor-ui && npx tsc --noEmit` |
| **Estimated runtime** | ~5 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cd editor-ui && npx tsc --noEmit`
- **After every plan wave:** Run `cd editor-ui && npx tsc --noEmit` + visual inspection
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 5 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 06-01-01 | 01 | 1 | STAT-01 | compile | `cd editor-ui && npx tsc --noEmit` | ✅ | ⬜ pending |
| 06-01-02 | 01 | 1 | STAT-01 | visual | Manual — verify breadcrumb segments | N/A | ⬜ pending |
| 06-01-03 | 01 | 1 | STAT-02 | compile | `cd editor-ui && npx tsc --noEmit` | ✅ | ⬜ pending |
| 06-01-04 | 01 | 1 | STAT-02 | visual | Manual — verify icon+count pairs | N/A | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. No test runner exists in this project and no previous phase introduced one. TypeScript compilation serves as the lightweight automated gate, consistent with all prior phases.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| NavPath shows "project > folder > file" | STAT-01 | Visual layout verification | Open editor, select a script tab, verify breadcrumb renders correct segments |
| NavPath shows project only when no tab | STAT-01 | Visual layout verification | Close all tabs, verify only project name shows |
| Error icon + count renders red circle | STAT-02 | SVG visual verification | Trigger errors in script, verify red circle icon with count |
| Warning icon + count renders yellow triangle | STAT-02 | SVG visual verification | Trigger warnings, verify yellow triangle icon with count |
| Status bar is 24px tall, 11px Inter text | STAT-01/02 | Visual/CSS verification | Inspect element, confirm height and font-size |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 5s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
