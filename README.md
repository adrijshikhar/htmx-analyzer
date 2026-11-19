# `htmx-analyzer`

A fast, production-grade HTMX 2.0 language server and static analyzer written in Rust.

[![CI](https://github.com/adrijshikhar/htmx-analyzer/actions/workflows/ci.yml/badge.svg)](https://github.com/adrijshikhar/htmx-analyzer/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/htmx-analyzer.svg)](https://crates.io/crates/htmx-analyzer)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

`htmx-analyzer` provides IDE-grade language features for [HTMX](https://htmx.org/) (including HTMX 2.0):
- **Target Navigation & Cross-File Jump (`F12` / `gd`)**: Jump directly from target selectors (`hx-target="#id"`, `hx-indicator="#id"`, `hx-swap-oob="true:#id"`, `hx-sync="#id:event"`) to the declaring element `<tag id="id">`. Supports LSP `LocationLink` for high-precision cursor targeting.
- **Find References (`Shift+F12`)**: Discover all HTMX attributes in the document and workspace referencing any given element `<tag id="id">`.
- **Cross-Template Workspace Indexing**: Fast in-memory symbol index resolves target IDs across partials and layout templates on startup, eliminating false-positive `HTMX004` diagnostics.
- **Intelligent Autocompletion**: 34 active core attributes (`hx-*`, `data-hx-*`), swap styles, trigger events, modifiers, CSS selector targets, and extensions (`sse`, `ws`, `preload`, `response-targets`).
- **Context-Aware Hover Documentation**: Full Markdown documentation, syntax guides, and examples directly from the HTMX specification.
- **Static Analysis & Diagnostics**: Real-time linting for deprecated HTMX 1.x patterns (`HTMX001`), invalid swap styles and durations (`HTMX002`), malformed trigger syntax and polling intervals (`HTMX003`), and broken target selector IDs (`HTMX004`).
- **Quick-Fix Code Actions**: One-click refactorings (e.g. migrating `hx-ws` or `hx-sse` to HTMX 2.0 `hx-ext="ws"` / `hx-ext="sse"`).
- **Fast & Resilient**: Linear attribute lexer with zero external C dependencies, non-blocking stdio pump thread, and resilient tokenization across templates.

> [!NOTE]
> **Minimum Version Requirement:** Template languages like Astro, Svelte, and Templ require `htmx-analyzer` ≥ `0.2.0`. Note that if you have an older version of `htmx-analyzer` installed on your `PATH`, your editor will typically resolve that binary over any version managed by an editor extension (such as `zed-htmx`).

---

## Supported Templates

`htmx-analyzer` parses attributes using a resilient linear lexer that skips opaque template delimiters and raw text blocks (`<script>`, `<style>`). It supports:
- HTML
- Jinja2
- Django
- PHP
- HTML+ERB
- Blade
- Vue.js
- Astro
- Svelte
- Templ

---

## Installation

### Prebuilt Binaries
Download prebuilt release binaries for your platform from [GitHub Releases](https://github.com/adrijshikhar/htmx-analyzer/releases).

Supported platforms:
- Linux x86_64 (`musl`): `htmx-analyzer-linux-x64`
- Linux AArch64 (`musl`): `htmx-analyzer-linux-arm64`
- macOS Apple Silicon: `htmx-analyzer-darwin-arm64`
- macOS Intel: `htmx-analyzer-darwin-x64`
- Windows x86_64: `htmx-analyzer-windows-x64.exe`
- Windows AArch64: `htmx-analyzer-windows-arm64.exe`

### From crates.io (Cargo)
Install directly using Cargo:
```bash
cargo install htmx-analyzer
```

### From Git Source
```bash
cargo install --git https://github.com/adrijshikhar/htmx-analyzer htmx-analyzer
```

---

## Verifying Downloads

All release binaries include cryptographic build provenance attestations generated via GitHub Actions. You can verify the authenticity and provenance of any downloaded binary using the GitHub CLI:

```bash
gh attestation verify <asset> \
  -R adrijshikhar/htmx-analyzer \
  --signer-workflow adrijshikhar/htmx-analyzer/.github/workflows/release.yml \
  --source-ref refs/tags/<tag>
```

---

## Configuration

You can configure diagnostic severities via LSP `initializationOptions` or dynamic `workspace/didChangeConfiguration` settings.

### Severity Levels
- `error`
- `warning`
- `information`
- `hint` (default for `HTMX004`)
- `off`

### JSON Schema
```json
{
  "diagnostics": {
    "severity": {
      "HTMX001": "warning",
      "HTMX002": "error",
      "HTMX003": "error",
      "HTMX004": "hint"
    }
  }
}
```

### Editor Setup

#### Zed
Install the official Zed extension [**`zed-htmx`**](https://github.com/adrijshikhar/zed-htmx) for automatic binary management.

To configure diagnostic severities in Zed `settings.json`:
```json
{
  "lsp": {
    "htmx-analyzer": {
      "settings": {
        "diagnostics": {
          "severity": {
            "HTMX004": "off"
          }
        }
      }
    }
  }
}
```

#### Neovim
```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

if not configs.htmx_analyzer then
  configs.htmx_analyzer = {
    default_config = {
      cmd = { 'htmx-analyzer' },
      filetypes = { 'html', 'htm', 'jinja', 'template', 'astro', 'svelte', 'templ', 'blade', 'php' },
      root_dir = lspconfig.util.root_pattern('.git', '.'),
      init_options = {
        diagnostics = {
          severity = {
            HTMX004 = "hint"
          }
        }
      }
    },
  }
end
lspconfig.htmx_analyzer.setup{}
```

#### VS Code
Use any generic LSP client extension pointing to `htmx-analyzer`.

---

## Performance & Stress Testing

`htmx-analyzer` is engineered in pure Rust for extreme scale, zero-dependency deployment, and rock-solid memory stability. Powered by an allocation-conscious linear attribute lexer, incremental rope-based VFS, and non-blocking JSON-RPC transport, it delivers sub-millisecond editor responsiveness across workloads ranging from single-line templates to 100,000-line monolithic files.

Automated benchmark results below were measured on Apple Silicon with release builds via the integrated stress test harness.

### 1. Architectural Head-to-Head Comparison

| Architectural Dimension | `htmx-analyzer` (Pure Rust) | Typical Language Server (Node.js / Python) |
| :--- | :--- | :--- |
| **Cold Startup Latency** | **`2.6 ms`** (instant initialization) | `300 – 1,500 ms` (runtime startup & module resolution) |
| **Base Idle Memory (RSS)** | **`2.9 MB`** | `45 – 120 MB` (V8 / Python runtime baseline) |
| **Binary & Distribution Size** | **`~2.4 MB`** (single standalone native binary) | `50 – 200 MB` (runtime + `node_modules` / virtualenv) |
| **External Dependencies** | **`Zero`** (no Node, Python, npm, or Tree-Sitter) | Requires runtime interpreter + hundreds of package dependencies |
| **Safety Cap & Memory Protection** | **`5 MiB safety cap`** & linear $O(N)$ parser | Uncapped AST recursion & garbage collection pauses |
| **Template Dialect Support** | **10 dialects** (HTML, Jinja, Django, Blade, Astro, Svelte, Templ, PHP, ERB, Vue) | Often limited to HTML or requires separate external plugins |

### 2. Massive Scale Performance (Single File)

Stress testing across single files from 1,000 to 100,000 lines containing dense, repeated HTMX components (triggers, targets, swaps, and IDs):

| File Size | didOpen Latency | Ingest Throughput | didChange (p50) | Hover (p50) | Completion (p50) | Peak RSS |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **1,000 lines** (0.05 MB) | `0.5 ms` | `1,881,910 lines/s` (91 MB/s) | `0.35 ms` | `0.098 ms` | `0.100 ms` | `4.6 MB` |
| **10,000 lines** (0.50 MB) | `3.4 ms` | `2,914,177 lines/s` (145 MB/s) | `3.00 ms` | `0.648 ms` | `0.653 ms` | `10.1 MB` |
| **50,000 lines** (2.53 MB) | `18.0 ms` | `2,780,255 lines/s` (141 MB/s) | `15.55 ms` | `3.346 ms` | `3.351 ms` | `37.2 MB` |
| **100,000 lines** (5.11 MB) | `14.6 ms` | **`6,856,046 lines/s`** (350 MB/s) | `13.86 ms` | `0.481 ms` | `0.489 ms` | `340.4 MB` |

> [!TIP]
> Documents exceeding the 5 MiB safety cap automatically utilize linear streaming scans to prevent unbounded AST memory expansion while maintaining responsive sub-millisecond hover and completion latencies.

### 3. Burst Typing & Backpressure

Simulating rapid keystroke bursts without editor debouncing to test transport queues and non-blocking execution:

| Metric | Measurement | Target Requirement | Status |
| :--- | :--- | :--- | :--- |
| **Burst Keystroke Volume** | `100 keystrokes` in `0.69 ms` | — | — |
| **Ingest Throughput** | **`144,153 keystrokes/s`** | `> 500 keystrokes/s` | **PASS** |
| **Post-Burst Completion Latency** | **`0.260 ms`** | `< 5.0 ms` | **PASS** |
| **Peak Memory (RSS)** | **`3.66 MB`** | `< 50.0 MB` | **PASS** |

Zero dropped messages, zero transport backpressure lockups, and instantaneous autocompletion immediately following high-frequency typing bursts.

### 4. Multi-File Workspace Indexing

Evaluating project-wide symbol resolution across a distributed multi-file template workspace:

| Metric | Measurement | Target Requirement | Status |
| :--- | :--- | :--- | :--- |
| **Workspace Scale** | `500` files (`25,000` total lines) | 500 files | **PASS** |
| **Total Indexing Time** | **`22.2 ms`** | `< 1,000 ms` | **PASS** |
| **Ingest Throughput** | **`22,519 files/s`** | `> 500 files/s` | **PASS** |
| **Cross-File Target Completion (p50)** | **`0.040 ms`** | `< 2.0 ms` | **PASS** |
| **Cross-File Target Completion (p95)** | **`0.062 ms`** | `< 5.0 ms` | **PASS** |
| **Peak Workspace Memory (RSS)** | **`6.2 MB`** | `< 100.0 MB` | **PASS** |

### 5. Multi-Dialect Parity Matrix

Performance consistency across modern and classic template syntaxes (10,000 lines each):

| Dialect | Parse + Diag Latency | Throughput | Hover (p50) | Completion (p50) | Peak RSS |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **HTML** | `3.3 ms` | `2,993,377 lines/s` | `0.650 ms` | `0.652 ms` | `8.7 MB` |
| **Jinja** | `2.7 ms` | `3,694,751 lines/s` | `0.619 ms` | `0.617 ms` | `7.5 MB` |
| **Blade** | `2.6 ms` | `3,806,321 lines/s` | `0.588 ms` | `0.587 ms` | `7.2 MB` |
| **Astro** | `2.0 ms` | `5,015,777 lines/s` | `0.504 ms` | `0.503 ms` | `7.8 MB` |
| **Svelte** | `1.9 ms` | **`5,305,040 lines/s`** | `0.517 ms` | `0.518 ms` | `6.7 MB` |
| **Templ** | `2.7 ms` | `3,736,048 lines/s` | `0.693 ms` | `0.696 ms` | `8.8 MB` |

### 6. Endurance & Zero-Leak Stability

Continuous editing session verifying memory allocator stability under continuous re-tokenization and rope-based text mutations:

| Metric | Measurement | Target Requirement | Status |
| :--- | :--- | :--- | :--- |
| **Continuous Edits (`didChange`)** | `2,000` events in `0.16s` | 2,000 edits | **PASS** |
| **Edit Processing Throughput** | **`12,142.9 edits/s`** | `> 500 edits/s` | **PASS** |
| **Baseline Starting RSS** | `4.66 MB` | — | — |
| **Final RSS (After 2,000 Edits)** | `5.20 MB` | — | — |
| **Memory Drift ($\Delta\text{RSS}$)** | **`0.55 MB`** | `≤ 1.0 MB` | **PASS (Zero Memory Leak)** |

### Reproducing Benchmarks

Run the complete 6-dimension benchmark suite locally:

```bash
# Build the optimized release binary
cargo build --release

# Execute the automated benchmark runner
python3 scripts/run_benchmarks.py --binary target/release/htmx-analyzer --suite all
```

---

## Architecture

`htmx-analyzer` is engineered as a modular multi-crate Rust workspace patterned after `rust-analyzer` and `vscode-html-languageservice`:

```text
htmx-analyzer/
├── crates/
│   ├── htmx-schema/       # Declarative attribute definitions, rules & docs
│   ├── htmx-parser/       # Linear attribute lexer & mini-parsers (no Tree-Sitter)
│   ├── htmx-vfs/          # Virtual file system & rope-based text document management
│   ├── htmx-service/      # Pure Language Service (completions, hover, diagnostics)
│   └── htmx-lsp/          # Library & CLI binary with JSON-RPC stdio transport
```

---

## License
Licensed under Apache-2.0.
