# Phase 1: Foundation - Context

**Gathered:** 2026-03-14
**Status:** Ready for planning

<domain>
## Phase Boundary

Self-hosted fonts (Inter, JetBrains Mono), CSS custom property token system replacing all hardcoded hex values, and macOS rendering fixes. This is invisible infrastructure that gates all visual work in subsequent phases. No visible UI changes beyond correct font rendering.

</domain>

<decisions>
## Implementation Decisions

### Font Loading
- Variable fonts via Fontsource npm packages (`@fontsource-variable/inter`, `@fontsource-variable/jetbrains-mono`)
- Import in `main.tsx`, Vite bundles font files into `dist/`, rust-embed serves them via `voidscript://` protocol
- Add a verification step to confirm wry custom protocol serves `.woff2` files with correct MIME type
- JetBrains Mono ligatures OFF (`font-variant-ligatures: none`)
- Silent fallback to system sans-serif if fonts fail to load (`font-display: swap` or similar)
- Fallback chain: `'Inter Variable', Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif`

### Token Naming Convention
- Hybrid approach: semantic names for shared values (`--bg-editor`, `--text-primary`, `--border-default`), component-scoped names for one-off values (`--run-btn-bg`)
- Tokens cover colors AND key structural dimensions (title bar height, tab height, status bar height, strip width, border radius)
- Color values must be audited against a real Rider instance before being committed to tokens — do not assume current hex values are correct
- Mark any unverified values with `/* verify against Rider */` comments

### Token File Structure
- Single `src/theme/tokens.css` file for all tokens (colors, dimensions, typography)
- CSS Modules co-located with components: `components/Header.module.css` alongside `Header.tsx`
- `tokens.css` imported from `main.tsx` to make custom properties globally available

### macOS Rendering
- Full rendering stack applied to `html` or `:root`:
  - `-webkit-font-smoothing: antialiased`
  - `-moz-osx-font-smoothing: grayscale`
  - `color-scheme: dark`
  - `text-rendering: optimizeLegibility`
- `html { background: #1E1F22 }` set in `index.html` `<style>` block to prevent cold-launch white flash

### Claude's Discretion
- Where exactly to apply `font-variant-ligatures: none` (editor only, or all monospace text including console/debug)
- Exact fallback font-weight mapping if Inter Variable renders differently than expected
- Whether to add `font-display: swap` vs `font-display: optional` for the @font-face declarations

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `index.html` `<style>` block: already sets base font-family and background — extend with rendering properties
- `voidscript-theme.ts`: defines CodeMirror theme with font-family `'JetBrains Mono', 'Fira Code', monospace` — update to use Fontsource-loaded font

### Established Patterns
- All styling is currently inline via React `style` props with hardcoded hex values
- `font-family: inherit` used in Header widgets — will inherit from `body` once Inter is loaded
- No existing CSS files beyond `index.html` inline styles — this phase introduces the CSS file pattern

### Integration Points
- `main.tsx`: entry point where Fontsource imports should be added
- `index.html` `<style>`: base rendering properties (`-webkit-font-smoothing`, `color-scheme`, `html background`)
- `voidscript-theme.ts`: CodeMirror font-family declaration needs updating
- `embedded_assets.rs` / `rust-embed`: must serve `.woff2` files with `font/woff2` MIME type via `mime_guess`

</code_context>

<specifics>
## Specific Ideas

- Reference: JetBrains Rider New UI uses Inter Variable as its universal UI font
- The editor should feel identical to Rider's font rendering on macOS — same weight, same smoothing
- Audit Rider's actual color values before locking tokens (don't assume current hex values are correct)

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 01-foundation*
*Context gathered: 2026-03-14*
