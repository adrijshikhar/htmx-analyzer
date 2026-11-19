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
* **Zero-copy tokenization (`m01-ownership`, `m04-zero-cost`):** Extracts exact byte ranges with zero heap allocations during scanning:
  * ID definitions: `IdDefinition<'a> { pub id: &'a str, pub value_range: Range<usize>, pub tag_range: Range<usize> }`.
  * Target ID references (`m05-type-driven`): `IdReference<'a> { pub id: &'a str, pub span: Range<usize>, pub origin_span: Range<usize> }`. Identifies attributes (`hx-target`, `hx-indicator`, `hx-swap-oob`, `hx-sync`), strips leading `#`, handles prefix flags (e.g. `true:`), and provides the target ID slice with its source ranges.
* **Dynamic skipping:** Reuses `AttrValue::is_dynamic()` to skip template interpolations (e.g. `id="item-{{ .ID }}"`).
* **Infallible parsing (`m06-error-handling`):** Parser functions return zero-copy iterators or slices, never panic on unclosed tags or malformed quotes, and adhere to `#![forbid(unsafe_code)]`.

### 2.2. `crates/htmx-service`
* **`WorkspaceIdIndex` (`m10-performance`, `m02-resource`):** Manages an in-memory symbol table optimized for low memory footprint and zero heap indirection for the 99.9% common case:
  ```rust
  use smallvec::SmallVec;

  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct IdLocation {
      pub uri: lsp_types::Url,
      pub range: lsp_types::Range,        // Exact span of "my-id"
      pub tag_range: lsp_types::Range,    // Span of <div id="my-id">
  }

  #[derive(Default, Debug, Clone)]
  pub struct WorkspaceIdIndex {
      /// HTML IDs are virtually always unique across templates; SmallVec<[IdLocation; 1]>
      /// inlines the single location directly in the map entry with zero heap allocation.
      defs: HashMap<String, SmallVec<[IdLocation; 1]>>,
      file_to_ids: HashMap<lsp_types::Url, Vec<String>>,
  }
  ```
* **Concurrency Model (`m07-concurrency`, `m03-mutability`):**
  * Wrap `WorkspaceIdIndex` in `parking_lot::RwLock<WorkspaceIdIndex>` inside `DocumentStore`.
  * Read operations (`goto_definition`, `find_references`, `diagnostics`) acquire read locks (`index.read()`) concurrently with zero blocking between requests.
  * Write operations (startup crawler, incremental `didChange`) acquire write locks briefly. The crawler aggregates all files in a local staging index on a worker thread and merges in a single lock acquisition to prevent reader starvation.
* **`navigation.rs` (`m05-type-driven`):**
  * `goto_definition(uri, position, doc, index, link_support) -> Option<lsp_types::GotoDefinitionResponse>`:
    * When client supports `linkSupport`, returns `GotoDefinitionResponse::Link(Vec<LocationLink>)` with `origin_selection_range` highlighting the exact `#target-id` token, and `target_range` / `target_selection_range` pointing to the element and ID.
    * Fallback returns `GotoDefinitionResponse::Scalar(Location)` or `GotoDefinitionResponse::Array(Vec<Location>)`.
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

## 5. Micro-benchmarking Suite (`golang-benchmark`, `m10-performance`)

Benchmarks are added to `crates/htmx-service/benches/navigation.rs` using Criterion, adopting the statistical rigor and table-driven methodology from Go & Rust performance standards:
* **Table-driven fixture sizing:**
  * S (Small): 100 lines, 10 IDs.
  * M (Medium): 1,000 lines, 100 IDs.
  * L (Large): 10,000 lines, 1,000 IDs.
* **Separation of setup vs measurement:**
  * Document creation and indexing are excluded from hot-loop lookup measurement.
  * Inputs and return values are shielded with `criterion::black_box` to prevent dead-code elimination.
* **Throughput and Custom Metrics:**
  * Benchmark groups declare `group.throughput(Throughput::Bytes(...))` and `group.throughput(Throughput::Elements(lines))` to measure scan and resolution velocity.
* **Positive vs Negative Workloads:**
  * `bench_goto_definition/hit`: Resolution time for existing IDs. SLA: `< 20 µs` (10k lines).
  * `bench_goto_definition/miss`: Resolution time for non-existent IDs (verifying worst-case diagnostic check). SLA: `< 25 µs`.
  * `bench_find_references`: Reference search across multi-thousand-line documents. SLA: `< 150 µs`.
  * `bench_workspace_index_build`: Time to crawl and index 100 simulated template files. SLA: `< 15 ms`.

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
