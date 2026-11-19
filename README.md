# `htmx-analyzer`

A fast, production-grade HTMX 2.0 language server and static analyzer written in Rust.

[![CI](https://github.com/adrijshikhar/htmx-analyzer/actions/workflows/ci.yml/badge.svg)](https://github.com/adrijshikhar/htmx-analyzer/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

`htmx-analyzer` provides IDE-grade language features for [HTMX](https://htmx.org/) (including HTMX 2.0):
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

### From Source
Install directly using Cargo:
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

`htmx-analyzer` is engineered for extreme scale with a linear attribute lexer (zero Tree-Sitter dependency), incremental rope-based VFS, and non-blocking JSON-RPC transport. 

Automated stress benchmarks on a single file containing dense, repeated HTMX components (triggers, targets, swaps, and IDs):

| Metric | 10,000 Lines (0.61 MB) | 100,000 Lines (4.55 MB) |
|---|---|---|
| **Document Open (`didOpen` → Diagnostics)** | **94.7 ms** | **2,192.3 ms** |
| **Analysis Throughput** | **105,559 lines/sec** (6.41 MB/s) | **45,615 lines/sec** (2.07 MB/s) |
| **Diagnostics Evaluated** | Full document pass | **49,998 diagnostics emitted** |
| **Keystroke Latency (`didChange` p50)** | **178.2 ms** | **626.2 ms** |
| **Hover Latency (p50)** | **0.2 ms** | **0.5 ms** |
| **Autocompletion Latency (p50)** | **1.8 ms** | **320.9 ms** (25,000 targets indexed) |
| **Peak Memory (RSS)** | **27.9 MB** | **362.2 MB** |

> [!TIP]
> Documents exceeding the 5 MiB safety cap are automatically protected against unbounded AST memory consumption while maintaining responsive sub-millisecond hover and completions.

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
