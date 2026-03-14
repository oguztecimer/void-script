# Technology Stack

**Analysis Date:** 2026-03-14

## Languages

**Primary:**
- Rust 1.93.1 - Core engine, IPC bridge, editor application
- TypeScript 5.7.0 - Editor UI with React
- JavaScript (ES2020+) - Runtime target for TypeScript compilation

**Secondary:**
- HTML5 - Editor UI markup

## Runtime

**Environment:**
- Rust 1.93.1 (stable) - Primary application runtime
- Node.js 25.8.1 - Development only (build tooling)

**Package Manager:**
- npm 11.11.0 (Node)
- Cargo (Rust) - Version managed by rustc

## Frameworks

**Core:**
- Bevy 0.15 - Game engine framework used by all Rust crates (`voidscript-game`, `voidscript-editor`, `voidscript-sim`)
- React 19.0.0 - UI framework for editor interface (`editor-ui`)

**Editor/Development:**
- CodeMirror 6 (multiple packages) - Code editor component
  - @codemirror/view 6.35.0
  - @codemirror/state 6.5.0
  - @codemirror/language 6.10.0
  - @codemirror/autocomplete 6.18.0
  - @codemirror/commands 6.7.0
  - @codemirror/lint 6.8.0

**Build/Dev:**
- Vite 6.0.0 - Frontend build tool and dev server
- TypeScript 5.7.0 - Static type checking

## Key Dependencies

**Critical:**
- Bevy 0.15 - Game engine powering the entire application stack (`Cargo.toml` workspace dependency)
- wry 0.50 - WebView integration for embedding React UI in Bevy window (`crates/voidscript-editor/Cargo.toml`)
- React 19.0.0 - UI rendering for editor (`editor-ui/package.json`)

**Infrastructure:**
- serde 1.x with derive feature - Serialization/deserialization for IPC messages (`crates/voidscript-editor/src/ipc.rs`)
- serde_json 1.x - JSON serialization for IPC protocol
- crossbeam-channel 0.5 - Inter-thread communication between Bevy and WebView (`crates/voidscript-editor/Cargo.toml`)
- uuid 1.x (v4 feature) - Unique script identifiers (`crates/voidscript-editor/Cargo.toml`)
- http 1.x - HTTP response building for custom protocol handler (`crates/voidscript-editor/Cargo.toml`)
- rust-embed 8.x - Embedding compiled UI assets into binary (`crates/voidscript-editor/Cargo.toml`)
- mime_guess 2.x - MIME type detection for asset serving (`crates/voidscript-editor/Cargo.toml`)
- raw-window-handle 0.6 - Cross-platform window handle access (`crates/voidscript-editor/Cargo.toml`)

**macOS-specific:**
- objc2 0.6 - Objective-C interop for native macOS features
- objc2-app-kit 0.3 - macOS AppKit bindings for NSWindow, NSResponder, NSView
- objc2-foundation 0.3 - macOS Foundation framework bindings

**Frontend State:**
- Zustand 5.0.0 - Lightweight state management for React UI (`editor-ui/package.json`)

**Highlighting:**
- @lezer/highlight 1.2.0 - Syntax highlighting infrastructure for CodeMirror

## Configuration

**Environment:**
- No `.env` files detected - configuration is compile-time or hardcoded
- Runtime configuration via script files in user directory

**Build:**
- `editor-ui/tsconfig.json` - TypeScript configuration for editor UI
  - Target: ES2020
  - Strict mode enabled
  - React JSX support
- `editor-ui/vite.config.ts` - Vite build configuration
  - React plugin enabled
  - Output directory: `editor-ui/dist/`
  - Dev server port: 5173

**Asset Embedding:**
- Compiled UI assets are embedded into Bevy binary via `rust-embed` crate
- Custom protocol handler `voidscript://` serves assets from embedded storage
- No external asset loading required at runtime

## Platform Requirements

**Development:**
- Rust 1.93.1+ (tested with latest stable)
- Node.js 25.x
- npm 11.x
- TypeScript 5.7.0+
- macOS (primary dev platform)
  - macOS AppKit framework access required for native integrations
  - Objective-C interop via objc2 bindings

**Production:**
- Standalone Bevy executable (cross-platform capable)
- macOS: Native window management via AppKit
- No external runtime dependencies needed for deployed binary
- Script files stored locally in user filesystem

---

*Stack analysis: 2026-03-14*
