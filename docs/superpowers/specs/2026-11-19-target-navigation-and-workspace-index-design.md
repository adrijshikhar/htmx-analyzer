# Technical Design Spec: HTMX Target Navigation & Workspace Indexing

**Author:** Antigravity & Adrij Shikhar  
**Date:** 2026-11-19  
**Status:** Approved for Implementation  
**Target Repository:** `adrijshikhar/htmx-analyzer`  

---

## 1. Executive Summary

This specification outlines the architecture, data structures, and implementation plan for **Target Navigation** and **Lightweight Workspace Indexing** in `htmx-analyzer`. 

It enables first-class IDE navigation:
* **Go to Definition (`textDocument/definition`):** Jumping from `hx-target="#id"`, `hx-indicator="#id"`, `hx-swap-oob="true:#id"`, and `hx-sync="#id:event"` directly to the declaring `<tag id="id">`.
* **Find References (`textDocument/references`):** Discovering all HTMX attributes in the document and workspace that reference a given element `<tag id="id">`.
* **Cross-File Template Awareness:** Resolves target IDs across partials and layout files (e.g. `base.html` ↔ `partials/todo_item.html`) using a lightweight, in-memory workspace index built on startup.
* **Micro-benchmarking:** Rigorous performance benchmarks via Criterion measuring definition lookup, reference collection, and workspace index generation under realistic multi-thousand-line template corpora.

---

## 2. Architecture & Component Boundaries

The implementation respects strict boundaries across the workspace crates:

```
┌────────────────────────────────────────────────────────┐
│  htmx-lsp (LSP Server Entrypoint)                      │
│  - Registers definitionProvider & referencesProvider   │
│  - Handles textDocument/definition & references RPCs   │
│  - Drives startup workspace scan on `initialized`      │
└───────────────────────────┬────────────────────────────┘
                            │
                            ▼
┌────────────────────────────────────────────────────────┐
│  htmx-service (Pure Language Engine)                   │
│  - navigation.rs: goto_definition & find_references    │
│  - WorkspaceIdIndex: in-memory defs & file mappings    │
│  - Validates and links target IDs across documents     │
└───────────────────────────┬────────────────────────────┘
                            │
                            ▼
┌────────────────────────────────────────────────────────┐
│  htmx-parser (Linear Lexer & AST Tokens)               │
│  - find_id_definitions: exact byte ranges of id="..."  │
│  - find_id_reference_at: cursor token resolution       │
│  - Zero-I/O, zero-copy, `#![forbid(unsafe_code)]`      │
└────────────────────────────────────────────────────────┘
```

### 2.1. `crates/htmx-parser`
* **Zero-copy tokenization:** Extracts exact byte ranges for:
  * ID definitions: `IdDefinition { id: &'a str, value_range: Range<usize>, tag_range: Range<usize> }`.
  * Target ID references: Identifies attributes (`hx-target`, `hx-indicator`, `hx-swap-oob`, `hx-sync`), strips leading `#`, handles prefix flags (e.g. `true:`), and returns the target ID string with its byte range.
* **Dynamic skipping:** Reuses `AttrValue::is_dynamic()` to skip template interpolations (e.g. `id="item-{{ .ID }}"`).

### 2.2. `crates/htmx-service`
* **`WorkspaceIdIndex`:** Manages an in-memory symbol table:
  ```rust
  pub struct IdLocation {
      pub uri: lsp_types::Url,
      pub range: lsp_types::Range,        // Exact span of "my-id"
      pub tag_range: lsp_types::Range,    // Span of <div id="my-id">
  }

  pub struct WorkspaceIdIndex {
      defs: HashMap<String, Vec<IdLocation>>,
      file_to_ids: HashMap<lsp_types::Url, Vec<String>>,
  }
  ```
* **`navigation.rs`:**
  * `goto_definition(uri, position, doc, index) -> Option<lsp_types::GotoDefinitionResponse>`
  * `find_references(uri, position, doc, index, include_declaration) -> Vec<lsp_types::Location>`
* **Diagnostics integration:** Enhances `HTMX004` (unknown target ID) to check `WorkspaceIdIndex` before flagging, eliminating false warnings in template partials.

### 2.3. `crates/htmx-lsp`
* **Capabilities:** Declares `definition_provider: Some(GotoCapability::default())` and `references_provider: Some(OneOf::Left(true))` in `ServerCapabilities`.
* **Workspace Discovery:** On `initialized`, spawns a fast background thread scanning workspace directories (`*.html`, `*.jinja`, `*.blade.php`, `*.templ`, etc.), skipping ignored paths (`target`, `node_modules`, `.git`, `vendor`).
* **Cache Sync:** Updates `WorkspaceIdIndex` on `textDocument/didChange` and `textDocument/didClose`.

---

## 3. Data Flow & Resolution Algorithms

### 3.1. Go to Definition Flow
1. Receive `textDocument/definition` with `(uri, position)`.
2. Convert LSP position to byte offset via `htmx-vfs::RopeDocument`.
3. Check if cursor is on an ID reference attribute:
   * `hx-target`: `#my-id`, `this, #my-id`, `find #my-id`
   * `hx-indicator`: `#loading-spinner`
   * `hx-swap-oob`: `true:#my-id`, `#my-id`
   * `hx-sync`: `#form-id:abort`
4. If matched, extract `#my-id` ➔ `"my-id"`.
5. Lookup `"my-id"` in `WorkspaceIdIndex`:
   * Return `GotoDefinitionResponse::Scalar(Location)` if single definition.
   * Return `GotoDefinitionResponse::Array(Vec<Location>)` if duplicate IDs exist.
   * Return `None` if not found or if cursor is on a relative keyword (`this`, `closest`).

### 3.2. Find References Flow
1. Receive `textDocument/references` with `(uri, position, context)`.
2. Determine symbol under cursor:
   * On `<* id="my-id">` ➔ `"my-id"`.
   * On `hx-*="#my-id"` ➔ `"my-id"`.
3. Query all occurrences in open documents and the workspace index:
   * If `context.include_declaration == true`, include `<* id="my-id">`.
   * Include all matching `hx-target`, `hx-indicator`, `hx-swap-oob`, and `hx-sync` attribute positions.
4. Return `Vec<Location>`.

---

## 4. Edge Cases & Error Handling

1. **Dynamic Template Expressions:**
   * Attributes with `{{...}}`, `<%...%>`, `<?...?>` are treated as dynamic and excluded from static indexing to prevent false jumps.
2. **Relative Selectors:**
   * Selectors like `this`, `closest tr`, `find .card`, `next li` return `None` cleanly rather than throwing errors.
3. **Compound Targets:**
   * In `hx-target="this, #results"`, the cursor position dictates whether `None` (on `this`) or a jump (on `#results`) occurs.
4. **Invalid Markup & Unterminated Quotes:**
   * Linear lexer never hangs or panics; continues scanning past stray quotes and malformed tags.
5. **Surrogate Pairs & UTF-16:**
   * Character offsets are calculated via `ropey` and `htmx-vfs` ensuring non-ASCII and emoji lines do not shift column targets.
6. **Workspace Scale Limits:**
   * File crawler limits index to `10,000` files and skips files `> 5 MiB` (`MAX_FILE_SIZE`).

---

## 5. Micro-benchmarking Suite

Added to `crates/htmx-service/benches/navigation.rs` using Criterion:
* `bench_goto_definition_10k`: Resolution time in a 10,000-line document with 500 ID definitions. Target: `< 50 µs`.
* `bench_find_references_10k`: Reference search across 10,000 lines. Target: `< 200 µs`.
* `bench_workspace_index_build`: Time to crawl and index 100 simulated template files. Target: `< 15 ms`.

---

## 6. Implementation Checklist & Verification

* [ ] `htmx-parser`: Implement `IdDefinition` and `find_id_reference_at`.
* [ ] `htmx-service`: Implement `WorkspaceIdIndex` and `navigation.rs` (`goto_definition`, `find_references`).
* [ ] `htmx-service`: Update `diagnostics.rs` to query `WorkspaceIdIndex`.
* [ ] `htmx-service`: Add unit tests and Criterion microbenchmarks.
* [ ] `htmx-lsp`: Register capabilities in `server.rs` and handle `textDocument/definition` & `textDocument/references`.
* [ ] `htmx-lsp`: Implement background workspace crawler and cache sync on `didChange`.
* [ ] Protocol E2E tests verifying JSON-RPC responses.
* [ ] Verification: `cargo test --workspace` (100% passing) and `cargo clippy` (0 warnings).
