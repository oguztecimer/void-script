# External Integrations

**Analysis Date:** 2026-03-14

## APIs & External Services

**None Detected:**
- No third-party API integrations found in codebase
- All functionality is self-contained within the application
- No network calls to external services identified

## Data Storage

**Databases:**
- None - Not used
- Application uses local filesystem only

**File Storage:**
- Local filesystem only
- Script storage location: User-defined directory managed by `ScriptStore` (`crates/voidscript-editor/src/scripts.rs`)
- Scripts stored as `.vs` files (VOID//SCRIPT format)
- Storage accessed via standard Rust `std::fs` operations
- No database ORM or connection pooling needed

**Caching:**
- None - Not implemented
- Application maintains in-memory cache of loaded scripts via `HashMap<String, Script>` in `ScriptStore`

## Authentication & Identity

**Auth Provider:**
- None - Not used
- Application is single-user, desktop-only
- No user authentication or authorization layer
- Scripts managed locally with no multi-user access control

## Monitoring & Observability

**Error Tracking:**
- None - Not integrated
- Errors logged to stderr via `eprintln!()` macro for IPC parse failures (`crates/voidscript-editor/src/window.rs`)
- Console output sent to UI via IPC messages (ConsoleOutput variant)

**Logs:**
- Console logging via Bevy's `info!()` macro
- Example: WebView attachment confirmation (`crates/voidscript-editor/src/window.rs`)
- No structured logging framework
- All logs output to stdout/stderr

## CI/CD & Deployment

**Hosting:**
- Standalone desktop application
- Compiled to native executable (cross-platform Bevy binary)
- No server infrastructure or cloud hosting required
- macOS primary platform

**CI Pipeline:**
- None detected - No CI configuration files found
- Build requires Cargo for Rust compilation and npm for UI dependencies

## Environment Configuration

**Required env vars:**
- None identified - Application is fully self-contained
- No external service credentials needed
- Runtime configuration via script directory path (hardcoded or configured at startup)

**Secrets location:**
- Not applicable - No secrets management needed
- No API keys, authentication tokens, or credentials required

## Webhooks & Callbacks

**Incoming:**
- None - Not applicable to desktop application

**Outgoing:**
- None - Application does not make external API calls
- IPC communication is unidirectional (Rust → JavaScript, JavaScript → Rust) within same process

## Desktop Platform Integration

**Window Management (macOS):**
- Wry 0.50 - WebView embedding in Bevy window
- Custom protocol handler `voidscript://` for serving embedded UI assets
- No external HTTP server required

**File System:**
- Script persistence via standard filesystem operations
- Script discovery by file extension (`.vs` files)
- Path operations via `std::path::PathBuf`

---

*Integration audit: 2026-03-14*
