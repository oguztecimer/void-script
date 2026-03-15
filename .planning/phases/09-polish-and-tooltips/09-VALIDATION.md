---
phase: 9
slug: polish-and-tooltips
status: draft
nyquist_compliant: true
wave_0_complete: true
created: 2026-03-15
---

# Phase 9 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | None — no test infrastructure in project |
| **Config file** | None |
| **Quick run command** | `cd editor-ui && npm run build` (compile check) |
| **Full suite command** | `cd editor-ui && npm run build` |
| **Estimated runtime** | ~5 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cd editor-ui && npm run build`
- **After every plan wave:** Run `cd editor-ui && npm run build`
- **Before `/gsd:verify-work`:** Full build must pass
- **Max feedback latency:** 5 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 09-01-01 | 01 | 1 | PLSH-03 | manual | `npm run build` (compile) | N/A | ⬜ pending |
| 09-01-02 | 01 | 1 | PLSH-04 | manual | `npm run build` (compile) | N/A | ⬜ pending |
| 09-02-01 | 02 | 1 | EDIT-01 | manual | `npm run build` (compile) | N/A | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. No test framework needed — this is a visual polish phase validated through the running wry application.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Custom tooltip shows on hover, no native title | PLSH-03 | Visual UI behavior in wry WebView | Hover any toolbar button for 800ms — custom styled tooltip appears, no native OS tooltip |
| Shortcut hint in tooltip text | PLSH-04 | Visual text content check | Hover Run button — tooltip reads "Run (Shift+F10)" |
| Single tooltip at a time | PLSH-03 | Hover timing behavior | Move mouse quickly between buttons — only one tooltip visible |
| Breadcrumb shows function on cursor move | EDIT-01 | Cursor position + DOM update | Open script with `def` blocks, move cursor inside one — breadcrumb shows filename › function_name |
| Breadcrumb at top level | EDIT-01 | Cursor position check | Move cursor to top level — breadcrumb shows just filename |
| Tooltip viewport flip | PLSH-03 | Visual positioning | Resize window so tooltip would clip bottom — tooltip flips above trigger |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 5s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
