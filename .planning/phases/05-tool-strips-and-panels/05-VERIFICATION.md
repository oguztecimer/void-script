---
phase: 05-tool-strips-and-panels
verified: 2026-03-14T00:00:00Z
status: human_needed
score: 3/4 success criteria verified automatically
re_verification: false
human_verification:
  - test: "Drag a panel resize handle and verify panel resizes smoothly; close and reopen the editor and confirm panel width was restored"
    expected: "Dragging the 1px Separator resizes left/right panels. After reloading the WebView, the same widths are restored — not the 18% default."
    why_human: "useDefaultLayout hook wires localStorage persistence, but the actual round-trip (write on layout change, read on mount, apply to panel) requires a live browser session to confirm. Cannot be verified from static code inspection alone."
  - test: "Verify collapse animation is ~150ms ease — NOT an instant snap — when clicking a tool strip button to close a side panel"
    expected: "Panel width smoothly transitions to 0 over approximately 150ms. The .panelAnimated class (transition: flex-basis 150ms ease) is applied when isResizing is false."
    why_human: "CSS transition timing and the isResizing toggle via onLayoutChange/onLayoutChanged callbacks require interactive observation to confirm the animation fires on programmatic collapse but not during drag."
  - test: "Reopen a side panel that was previously resized to a custom width — confirm it restores the custom width, not 18%"
    expected: "panel.expand() restores the panel to its last known size before collapse, not defaultSize."
    why_human: "react-resizable-panels v4 expand() behavior for restoring pre-collapse size cannot be verified from static analysis."
---

# Phase 5: Tool Strips and Panels Verification Report

**Phase Goal:** Tool strips are the correct Rider width; all three side panels have Rider-style header chrome; the bottom panel has a proper tab strip; panels are resizable
**Verified:** 2026-03-14
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (from ROADMAP.md Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Left and right tool strips are 40px wide with 36px icon buttons | VERIFIED | `tokens.css:183 --width-toolstrip: 40px`, `tokens.css:186 --size-toolstrip-btn: 36px`; ToolStrip.module.css uses `width: var(--width-toolstrip)`; ToolBtn.module.css uses `width/height: var(--size-toolstrip-btn)` |
| 2 | ScriptList, DebugPanel, and Console panels each have a header row with a title and right-aligned action icons | VERIFIED | ScriptList.tsx: PanelHeader title="Scripts" with plus + close ToolBtn actions. DebugPanel.tsx: PanelHeader title="Debug" with close ToolBtn action. BottomTabStrip.tsx: renders Console tab label + clearConsole + toggleBottomPanel ToolBtn actions in `.actions` div |
| 3 | The bottom panel has a tab strip with a "Console" tab showing a 2px blue active indicator | VERIFIED | BottomTabStrip.module.css `.active { border-bottom-color: var(--accent-blue) }` on `.tab { border-bottom: 2px solid transparent }`. store.ts default `bottomPanelTab: 'console'`. BOTTOM_TABS = `[{ id: 'console', label: 'Console' }]` |
| 4 | Users can drag panel resize handles to change side panel widths; sizes persist after closing and reopening | PARTIAL — needs human | Code: Group + Separator (col-resize cursor, 1px width), collapsible Panel with panelRef/imperative API, useDefaultLayout({ id: 'void-main-layout' }) for persistence. Actual drag-resize behavior and localStorage round-trip require live session |

**Score:** 3/4 truths verified automatically

---

### Required Artifacts

#### Plan 01 Artifacts

| Artifact | Provides | Level 1: Exists | Level 2: Substantive | Level 3: Wired | Status |
|----------|----------|:---:|:---:|:---:|--------|
| `editor-ui/src/components/ToolStrip.tsx` | SVG icon rendering via ReactNode children in ToolBtn | YES | YES — icon field `React.ReactNode`, renders `{item.icon}` with `className` per active state | YES — imported and used in App.tsx | VERIFIED |
| `editor-ui/src/components/ToolStrip.module.css` | Active edge indicator and tertiary color override | YES | YES — `.stripBtn` tertiary color, `.left .activeBtn / .right .activeBtn` with inset box-shadow, no solid blue fill | YES — applied to ToolStrip.tsx via CSS Modules | VERIFIED |
| `editor-ui/src/components/ScriptList.tsx` | PanelHeader with close + add-script action buttons | YES | YES — PanelHeader actions: plus ToolBtn (sendToRust create_script) + hide ToolBtn | YES — rendered in App.tsx left Panel | VERIFIED |
| `editor-ui/src/components/DebugPanel.tsx` | Top-level Debug PanelHeader with close action | YES | YES — PanelHeader title="Debug" with close ToolBtn above Frames/Variables sub-headers | YES — rendered in App.tsx right Panel | VERIFIED |
| `editor-ui/src/App.tsx` | Updated LEFT_ITEMS/RIGHT_ITEMS with ReactNode SVG icons | YES | YES — `icon: (<svg .../>)` for both items, `currentColor` strokes | YES — passed to ToolStrip as `items` prop | VERIFIED |

#### Plan 02 Artifacts

| Artifact | Provides | Level 1: Exists | Level 2: Substantive | Level 3: Wired | Status |
|----------|----------|:---:|:---:|:---:|--------|
| `editor-ui/src/components/BottomTabStrip.tsx` | Standalone bottom tab strip wired to Zustand | YES | YES — useStore selectors for all 4 store values, BOTTOM_TABS constant, conditional `.active` class | YES — imported and rendered in App.tsx bottom panel | VERIFIED |
| `editor-ui/src/components/BottomTabStrip.module.css` | Tab strip styles with 2px active indicator | YES | YES — `.active { border-bottom-color: var(--accent-blue) }` on 2px-border tab | YES — CSS Modules applied in BottomTabStrip.tsx | VERIFIED |
| `editor-ui/src/App.tsx` | PanelGroup/Panel/PanelResizeHandle layout with imperative collapse/expand | YES | YES — Group + Panel (panelRef, collapsible, collapsedSize=0) + Separator; two useEffect watchers calling panel.collapse()/expand() | YES — root component, fully rendered | VERIFIED |
| `editor-ui/src/App.module.css` | Resize handle styling and conditional panel animation | YES | YES — `.panelAnimated { transition: flex-basis 150ms ease }`, `.resizeHandle { width: 1px; cursor: col-resize }` | YES — applied conditionally via `!isResizing ? styles.panelAnimated : ''` | VERIFIED |
| `editor-ui/src/state/store.ts` | Updated bottomPanelTab default to 'console' | YES | YES — `bottomPanelTab: 'console'` at line 71 | YES — consumed by BottomTabStrip and App | VERIFIED |
| `editor-ui/package.json` | react-resizable-panels dependency | YES | YES — `"react-resizable-panels": "^4.7.3"` in dependencies | YES — imported in App.tsx | VERIFIED |

---

### Key Link Verification

#### Plan 01 Key Links

| From | To | Via | Status | Evidence |
|------|----|-----|--------|----------|
| App.tsx | ToolStrip.tsx | LEFT_ITEMS/RIGHT_ITEMS icon property as ReactNode | WIRED | `icon: (<svg .../>)` at App.tsx:26,39; consumed by `{item.icon}` in ToolStrip.tsx:31 |
| ToolStrip.module.css | ToolBtn.module.css | Override .active style with box-shadow `.left .activeBtn` / `.right .activeBtn` | WIRED | `.activeBtn` class injected via `className` prop (not `active` prop) — bypasses ToolBtn's solid blue fill; compound selectors place inset shadow on correct inner edge |

#### Plan 02 Key Links

| From | To | Via | Status | Evidence |
|------|----|-----|--------|----------|
| BottomTabStrip.tsx | store.ts | useStore selectors for bottomPanelTab and setBottomPanelTab | WIRED | Line 10: `useStore((s) => s.bottomPanelTab)`, line 11: `useStore((s) => s.setBottomPanelTab)` |
| App.tsx | react-resizable-panels | Group/Panel/Separator/PanelImperativeHandle/useDefaultLayout imports | WIRED | Lines 3–8: all v4 API symbols imported from 'react-resizable-panels' |
| App.tsx | BottomTabStrip.tsx | BottomTabStrip replaces inline bottom panel header | WIRED | Line 17: import; line 144: `<BottomTabStrip />` inside bottomPanel div |
| App.tsx | store.ts | useEffect watches leftPanelOpen/rightPanelOpen to call panel.collapse()/expand() | WIRED | Lines 70–88: two useEffect hooks watching Zustand state, calling `leftPanelRef.current.collapse()/expand()` and `rightPanelRef.current.collapse()/expand()` |

---

### Requirements Coverage

| Requirement | Plan(s) | Description | Status | Evidence |
|-------------|---------|-------------|--------|----------|
| PNLS-01 | 05-01 | Tool strip width 40px, 36px buttons, appropriately sized icons | SATISFIED | `--width-toolstrip: 40px` (tokens.css:183), `--size-toolstrip-btn: 36px` (tokens.css:186), ReactNode SVG icons replacing emoji text |
| PNLS-02 | 05-01, 05-02 | Panel header rows with title + right-aligned action icons on ScriptList, DebugPanel, Console | SATISFIED | ScriptList PanelHeader (plus+hide), DebugPanel PanelHeader (hide), BottomTabStrip (Console label + clear+close actions) |
| PNLS-03 | 05-02 | Bottom panel tab strip with Rider-style chrome (2px active indicator, proper tab sizing) | SATISFIED | BottomTabStrip.module.css `.active { border-bottom-color: var(--accent-blue) }` on `border-bottom: 2px solid transparent` tab |
| PNLS-04 | 05-02 | Resizable panels with drag handles using react-resizable-panels | PARTIAL — needs human | Library installed (v4.7.3), DOM structure in place (Group/Panel/Separator), imperative API wired. Drag behavior and size persistence require human testing. See note below. |

**PNLS-04 traceability note:** REQUIREMENTS.md traceability table (line 109) maps PNLS-04 to "Phase 7", but ROADMAP.md (line 88) lists PNLS-04 under both Phase 5 and Phase 7 requirements. ROADMAP.md line 120 explains the split: "Phase 5 prepares the DOM structure; Phase 7 makes resize handles live and persistent." This is an intentional split delivery — Phase 5 satisfies the DOM-prep portion of PNLS-04. The REQUIREMENTS.md traceability table should be updated to reference both phases, but this is a documentation inconsistency rather than a code gap.

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| App.module.css | 43 | `/* Phase 7: add visible drag indicator, hover highlight */` | Info | Forward-looking comment, not a blocker; resize handle is functional (1px border-strong, col-resize cursor) without the enhancement |

No TODO/FIXME, placeholder components, empty handlers, or stub return values found in any modified files.

---

### Human Verification Required

#### 1. Panel drag-resize and size persistence

**Test:** Open the editor. Drag the left panel divider to approximately 30% width. Close the left panel by clicking the Scripts tool strip button. Reopen it by clicking again. Then reload the WebView.
**Expected:** (a) Dragging resizes the panel smoothly. (b) Closing/reopening restores the dragged width via `panel.collapse()`/`panel.expand()`. (c) After page reload, `useDefaultLayout` restores the last layout from localStorage — the panel opens at 30%, not at the default 18%.
**Why human:** The `useDefaultLayout` hook wires localStorage write on `onLayoutChanged` and reads on mount. Verifying the round-trip requires a live browser session; static analysis only confirms the hook is wired.

#### 2. Collapse animation timing

**Test:** With a side panel open, click the tool strip button to close it.
**Expected:** The panel width transitions to 0 over approximately 150ms (smooth ease), not a jarring instant snap. During a drag operation, no CSS transition should interfere with pointer tracking.
**Why human:** The `isResizing` state is toggled by `onLayoutChange`/`onLayoutChanged` Group callbacks. Confirming the animation fires on programmatic collapse (isResizing=false) but is suppressed during drag (isResizing=true) requires interactive observation.

#### 3. Reopen restores pre-collapse width, not defaultSize

**Test:** Drag the left panel to a custom width (e.g., 25%). Close the panel. Reopen it.
**Expected:** Panel reopens at 25% — the last dragged width — not at the `defaultSize="18%"`.
**Why human:** react-resizable-panels v4 `expand()` should restore pre-collapse size, but this is a library runtime behavior that must be confirmed empirically.

---

### Gaps Summary

No automated gaps. All code artifacts exist, are substantive, and are properly wired. The build compiles with zero TypeScript errors. Commits 67e377f, bfea1d4, f506f15, and b5d190e are all present in git log.

The three human verification items above relate to Success Criterion 4 (panel resize and size persistence) — the interactive runtime behaviors of react-resizable-panels v4 that cannot be validated from static code inspection. These are not code gaps; the implementation appears correct. Human verification is a confidence check.

---

_Verified: 2026-03-14_
_Verifier: Claude (gsd-verifier)_
