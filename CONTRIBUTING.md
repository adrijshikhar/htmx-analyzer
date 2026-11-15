# Contributing to htmx-analyzer

Thank you for your interest in contributing to **htmx-analyzer**! We welcome bug reports, feature requests, documentation improvements, and code contributions.

This document outlines the development workflow, coding standards, and how to submit pull requests.

---

## Workspace Architecture

`htmx-analyzer` is structured as a Cargo workspace with modular crates:

| Crate | Responsibility |
| :--- | :--- |
| **`crates/htmx-schema`** | Static HTMX 2.0 and 1.x specification data, attribute metadata, completions, and deprecations. |
| **`crates/htmx-parser`** | HTML document parser, attribute extraction, and mini-parsers for swap styles and trigger specifications. |
| **`crates/htmx-vfs`** | In-memory Virtual File System tracking document versions and UTF-8/UTF-16 line-column offsets. |
| **`crates/htmx-service`** | Core LSP domain logic: hover, completion, diagnostics (`HTMX001`–`HTMX004`), and quick-fix code actions. |
| **`crates/htmx-lsp`** | CLI entry point, JSON-RPC 2.0 stdio transport, and server lifecycle loop (`initialize`, `shutdown`, `exit`). |

---

## Development Prerequisites

- **Rust toolchain:** Latest stable Rust (`rustup default stable`).
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

---

## Adding or Modifying HTMX Schema & Diagnostics

- **New Attributes or Values:** Add new attribute definitions or documentation to [`crates/htmx-schema/src/lib.rs`](file:///Users/nemesis/Projects/my-projects/htmx-analyzer/crates/htmx-schema/src/lib.rs).
- **New Diagnostics:** Implement diagnostic checks in [`crates/htmx-service/src/diagnostics.rs`](file:///Users/nemesis/Projects/my-projects/htmx-analyzer/crates/htmx-service/src/diagnostics.rs) and assign a clear diagnostic code (e.g., `HTMX005`). Add accompanying unit tests validating positive and negative match cases.
- **Quick-Fix Code Actions:** Register migration actions in [`crates/htmx-service/src/code_action.rs`](file:///Users/nemesis/Projects/my-projects/htmx-analyzer/crates/htmx-service/src/code_action.rs).

---

## Submitting Pull Requests

1. Keep PRs focused on a single logical change.
2. Ensure your commit messages are descriptive and follow conventional commits (e.g., `feat: ...`, `fix: ...`, `docs: ...`, `refactor: ...`).
3. Push your branch to your fork and open a Pull Request against `main`.
4. Ensure all CI checks pass. GitHub branch protection requires the `Test & Lint` workflow to succeed before merging.
