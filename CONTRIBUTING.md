# Contributing to htmx-analyzer

Thank you for your interest in contributing to **htmx-analyzer**! We welcome bug reports, feature requests, documentation improvements, and code contributions.

This document outlines the development workflow, coding standards, testing instructions, and release process.

---

## Workspace Architecture

`htmx-analyzer` is structured as a Cargo workspace with modular crates:

| Crate | Responsibility |
| :--- | :--- |
| **`crates/htmx-schema`** | Static HTMX 2.0 and 1.x specification data, attribute metadata, completions, and deprecations. |
| **`crates/htmx-parser`** | Resilient linear attribute lexer and mini-parsers for swap styles and trigger specifications (Tree-Sitter free). |
| **`crates/htmx-vfs`** | In-memory Virtual File System tracking document versions and UTF-8/UTF-16 line-column offsets. |
| **`crates/htmx-service`** | Core LSP domain logic: hover, completion, diagnostics (`HTMX001`–`HTMX004`), and quick-fix code actions. |
| **`crates/htmx-lsp`** | Server library (`htmx_analyzer`) and CLI binary with JSON-RPC 2.0 stdio transport and lifecycle handling. |

---

## Development Prerequisites

- **Rust toolchain:** Minimum Supported Rust Version (MSRV) is **1.85** (`rustup default stable`).
- **Components:** `rustfmt` and `clippy`:
  ```bash
  rustup component add rustfmt clippy
  ```

---

## Getting Started

1. **Fork and clone the repository:**
   ```bash
   git clone https://github.com/adrijshikhar/htmx-analyzer.git
   cd htmx-analyzer
   ```

2. **Create a topic branch:**
   ```bash
   git checkout -b feat/your-feature-name
   # or
   git checkout -b fix/your-bugfix-name
   ```

---

## Quality Standards

All pull requests must pass our automated quality gates before merging:

1. **Safety:** `#![forbid(unsafe_code)]` is strictly enforced across every crate in the workspace.
2. **Formatting:**
   ```bash
   cargo fmt --all --check
   ```
3. **Clippy (Zero Warnings):**
   ```bash
   cargo clippy --workspace --all-targets --locked -- -D warnings
   ```
4. **Unit & Integration Tests:**
   ```bash
   cargo test --workspace --all-targets --locked
   ```
5. **Supply Chain & Licenses:**
   ```bash
   cargo deny check
   ```

---

## Performance Benchmarking & Stress Testing

### Criterion Benchmarks
Run Criterion microbenchmarks for validation and completion performance:
```bash
cargo bench -p htmx-service
```

### Stdio Stress Test Harness
Run the Python stdio stress test to simulate rapid burst typing and verify response latency and diagnostic batching:
```bash
cargo build --release -p htmx-analyzer
python3 scripts/lsp_stress.py target/release/htmx-analyzer
```

### End-to-End (E2E) LSP Test
Verify the end-to-end LSP protocol handshake and capabilities:
```bash
cargo build --release -p htmx-analyzer
python3 tests/e2e/lsp_e2e.py target/release/htmx-analyzer
```

### Fuzzing
Fuzz targets are located in the `fuzz/` directory and tested on nightly Rust using `cargo-fuzz`:
```bash
cargo +nightly fuzz run lexer -- -max_total_time=60
cargo +nightly fuzz run cursor_scan -- -max_total_time=60
```

---

## Adding or Modifying HTMX Schema & Diagnostics

- **New Attributes or Values:** Add new attribute definitions or documentation to [`crates/htmx-schema/src/registry.rs`](crates/htmx-schema/src/registry.rs).
- **New Diagnostics:** Implement diagnostic checks in [`crates/htmx-service/src/diagnostics.rs`](crates/htmx-service/src/diagnostics.rs) and assign a clear diagnostic code (e.g., `HTMX005`). Add accompanying unit tests validating positive and negative match cases.
- **Quick-Fix Code Actions:** Register migration actions in [`crates/htmx-service/src/code_action.rs`](crates/htmx-service/src/code_action.rs).

---

## Release Process

Release artifacts are built, verified, and attested automatically via GitHub Actions:

- **Immutability:** Published releases are immutable. Never re-run or re-upload assets for an existing published non-prerelease tag; if a release has an issue, cut the next patch release instead.
- **Branch Restriction:** Release tags (`vX.Y.Z`) must be cut exclusively from commits that have been merged into `main`. The release workflow verifies that the tag is a direct ancestor of `origin/main`.
- **Version Parity:** The release tag must match the version specified in `Cargo.toml`.
- **Attestation:** Every released binary is attested via `actions/attest-build-provenance`.
