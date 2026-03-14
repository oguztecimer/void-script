---
phase: 3
slug: title-bar
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-14
---

# Phase 3 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | None — no test runner installed in project |
| **Config file** | None — see Wave 0 |
| **Quick run command** | `cd editor-ui && npm run build` |
| **Full suite command** | `cd editor-ui && npm run build` |
| **Estimated runtime** | ~5 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cd editor-ui && npm run build`
- **After every plan wave:** Run `cd editor-ui && npm run build` + visual inspection
- **Before `/gsd:verify-work`:** Build must be green, all 4 TBAR requirements visually verified
- **Max feedback latency:** 5 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 03-01-01 | 01 | 1 | TBAR-01 | build | `cd editor-ui && npm run build` | N/A | ⬜ pending |
| 03-01-02 | 01 | 1 | TBAR-02 | build | `cd editor-ui && npm run build` | N/A | ⬜ pending |
| 03-01-03 | 01 | 1 | TBAR-03 | build | `cd editor-ui && npm run build` | N/A | ⬜ pending |
| 03-01-04 | 01 | 1 | TBAR-04 | build | `cd editor-ui && npm run build` | N/A | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

*Existing infrastructure covers build verification. No test framework needed for visual-only requirements.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Toolbar buttons 26px tall | TBAR-01 | Pure CSS dimension — no DOM test runner | Open DevTools → select any toolbar button → verify computed height = 26px |
| Correct separator positions, font weights | TBAR-02 | Visual layout — requires human judgment against Rider reference | Compare toolbar groups and separator placement against Rider screenshot |
| Search Everywhere pill visible | TBAR-03 | Visual presence check | Verify magnifying glass icon + "Search" text + "⇧⇧" hint visible in toolbar center-right |
| Settings gear at far-right | TBAR-04 | Visual presence check | Verify gear icon is the rightmost element in toolbar |
| Hover works after window drag | SC-04 | macOS-specific WKWebView behavior | Drag window → hover toolbar button → verify state change appears (no stuck hover) |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 5s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
