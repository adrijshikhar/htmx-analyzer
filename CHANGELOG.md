# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-11-18

### Added
- Linear attribute lexer replacing tree-sitter: parses HTML and template markup in a single zero-copy byte scan without external C dependencies.
- First-class support for 10 template languages: HTML, Jinja2, Django, PHP, HTML+ERB, Blade, Vue.js, Astro, Svelte, and Templ.
- Configurable diagnostic severity via LSP `initializationOptions` and `workspace/didChangeConfiguration` (`diagnostics.severity` mapping).
- Schema additions: `hx-disable`, `hx-inherit`, `hx-request` attributes, plus DOM trigger events (`input`, `focus`, `blur`, `keypress`, `mouseover`, `mouseout`, `mousedown`, `mouseup`, `scroll`, `reset`).
- Canonical attribute name handling: case-insensitive matching and transparent support for `data-hx-*` attribute variants.
- Duration syntax validation: diagnostics for invalid timing in `swap:`/`settle:` modifiers (`HTMX002`) and `every <duration>` polling intervals (`HTMX003`).
- Resilient quick-fix code actions for migrating `hx-ws` and `hx-sse` to HTMX 2.0 extension syntax (`hx-ext="ws"` / `hx-ext="sse"`), merging with existing extension lists and escaping quotes properly.
- Target ID completion capping: results capped at 200 items with `isIncomplete: true` to prevent client UI stalls in huge documents.
- Unbounded stdio pump thread and batching loop: coalesces rapid `didChange` bursts and prioritizes requests over diagnostics.
- Spec-compliant lifecycle handling: rejects requests after `shutdown`, joins IO threads before clean exit, and isolates panics via `std::panic::catch_unwind`.
- 5 MiB document size guard (`MAX_FILE_SIZE`) with a warning `window/logMessage` notification.
- Automated release build provenance attestations and cross-compiled binaries for 6 platforms (Linux x64/arm64 musl, macOS arm64/x64, Windows x64/arm64).

### Changed
- `HTMX004` (unknown target ID) severity default changed from `Warning` to `Hint`.
- Removed `tree-sitter` and `tree-sitter-html` dependencies entirely.
- Minimum Supported Rust Version (MSRV) established at Rust 1.85.

### Fixed
- Fixed parser hangs, recovery failures, and spurious diagnostics when editing template syntax (`{{ ... }}`, `<% ... %>`, `<? ... ?>`, `{ ... }`).
- Fixed server hang during shutdown when client disconnects or sends requests post-shutdown.
