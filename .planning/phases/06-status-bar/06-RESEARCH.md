# Phase 6: Status Bar - Research

**Researched:** 2026-03-15
**Domain:** React component restyle — CSS Modules, Zustand, inline SVG icons
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Navigation path content**
- Path segments are dynamic based on actual script directory structure
- Default flat case: `VOID//SCRIPT › miner_brain.vs` (two segments)
- Nested case: `VOID//SCRIPT › combat › miner_brain.vs` (segments per directory level)
- Separator: chevron `›` character (Rider's breadcrumb style)
- Last segment (filename) rendered in `--text-primary`; all other segments and chevrons in `--text-secondary`
- When no file is open: show just `VOID//SCRIPT` (project name always visible)

**Status bar layout**
- Navigation path on far left
- VCS branch widget removed entirely
- Right cluster order unchanged: diagnostics, `Ln X, Col Y`, LF, UTF-8, VoidScript
- Layout: `[nav path] — spacer — [diagnostics] [cursor] [encoding] [language]`

**Segment interactivity**
- All path segments are clickable: project name opens ScriptList panel, folder segment filters to that folder, file segment scrolls to top
- Chevron separators are clickable as part of the segment to their left (larger hit target)
- Hover: text color shifts to `--text-primary` on hover, no underline or background change
- Click behavior when no file is open: Claude's discretion

### Claude's Discretion
- Diagnostics icon SVG design (error circle, warning triangle) — match Rider's icon style
- Whether error/warning counts show at 0 or only when non-zero
- Whether to keep the green "OK" state when no diagnostics
- Combined diagnostics widget vs separate StatusSegments
- Click behavior on navigation path when no file is open

### Deferred Ideas (OUT OF SCOPE)
- Folder creation for organizing scripts into directories — new capability, separate phase
- Folder filtering behavior when clicking folder segments — depends on ScriptList supporting folder views
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| STAT-01 | Navigation breadcrumb path segments in status bar left region (project > folder > file) | Path derivation from `Tab.scriptId` + `scriptList`; `NavPath` component renders segments with chevrons; `toggleLeftPanel` action wires project-name click |
| STAT-02 | Diagnostics widget with icon + count pattern replacing plain text (error icon + red count, warning icon + yellow count) | Inline SVG circle (error) and triangle (warning) icons; existing `--accent-red`, `--accent-yellow` tokens; existing `activeTab.diagnostics` array |
</phase_requirements>

---

## Summary

Phase 6 is a focused restyle of `StatusBar.tsx` with two new sub-components. No new state, no new IPC messages, and no new dependencies are needed — all required data already exists in the Zustand store.

The core challenge for STAT-01 is **path derivation**: the `Tab` interface carries only `scriptId` and `name`. The existing `ScriptInfo` type (`id`, `name`, `script_type`) also lacks a filesystem path field. Path segments must therefore be inferred from what is available — either from the tab `name` field (if it already encodes a relative path like `combat/miner_brain.vs`) or from a synthetic mapping using `script_type` as the folder segment. Examination of actual scripts (`scripts/miner_brain.vs`, `scripts/mothership_brain.vs`) shows flat layout today; the context confirms the default flat case is two segments. No Rust-side changes are needed for flat layout.

For STAT-02 the replacement is straightforward: swap `<span style={{ color: ... }}>{n} errors</span>` with a `DiagnosticsWidget` that renders inline SVG icons next to colored counts. The `StatusSegment` primitive already accepts `icon` and `label` as `React.ReactNode`, making it directly composable.

**Primary recommendation:** Build a `NavPath` component and a `DiagnosticsWidget` component, both consumed by `StatusBar.tsx`. Keep all hover behavior in CSS Modules. No new state, no new tokens needed.

---

## Standard Stack

### Core (already installed — no new packages required)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| React | 19.0.0 | Component rendering | Project foundation |
| Zustand | 5.0.0 | State (tabs, activeTabId, toggleLeftPanel) | Already in store |
| CSS Modules | Vite built-in | Scoped styling | Project-wide pattern |

### Supporting
| Tool | Version | Purpose | When to Use |
|------|---------|---------|-------------|
| Inline SVG | n/a | Icon rendering | Icons are 10-12px, no icon font needed |
| TypeScript | 5.7.0 | Type safety for segment props | Already in project |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Inline SVG icons | @radix-ui/react-icons | No new dependency justified for 2 small icons |
| Custom NavPath component | Extending StatusSegment | StatusSegment API (icon + label) doesn't cleanly represent a multi-segment breadcrumb row |

**Installation:** No new packages required.

---

## Architecture Patterns

### Current StatusBar Structure (to be replaced)
```
StatusBar.tsx
├── StatusSegment (VCS branch — REMOVE)
├── .spacer
├── StatusSegment (error count — plain text — REPLACE)
├── StatusSegment (warning count — plain text — REPLACE)
├── StatusSegment (OK — KEEP or adjust per discretion)
├── StatusSegment (Ln X, Col Y)
├── StatusSegment (LF)
├── StatusSegment (UTF-8)
└── StatusSegment (VoidScript)
```

### Target StatusBar Structure
```
StatusBar.tsx
├── NavPath             ← NEW — STAT-01
├── .spacer
├── DiagnosticsWidget   ← NEW — STAT-02
├── StatusSegment (Ln X, Col Y)
├── StatusSegment (LF)
├── StatusSegment (UTF-8)
└── StatusSegment (VoidScript)
```

### Recommended File Additions
```
editor-ui/src/
└── components/
    ├── StatusBar.tsx              (modify — replace VCS + plain diagnostics)
    ├── NavPath.tsx                (new — STAT-01)
    ├── NavPath.module.css         (new — segment + chevron styles)
    └── DiagnosticsWidget.tsx      (new — STAT-02)
```

### Pattern 1: NavPath Component
**What:** Derives path segments from active tab data, renders clickable segments separated by `›` chevrons.
**When to use:** Active tab exists (2+ segments) or no tab (project name only).

Path derivation logic (no new data needed):
- `Tab.name` currently holds the bare filename (e.g. `"miner_brain.vs"`)
- `Tab.scriptId` is an opaque ID (e.g. `"miner_brain"`)
- `ScriptInfo.script_type` is the folder-level discriminator (`"ship_brain"`, `"mothership_brain"`, `"production"`)

For the flat case (current scripts have no subdirectory), two segments suffice:
`VOID//SCRIPT › miner_brain.vs`

For the nested case (future), a folder segment sits between project and file. Since `script_type` maps to a human-readable label (see `ScriptList.tsx` `TYPE_LABELS`), we can use that as the folder segment when present.

**Example derivation:**
```typescript
// Source: project codebase analysis
function deriveSegments(activeTab: Tab | undefined, scriptList: ScriptInfo[]): PathSegment[] {
  const project: PathSegment = { label: 'VOID//SCRIPT', kind: 'project' };
  if (!activeTab) return [project];

  const info = scriptList.find((s) => s.id === activeTab.scriptId);
  const folder = info?.script_type
    ? { label: TYPE_LABELS[info.script_type] ?? info.script_type, kind: 'folder' as const }
    : null;
  const file: PathSegment = { label: activeTab.name, kind: 'file' };

  return folder ? [project, folder, file] : [project, file];
}
```

The `TYPE_LABELS` map already exists in `ScriptList.tsx` — extract to a shared `scriptTypes.ts` constant to avoid duplication.

### Pattern 2: Chevron as Part of Left Segment
**What:** Each segment except the last renders `label + " › "` as a single button, giving the chevron a larger click target without visual affordance change.
**Implementation:** Include the `›` character directly in the button's text content, styled with `--text-secondary` always (not inheriting the hover `--text-primary`).

```typescript
// Segment renders: <button>VOID//SCRIPT ›</button>  <button className={last}>miner_brain.vs</button>
// Last segment has no trailing chevron
```

### Pattern 3: NavPath Hover — Color Only, No Background
**What:** The hover contract for nav path segments differs from `StatusSegment` default — Rider breadcrumbs shift text color but do NOT change background.
**Implementation:** NavPath gets its own CSS module; do NOT reuse `StatusSegment` for these, or override with a modifier class. The simplest approach is a dedicated `NavPath.module.css`:

```css
/* Source: CONTEXT.md hover spec */
.segment {
  /* reset button defaults */
  background: transparent;
  border: none;
  font: inherit;
  font-size: var(--font-size-status);
  color: var(--text-secondary);
  padding: 0 4px;
  cursor: pointer;
  transition: color var(--transition-hover);
}
.segment:hover {
  color: var(--text-primary);
  /* NO background-color change — Rider breadcrumb style */
}
.segmentFile {
  composes: segment;
  color: var(--text-primary); /* filename always primary */
}
.segmentFile:hover {
  color: var(--text-primary);
}
.chevron {
  color: var(--text-secondary);
  /* chevron stays secondary even on button hover */
  pointer-events: none; /* chevron receives clicks via parent button */
}
```

### Pattern 4: DiagnosticsWidget Icon SVGs
**What:** Inline SVG icons sized 10×10 matching status bar 11px text.
**Rider icon style:** Error = filled circle with white `×`; Warning = filled triangle with white `!`.

Recommended SVG shapes (approximating Rider's compact icon style):

```tsx
// Error icon — red filled circle with X
const ErrorIcon = () => (
  <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
    <circle cx="5" cy="5" r="4.5" fill="var(--accent-red)" />
    <path d="M3.5 3.5l3 3M6.5 3.5l-3 3" stroke="white" strokeWidth="1.2" strokeLinecap="round"/>
  </svg>
);

// Warning icon — yellow filled triangle with !
const WarningIcon = () => (
  <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
    <path d="M5 1L9.33 8.5H0.67L5 1z" fill="var(--accent-yellow)" />
    <path d="M5 4.5v1.5" stroke="white" strokeWidth="1.2" strokeLinecap="round"/>
    <circle cx="5" cy="7.2" r="0.5" fill="white"/>
  </svg>
);
```

**Discretion resolutions (Claude's decisions for this phase):**
- Show error/warning counts only when non-zero (don't clutter bar with "0 errors")
- Retain the `OK` green state when `errorCount === 0 && warningCount === 0 && activeTab` — it's cheap and provides positive feedback
- Use separate segments per severity (error segment, warning segment) rather than a combined widget — more composable with existing `StatusSegment` primitive

### Anti-Patterns to Avoid
- **Reusing `StatusSegment` for nav path segments directly:** The hover contract differs (text-only hover vs background hover). Avoid fighting the existing `.segment:hover { background-color }` rule.
- **Deriving path from `Tab.name` assuming it contains slashes:** Currently `name` is just the bare filename. Don't assume slash-separated relative paths unless confirmed by Rust backend.
- **Adding a new token for the nav path chevron:** The `--text-secondary` token already serves this role. Don't add `--text-nav-chevron`.
- **Mutating `StatusSegment` to support "no background on hover":** Adding a `noHoverBg` boolean prop to `StatusSegment` spreads complexity. Use a dedicated `NavPath.module.css` instead.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Path segment list from tab data | Custom path parser | Simple `deriveSegments()` pure function | No filesystem path exists; derivation is 3-line logic |
| Icon SVG management | Icon component library | Inline SVG literals in component | Only 2 icons; zero dependency overhead |
| Hover transitions | JS mouseEnter/mouseLeave | CSS `:hover` + `transition` | Project convention since Phase 2; no re-renders |
| Diagnostic counts | Manual filter calls | Already computed in StatusBar.tsx | `activeTab?.diagnostics.filter(...)` pattern already there |

**Key insight:** This phase is pure presentational work. All state exists; all tokens exist; all transitions are CSS. Zero new abstractions are necessary beyond the two new components.

---

## Common Pitfalls

### Pitfall 1: Chevron Click Target Bleed
**What goes wrong:** If chevron is a sibling `<span>` outside the button, clicking it fires no action. If it is a separate button, it gets a separate hover state.
**Why it happens:** The CONTEXT.md spec says "chevrons are clickable as part of the segment to their left."
**How to avoid:** Include the `›` character inside the preceding segment's `<button>` element. Style it with `--text-secondary` and suppress its own hover by targeting it as a child: `.segment:hover .chevron { color: var(--text-secondary) }` to keep it from shifting.
**Warning signs:** Clicking on ` › ` text does nothing — check DevTools to confirm it's inside the button.

### Pitfall 2: CSS Modules Composing Across Files
**What goes wrong:** Using `composes: segment from './StatusSegment.module.css'` and then overriding hover background fails when specificity order changes.
**Why it happens:** `composes` inlines class names but doesn't guarantee declaration order.
**How to avoid:** Don't compose across files for overrides. Use a self-contained `NavPath.module.css` with no cross-file composes.

### Pitfall 3: Script Type Label Duplication
**What goes wrong:** `TYPE_LABELS` is currently defined inline in `ScriptList.tsx`. If `NavPath.tsx` re-defines it, they can drift.
**Why it happens:** Two components needing the same mapping.
**How to avoid:** Extract `TYPE_LABELS` and `TYPE_ORDER` to `src/state/scriptTypes.ts` and import from both components in one task.

### Pitfall 4: `script_type` May Be Undefined
**What goes wrong:** If `scriptList` hasn't loaded yet (race between `script_list` IPC message and render), `info` is undefined, and `info?.script_type` is undefined — nav path shows only project + file, no folder segment.
**Why it happens:** `scriptList` starts as `[]` in the Zustand store; `tabs` are populated by `script_load` messages which can arrive before or after `script_list`.
**How to avoid:** The two-segment fallback (project + file) is the correct graceful degradation. Don't show a broken folder segment; the CONTEXT.md confirms flat case is the default.

### Pitfall 5: Inline CSS tokens not updated
**What goes wrong:** If any new CSS token were added, it would need to be inlined in `index.html` for wry compatibility (project decision from Phase 1).
**Why it happens:** wry WKWebView does not apply `:root` blocks from external stylesheets.
**How to avoid:** This phase requires NO new tokens — all needed tokens (`--text-primary`, `--text-secondary`, `--accent-red`, `--accent-yellow`, `--accent-green`, `--font-size-status`, `--transition-hover`) are already in `index.html`. No `index.html` changes needed.

---

## Code Examples

Verified patterns from project codebase:

### Existing StatusBar pattern to replace
```typescript
// Source: editor-ui/src/components/StatusBar.tsx (current)
// REMOVE: VCS branch segment
<StatusSegment
  icon={<svg>...</svg>}
  label="main"
/>
// REPLACE: plain-text diagnostics
{errorCount > 0 && (
  <StatusSegment label={<span style={{ color: 'var(--accent-red)' }}>{errorCount} errors</span>} />
)}
```

### StatusSegment primitive signature (unchanged)
```typescript
// Source: editor-ui/src/primitives/StatusSegment.tsx
interface StatusSegmentProps {
  icon?: React.ReactNode;
  label: React.ReactNode;
  onClick?: () => void;
}
// Renders <button> when onClick provided, <div> otherwise
```

### Zustand selectors available
```typescript
// Source: editor-ui/src/state/store.ts
const activeTabId = useStore((s) => s.activeTabId);
const tabs = useStore((s) => s.tabs);          // Tab[] — each has scriptId, name
const scriptList = useStore((s) => s.scriptList); // ScriptInfo[] — each has id, name, script_type
const toggleLeftPanel = useStore((s) => s.toggleLeftPanel);
// Derive active tab:
const activeTab = tabs.find((t) => t.scriptId === activeTabId);
```

### CSS Module hover convention (project standard)
```css
/* Source: editor-ui/src/primitives/StatusSegment.module.css */
.segment {
  transition: background-color var(--transition-hover), color var(--transition-hover);
}
.segment:hover {
  background-color: var(--bg-hover);
  color: var(--text-primary);
}
/* NavPath DEVIATES — no background change on hover */
```

---

## State of the Art

| Old Approach | Current Approach | Impact |
|--------------|-----------------|--------|
| VCS branch widget (git branch icon + "main") | Navigation path (project › folder › file) | Removes fake context; adds actual file orientation |
| Plain colored text for diagnostics (`2 errors`) | Icon + count pairs (red circle × + count) | More compact; matches Rider visual language |

**Deprecated in this phase:**
- VCS branch `<StatusSegment>` block: replaced by `<NavPath>`
- Plain-text `{n} errors` / `{n} warn` labels: replaced by `<DiagnosticsWidget>`

---

## Open Questions

1. **Does `Tab.name` ever contain a path separator?**
   - What we know: Current scripts are flat (`miner_brain.vs`, `mothership_brain.vs`); `name` field is set by Rust `script_load` message
   - What's unclear: If Rust ever sends `"combat/miner_brain.vs"` as the name, the derivation strategy needs updating
   - Recommendation: Implement flat-case derivation using `script_type` as folder segment. If Rust later sends slash-separated names, revisit. Document the assumption in a comment.

2. **Click behavior on project name when no file is open**
   - What we know: CONTEXT.md says "click behavior when no file is open: Claude's discretion"
   - Recommendation: Project name click always calls `toggleLeftPanel()` regardless of whether a file is open — consistent and predictable.

3. **Folder segment click behavior**
   - What we know: CONTEXT.md defers "folder filtering behavior" to a future phase
   - Recommendation: Render folder segment as a non-interactive `<div>` (no `onClick`), or wire to `toggleLeftPanel()` as a fallback. Do NOT implement filtering. Use `<div>` to avoid the `cursor: pointer` affordance that implies filtering.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | None installed — no `vitest`, `jest`, or test runner in package.json |
| Config file | None — Wave 0 gap |
| Quick run command | N/A — no test runner |
| Full suite command | N/A — no test runner |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| STAT-01 | NavPath derives correct segments from active tab + scriptList | unit | N/A — no test runner | ❌ Wave 0 |
| STAT-01 | NavPath shows project name only when no tab active | unit | N/A — no test runner | ❌ Wave 0 |
| STAT-02 | DiagnosticsWidget renders error icon + count when errors > 0 | unit | N/A — no test runner | ❌ Wave 0 |
| STAT-02 | DiagnosticsWidget renders OK when no diagnostics | unit | N/A — no test runner | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** TypeScript compiler check: `cd editor-ui && npx tsc --noEmit`
- **Per wave merge:** Same — no test runner; visual verification in wry/browser
- **Phase gate:** `npx tsc --noEmit` green + visual inspection before `/gsd:verify-work`

### Wave 0 Gaps
Given no test runner exists in this project and no previous phase has introduced one, and given the project's velocity pattern (all 8 completed plans used visual verification), adding a test runner in Wave 0 would be disproportionate overhead for a 2-component restyle phase.

**Decision:** Skip automated tests for this phase. TypeScript compilation serves as the lightweight gate. This is consistent with all prior phases.

*(If a test runner is introduced in a future phase, the unit tests described above should be the first candidates.)*

---

## Sources

### Primary (HIGH confidence)
- Project codebase (`editor-ui/src/components/StatusBar.tsx`) — current implementation analyzed
- Project codebase (`editor-ui/src/primitives/StatusSegment.tsx`, `StatusSegment.module.css`) — primitive API confirmed
- Project codebase (`editor-ui/src/state/store.ts`) — Zustand store shape confirmed
- Project codebase (`editor-ui/src/ipc/types.ts`) — `Tab`, `ScriptInfo` type shapes confirmed
- Project codebase (`editor-ui/src/theme/tokens.css`, `editor-ui/index.html`) — all required tokens already present
- `.planning/phases/06-status-bar/06-CONTEXT.md` — locked decisions and constraints

### Secondary (MEDIUM confidence)
- JetBrains Rider New UI visual pattern (breadcrumb + status bar) — referenced from project context; icon shapes are approximations matching the style intent

### Tertiary (LOW confidence — not applicable)
No external sources needed; all required information is in the project codebase.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries already installed, versions confirmed
- Architecture: HIGH — component structure derived directly from existing primitives
- Path derivation: MEDIUM — `Tab.name` is bare filename today; folder segment uses `script_type` inference; assumption documented
- Pitfalls: HIGH — all grounded in project decisions and codebase inspection
- Icon SVG shapes: MEDIUM — approximations of Rider style; no pixel-exact spec available

**Research date:** 2026-03-15
**Valid until:** 2026-04-15 (stable stack — no fast-moving dependencies)
