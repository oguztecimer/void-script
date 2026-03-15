---
phase: 7
slug: resizable-panels
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-15
---

# Phase 7 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | None — no test framework installed in project |
| **Config file** | none |
| **Quick run command** | Manual visual check in running app (`cargo tauri dev`) |
| **Full suite command** | Manual checklist walkthrough (see Per-Task Verification Map) |
| **Estimated runtime** | ~120 seconds (manual) |

---

## Sampling Rate

- **After every task commit:** Manual visual check in running app
- **After every plan wave:** Full manual checklist from verification map below
- **Before `/gsd:verify-work`:** All manual verifications confirmed
- **Max feedback latency:** ~120 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 07-01-01 | 01 | 1 | PNLS-04 | manual | Visual: vertical Group renders, bottom panel at 25% default | N/A | ⬜ pending |
| 07-01-02 | 01 | 1 | PNLS-04 | manual | Visual: drag bottom separator resizes smoothly | N/A | ⬜ pending |
| 07-01-03 | 01 | 1 | PNLS-04 | manual | Visual: collapse via drag snap syncs Zustand | N/A | ⬜ pending |
| 07-01-04 | 01 | 1 | PNLS-04 | manual | Visual: double-click separator toggles collapse | N/A | ⬜ pending |
| 07-01-05 | 01 | 1 | PNLS-04 | manual | DevTools: `localStorage.getItem('void-center-layout')` persists | N/A | ⬜ pending |
| 07-01-06 | 01 | 1 | PNLS-04 | manual | Visual: panel sizes restored after reload | N/A | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] No test framework installed — all PNLS-04 verification is human/visual

*No automated tests are feasible for this phase without adding a test framework, which is out of scope.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Vertical Group renders with bottom panel at 25% default | PNLS-04 | wry WebView visual rendering; no browser test harness | 1. Launch app 2. Verify bottom panel visible at ~25% of center height |
| Bottom panel resizes smoothly without layout shift | PNLS-04 | Drag interaction in wry WebView | 1. Drag bottom separator up/down 2. Verify no flicker or layout jump |
| Collapse via drag snap syncs Zustand | PNLS-04 | Pointer events + state sync | 1. Drag below minSize 2. Verify panel snaps closed 3. Verify toggle button reflects collapsed state |
| Double-click separator toggles collapse/expand | PNLS-04 | DOM event handling in wry | 1. Double-click bottom separator 2. Verify panel collapses 3. Double-click again 4. Verify panel expands |
| Panel sizes persist after reload | PNLS-04 | localStorage + page lifecycle | 1. Resize bottom panel 2. Reload page 3. Verify size restored |
| Collapsing side panel doesn't flicker editor | PNLS-04 | Visual rendering | 1. Collapse left/right panel 2. Verify editor area stays stable |

---

## Validation Sign-Off

- [ ] All tasks have manual verify instructions
- [ ] Sampling continuity: manual check after every task commit
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
