# Target Navigation & Workspace Indexing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Go to Definition (`F12`), Find References (`Shift+F12`), cross-template partial resolution, and Criterion micro-benchmarking for HTMX target IDs (`hx-target`, `hx-indicator`, `hx-swap-oob`, `hx-sync`).

**Architecture:** A three-layer zero-copy pipeline across workspace crates:
1. `htmx-parser`: Zero-copy, allocation-free extraction of `IdDefinition<'a>` and cursor `IdReference<'a>` token scanning from AST tags.
2. `htmx-service`: Thread-safe, low-allocation `WorkspaceIdIndex` using `SmallVec<[IdLocation; 1]>` and pure `navigation.rs` provider supporting `LocationLink` and `Location` responses, plus cross-file `HTMX004` diagnostic suppression.
3. `htmx-lsp`: LSP server wiring (`definitionProvider`, `referencesProvider`), background workspace template crawler on `initialized`, and incremental sync on `didChange`.

**Tech Stack:** Rust 2021 (MSRV 1.85), `lsp-types 0.97.0`, `smallvec 1.13`, `parking_lot 0.12`, `walkdir 2.5`, `criterion 0.8`.

**Spec:** [`docs/superpowers/specs/2026-11-19-target-navigation-and-workspace-index-design.md`](file:///Users/nemesis/Projects/my-projects/htmx-analyzer/docs/superpowers/specs/2026-11-19-target-navigation-and-workspace-index-design.md)

## Global Constraints

- `#![forbid(unsafe_code)]` enforced across all workspace crates.
- Zero clippy warnings on `cargo clippy --workspace --all-targets -- -D warnings`.
- Zero compiler warnings and 100% test pass rate on `cargo test --workspace`.
- Linear-time execution; zero allocations in parser hot paths.
- Memory safety: skip files `> 5 MiB` (`MAX_FILE_SIZE`) and cap crawler at 10,000 files.

---

### Task 1: Parser ID Definitions and Cursor Reference Tokenization

**Files:**
- Modify: `crates/htmx-parser/src/lexer.rs`
- Modify: `crates/htmx-parser/src/lib.rs`
- Test: `crates/htmx-parser/tests/navigation_tokens.rs`

**Interfaces:**
- Consumes: `Tag<'a>`, `Attribute<'a>`, `AttrValue<'a>` from `crates/htmx-parser/src/lexer.rs`.
- Produces:
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct IdDefinition<'a> {
      pub id: &'a str,
      pub value_range: Range<usize>,
      pub tag_range: Range<usize>,
  }

  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct IdReference<'a> {
      pub id: &'a str,
      pub span: Range<usize>,         // byte range of "my-id"
      pub origin_span: Range<usize>,  // byte range including '#' or prefix (e.g. "#my-id")
  }

  pub fn find_id_definitions<'a>(tags: &[Tag<'a>]) -> Vec<IdDefinition<'a>>;
  pub fn find_id_references<'a>(tags: &[Tag<'a>]) -> Vec<IdReference<'a>>;
  pub fn find_id_reference_at<'a>(tags: &[Tag<'a>], offset: usize) -> Option<IdReference<'a>>;
  ```

- [ ] **Step 1: Write failing unit test for `find_id_definitions` and `find_id_reference_at`**

Create `crates/htmx-parser/tests/navigation_tokens.rs`:
```rust
use htmx_parser::{find_id_definitions, find_id_reference_at, find_id_references, lex};

#[test]
fn test_find_id_definitions() {
    let html = r#"<div id="first-item" class="p-4"><span id="second-item"></span></div>"#;
    let tags = lex(html);
    let defs = find_id_definitions(&tags);
    assert_eq!(defs.len(), 2);
    assert_eq!(defs[0].id, "first-item");
    assert_eq!(&html[defs[0].value_range.clone()], "first-item");
    assert_eq!(&html[defs[0].tag_range.clone()], r#"<div id="first-item" class="p-4">"#);
    assert_eq!(defs[1].id, "second-item");
}

#[test]
fn test_find_id_reference_at_hx_target() {
    let html = r#"<button hx-target="#results" hx-indicator="#spinner">Click</button>"#;
    let tags = lex(html);
    
    // Offset inside "#results"
    let offset_target = html.find("results").unwrap() + 2;
    let ref_item = find_id_reference_at(&tags, offset_target).expect("reference found");
    assert_eq!(ref_item.id, "results");
    assert_eq!(&html[ref_item.span.clone()], "results");
    assert_eq!(&html[ref_item.origin_span.clone()], "#results");

    // Offset inside "#spinner"
    let offset_ind = html.find("spinner").unwrap() + 1;
    let ref_ind = find_id_reference_at(&tags, offset_ind).expect("reference found");
    assert_eq!(ref_ind.id, "spinner");
}

#[test]
fn test_find_id_reference_at_element_id() {
    let html = r#"<div id="target-div">Content</div>"#;
    let tags = lex(html);
    let offset = html.find("target-div").unwrap() + 3;
    let ref_item = find_id_reference_at(&tags, offset).expect("element id found as reference");
    assert_eq!(ref_item.id, "target-div");
    assert_eq!(&html[ref_item.span.clone()], "target-div");
}
```

- [ ] **Step 2: Run test to confirm failure**

```bash
cargo test -p htmx-parser --test navigation_tokens
```
Verify that compilation fails because `IdDefinition`, `find_id_definitions`, etc., are not yet declared.

- [ ] **Step 3: Implement `find_id_definitions`, `find_id_references`, and `find_id_reference_at` in `crates/htmx-parser/src/lexer.rs`**

Add structs and functions to `crates/htmx-parser/src/lexer.rs`:
- `IdDefinition<'a>` with `id`, `value_range`, and `tag_range`.
- `IdReference<'a>` with `id`, `span`, and `origin_span`.
- `find_id_definitions<'a>`: Scans `tags` for attributes with `name.eq_ignore_ascii_case("id")`, skipping dynamic values (`val.is_dynamic()`), extracting `val.range` and tag `start..end`.
- `find_id_references<'a>`: Scans `tags` for:
  1. `id="..."` attributes (element declarations themselves are reference points).
  2. HTMX target attributes:
     - `hx-target`: parses comma/space-separated targets, resolves `#<id>` spans.
     - `hx-indicator`: parses `#<id>`.
     - `hx-swap-oob`: parses `#<id>` or `true:#<id>`.
     - `hx-sync`: parses `#<id>:<strategy>`.
- `find_id_reference_at<'a>`: Checks if `offset` falls within the span of any reference found.
- Re-export in `crates/htmx-parser/src/lib.rs`.

- [ ] **Step 4: Run tests and verify they pass**

```bash
cargo test -p htmx-parser --test navigation_tokens
cargo clippy -p htmx-parser --all-targets -- -D warnings
```

- [ ] **Step 5: Commit changes**

```bash
git add crates/htmx-parser/
git commit -m "feat(parser): add IdDefinition and find_id_reference_at token extractors"
```

---

### Task 2: Service `WorkspaceIdIndex` and Target Navigation Provider

**Files:**
- Modify: `Cargo.toml` (workspace dependencies: `smallvec`)
- Modify: `crates/htmx-service/Cargo.toml`
- Create: `crates/htmx-service/src/navigation.rs`
- Modify: `crates/htmx-service/src/diagnostics.rs`
- Modify: `crates/htmx-service/src/lib.rs`
- Test: `crates/htmx-service/tests/navigation_test.rs`

**Interfaces:**
- Consumes: `htmx-parser::{find_id_definitions, find_id_references, find_id_reference_at, lex, IdDefinition, IdReference}` and `htmx-vfs::Document`.
- Produces:
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct IdLocation {
      pub uri: lsp_types::Url,
      pub range: lsp_types::Range,        // Exact span of ID text
      pub tag_range: lsp_types::Range,    // Span of enclosing element
  }

  #[derive(Default, Debug, Clone)]
  pub struct WorkspaceIdIndex {
      defs: HashMap<String, SmallVec<[IdLocation; 1]>>,
      file_to_ids: HashMap<lsp_types::Url, Vec<String>>,
  }

  impl WorkspaceIdIndex {
      pub fn new() -> Self;
      pub fn update_file(&mut self, uri: &lsp_types::Url, text: &str);
      pub fn remove_file(&mut self, uri: &lsp_types::Url);
      pub fn get_definitions(&self, id: &str) -> &[IdLocation];
      pub fn contains_id(&self, id: &str) -> bool;
  }

  pub fn goto_definition(
      uri: &lsp_types::Url,
      position: lsp_types::Position,
      doc: &Document,
      index: &WorkspaceIdIndex,
      link_support: bool,
  ) -> Option<lsp_types::GotoDefinitionResponse>;

  pub fn find_references(
      uri: &lsp_types::Url,
      position: lsp_types::Position,
      doc: &Document,
      open_docs: &[&Document],
      index: &WorkspaceIdIndex,
      include_declaration: bool,
  ) -> Vec<lsp_types::Location>;
  ```

- [ ] **Step 1: Add `smallvec` dependency**

Add `smallvec = "1.13"` to `[workspace.dependencies]` in root `Cargo.toml` and to `crates/htmx-service/Cargo.toml`.

- [ ] **Step 2: Write failing tests for `WorkspaceIdIndex` and `goto_definition` / `find_references`**

Create `crates/htmx-service/tests/navigation_test.rs`:
```rust
use htmx_service::{goto_definition, find_references, WorkspaceIdIndex};
use htmx_vfs::Document;
use lsp_types::{GotoDefinitionResponse, Position, Url};

#[test]
fn test_goto_definition_same_file() {
    let uri = Url::parse("file:///workspace/index.html").unwrap();
    let html = r#"
<div id="results">Initial content</div>
<button hx-target="#results">Click</button>
"#;
    let doc = Document::new(uri.clone(), 1, html.to_string());
    let mut index = WorkspaceIdIndex::new();
    index.update_file(&uri, html);

    // Cursor on "results" inside hx-target="#results"
    let button_line = 2;
    let col = html.lines().nth(button_line).unwrap().find("results").unwrap() as u32 + 2;
    let pos = Position::new(button_line as u32, col);

    let res = goto_definition(&uri, pos, &doc, &index, false).expect("definition found");
    match res {
        GotoDefinitionResponse::Scalar(loc) => {
            assert_eq!(loc.uri, uri);
            assert_eq!(loc.range.start.line, 1);
        }
        _ => panic!("Expected scalar location"),
    }
}

#[test]
fn test_goto_definition_cross_file_partial() {
    let base_uri = Url::parse("file:///workspace/base.html").unwrap();
    let partial_uri = Url::parse("file:///workspace/partials/item.html").unwrap();
    
    let base_html = r#"<div id="cart-counter">0</div>"#;
    let partial_html = r#"<button hx-target="#cart-counter">Add</button>"#;

    let base_doc = Document::new(base_uri.clone(), 1, base_html.to_string());
    let partial_doc = Document::new(partial_uri.clone(), 1, partial_html.to_string());

    let mut index = WorkspaceIdIndex::new();
    index.update_file(&base_uri, base_html);
    index.update_file(&partial_uri, partial_html);

    let pos = Position::new(0, partial_html.find("cart-counter").unwrap() as u32 + 2);
    let res = goto_definition(&partial_uri, pos, &partial_doc, &index, false).expect("cross-file definition found");
    match res {
        GotoDefinitionResponse::Scalar(loc) => {
            assert_eq!(loc.uri, base_uri);
            assert_eq!(loc.range.start.line, 0);
        }
        _ => panic!("Expected scalar location pointing to base.html"),
    }
}

#[test]
fn test_find_references_bidirectional() {
    let uri = Url::parse("file:///workspace/page.html").unwrap();
    let html = r#"
<div id="target-box">Box</div>
<button hx-target="#target-box">Target 1</button>
<button hx-indicator="#target-box">Target 2</button>
"#;
    let doc = Document::new(uri.clone(), 1, html.to_string());
    let mut index = WorkspaceIdIndex::new();
    index.update_file(&uri, html);

    // Cursor on <div id="target-box">
    let pos = Position::new(1, html.lines().nth(1).unwrap().find("target-box").unwrap() as u32 + 2);
    let refs = find_references(&uri, pos, &doc, &[&doc], &index, true);
    
    // Declaration + 2 references = 3
    assert_eq!(refs.len(), 3);
}
```

- [ ] **Step 3: Run test to confirm failure**

```bash
cargo test -p htmx-service --test navigation_test
```
Confirm compilation fails due to missing modules.

- [ ] **Step 4: Implement `WorkspaceIdIndex` and `navigation.rs`**

Create `crates/htmx-service/src/navigation.rs`:
- Define `IdLocation` and `WorkspaceIdIndex`.
- Implement `update_file` using `htmx_parser::lex` and `find_id_definitions`, converting byte offsets to `lsp_types::Range` via `htmx_vfs::RopeDocument`.
- Implement `goto_definition`:
  - Check cursor token via `find_id_reference_at`.
  - Lookup definitions in `WorkspaceIdIndex`.
  - Return `GotoDefinitionResponse::Link` if `link_support` is true, otherwise `Scalar` or `Array`.
- Implement `find_references`:
  - Check symbol under cursor (`find_id_reference_at`).
  - Collect declarations if `include_declaration` is true.
  - Search references across `open_docs` using `find_id_references`.
- Update `crates/htmx-service/src/diagnostics.rs`:
  - Pass `Option<&WorkspaceIdIndex>` to `validate` / `DiagnosticEngine::validate_with_index`.
  - When evaluating `HTMX004` (unknown target ID), suppress warning if `index.contains_id(id)` is true!
- Re-export `WorkspaceIdIndex`, `goto_definition`, and `find_references` in `crates/htmx-service/src/lib.rs`.

- [ ] **Step 5: Run tests and verify**

```bash
cargo test -p htmx-service
cargo clippy -p htmx-service --all-targets -- -D warnings
```

- [ ] **Step 6: Commit changes**

```bash
git add Cargo.toml crates/htmx-service/
git commit -m "feat(service): implement WorkspaceIdIndex, navigation provider, and cross-file diagnostic check"
```

---

### Task 3: Criterion Micro-benchmarking Suite

**Files:**
- Modify: `crates/htmx-service/Cargo.toml`
- Create: `crates/htmx-service/benches/navigation.rs`

**Interfaces:**
- Consumes: `htmx_service::{goto_definition, find_references, WorkspaceIdIndex}` and `htmx_vfs::Document`.
- Produces: Criterion benchmark binary measuring throughput, latency, positive hits, negative misses, and workspace indexing.

- [ ] **Step 1: Configure benchmark target in `crates/htmx-service/Cargo.toml`**

Add to `crates/htmx-service/Cargo.toml`:
```toml
[[bench]]
name = "navigation"
harness = false
```

- [ ] **Step 2: Implement `crates/htmx-service/benches/navigation.rs`**

Follow `golang-benchmark` and `m10-performance` rigor:
- Table-driven corpus generation:
  - Small: 100 lines (10 IDs)
  - Medium: 1,000 lines (100 IDs)
  - Large: 10,000 lines (1,000 IDs)
- Benchmarks:
  - `goto_definition_hit`: Cursor on existing target ID in 10k lines.
  - `goto_definition_miss`: Cursor on non-existent target ID in 10k lines.
  - `find_references_10k`: Discovering all references across 10k lines.
  - `workspace_index_build_100_files`: Crawling & indexing 100 template files.
- Use `criterion::black_box` on inputs and outputs.
- Set `Throughput::Bytes` and `Throughput::Elements(lines)` on benchmark groups.

- [ ] **Step 3: Run the micro-benchmarks**

```bash
cargo bench -p htmx-service --bench navigation -- --warm-up-time 1 --measurement-time 2
```
Verify all benchmarks run cleanly and meet performance budgets:
- `goto_definition`: `< 20 µs`
- `find_references`: `< 150 µs`
- `workspace_index_build`: `< 15 ms`

- [ ] **Step 4: Commit benchmark suite**

```bash
git add crates/htmx-service/
git commit -m "perf(service): add Criterion micro-benchmarks for definition lookup and reference search"
```

---

### Task 4: LSP Protocol Capabilities, Handlers, and Background Crawler

**Files:**
- Modify: `Cargo.toml` (workspace dependencies: `parking_lot`, `walkdir`)
- Modify: `crates/htmx-lsp/Cargo.toml`
- Modify: `crates/htmx-lsp/src/server.rs`
- Modify: `crates/htmx-lsp/src/handler.rs`
- Test: `crates/htmx-lsp/tests/lsp_navigation_e2e.rs`

**Interfaces:**
- Consumes: `htmx-service::{goto_definition, find_references, WorkspaceIdIndex}` and `lsp-types::{GotoDefinitionParams, ReferenceParams}`.
- Produces: Working LSP server responding to `textDocument/definition` and `textDocument/references`, plus background crawler indexing template files on `initialized`.

- [ ] **Step 1: Add dependencies `parking_lot` and `walkdir`**

Add `parking_lot = "0.12"` and `walkdir = "2.5"` to root `Cargo.toml` and `crates/htmx-lsp/Cargo.toml`.

- [ ] **Step 2: Write failing E2E LSP integration test**

Create `crates/htmx-lsp/tests/lsp_navigation_e2e.rs`:
```rust
use lsp_server::{Connection, Message, Request, Response};
use lsp_types::{
    request::{GotoDefinition, References},
    GotoDefinitionParams, GotoDefinitionResponse, Position, Range, ReferenceContext,
    ReferenceParams, TextDocumentIdentifier, TextDocumentPositionParams, Url,
};

#[test]
fn test_lsp_definition_and_references() {
    // Spin up server on memory channel connection, send initialize, open document,
    // request textDocument/definition and textDocument/references, and assert responses.
}
```

- [ ] **Step 3: Register capabilities in `crates/htmx-lsp/src/server.rs`**

- In `ServerCapabilities`:
  - `definition_provider: Some(lsp_types::GotoCapability::default())`
  - `references_provider: Some(lsp_types::OneOf::Left(true))`
- Store `workspace_index: Arc<parking_lot::RwLock<WorkspaceIdIndex>>` in `Server`.
- Store client capability `link_support: bool` extracted from `InitializeParams.capabilities.text_document.definition.link_support`.

- [ ] **Step 4: Implement background workspace crawler in `server.rs`**

On `initialized` notification:
- Read `root_uri` / `workspace_folders`.
- Spawn background thread walking directory using `walkdir`:
  - Filter extensions: `.html`, `.jinja`, `.jinja2`, `.j2`, `.blade.php`, `.templ`, `.erb`, `.hbs`, `.liquid`, `.twig`.
  - Skip directories: `.git`, `node_modules`, `target`, `dist`, `build`, `vendor`.
  - Skip files `> 5 MiB`.
  - Limit total files to 10,000.
  - Stage into local `WorkspaceIdIndex`, then write lock `workspace_index` in one batch.

- [ ] **Step 5: Implement request handlers in `crates/htmx-lsp/src/handler.rs`**

- `handle_definition`: calls `htmx_service::goto_definition` with `workspace_index.read()`.
- `handle_references`: calls `htmx_service::find_references` with `workspace_index.read()`.
- Update `didChange` / `didOpen` / `didClose` to update `workspace_index.write()`.
- Wire into `server.rs` request dispatcher.

- [ ] **Step 6: Run E2E test and verify**

```bash
cargo test -p htmx-analyzer --test lsp_navigation_e2e
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

- [ ] **Step 7: Commit LSP implementation**

```bash
git add Cargo.toml crates/htmx-lsp/
git commit -m "feat(lsp): wire definition and references LSP providers with background workspace indexing"
```

---

### Task 5: End-to-End Verification and Documentation

**Files:**
- Modify: `README.md`
- Modify: `crates/htmx-lsp/README.md`

- [ ] **Step 1: Update documentation with Target Navigation feature**

Document Go to Definition (`F12`), Find References (`Shift+F12`), and cross-template partial resolution in `README.md` and `crates/htmx-lsp/README.md`.

- [ ] **Step 2: Full verification check**

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo bench -p htmx-service --bench navigation -- --warm-up-time 1 --measurement-time 1
```

- [ ] **Step 3: Commit and finalize**

```bash
git add README.md crates/htmx-lsp/README.md
git commit -m "docs: document target navigation and workspace indexing capabilities"
```
