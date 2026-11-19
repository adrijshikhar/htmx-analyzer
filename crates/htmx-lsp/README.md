# `htmx-analyzer`

A fast, production-grade HTMX 2.0 language server and static analyzer written in Rust.

[![CI](https://github.com/adrijshikhar/htmx-analyzer/actions/workflows/ci.yml/badge.svg)](https://github.com/adrijshikhar/htmx-analyzer/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/htmx-analyzer.svg)](https://crates.io/crates/htmx-analyzer)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

`htmx-analyzer` provides IDE-grade language features for [HTMX](https://htmx.org/) (including HTMX 2.0):
- **Target Navigation & Cross-File Jump (`F12` / `gd`)**: Jump directly from target selectors (`hx-target="#id"`, `hx-indicator="#id"`, `hx-swap-oob="true:#id"`, `hx-sync="#id:event"`) to the declaring element `<tag id="id">`. Supports LSP `LocationLink` for high-precision cursor targeting.
- **Find References (`Shift+F12`)**: Discover all HTMX attributes in the document and workspace referencing any given element `<tag id="id">`.
- **Cross-Template Workspace Indexing**: Fast in-memory symbol index resolves target IDs across partials and layout templates on startup, eliminating false-positive `HTMX004` diagnostics.
- **Intelligent Autocompletion**: Attributes (`hx-*`, `data-hx-*`), swap styles, trigger events, modifiers, CSS selector targets, and extensions (`sse`, `ws`, `preload`, `response-targets`).
- **Context-Aware Hover Documentation**: Full Markdown documentation, syntax guides, and examples directly from the HTMX specification.
- **Static Analysis & Diagnostics**: Real-time linting for deprecated HTMX 1.x patterns (`HTMX001`), invalid swap styles (`HTMX002`), malformed trigger syntax (`HTMX003`), and broken target selector IDs (`HTMX004`).
- **Quick-Fix Code Actions**: One-click refactorings (e.g. migrating `hx-ws` to HTMX 2.0 `hx-ext="ws"`).
- **Fast & Resilient**: Incremental synchronization with resilient tokenization and fallback micro-scanners that never hang or crash on incomplete markup.

## Architecture

`htmx-analyzer` is engineered as a modular multi-crate Rust workspace patterned after `rust-analyzer` and `vscode-html-languageservice`:

```text
htmx-analyzer/
├── crates/
│   ├── htmx-schema/       # Declarative attribute definitions, rules & docs
│   ├── htmx-parser/       # Tree-Sitter & resilient attribute lexer/scanner
│   ├── htmx-vfs/          # Virtual file system & rope-based text document management
│   ├── htmx-service/      # Pure Language Service (completions, hover, diagnostics)
│   └── htmx-lsp/          # CLI binary & JSON-RPC LSP server entrypoint
```

## Editor Setup

### Zed
Install the official Zed extension [**`zed-htmx`**](https://github.com/adrijshikhar/zed-htmx) for zero-configuration setup and automatic binary management.

Alternatively, you can configure `htmx-analyzer` manually as an external language server in your Zed `settings.json`:
```json
{
  "lsp": {
    "htmx-analyzer": {
      "binary": {
        "path": "htmx-analyzer"
      }
    }
  },
  "languages": {
    "HTML": {
      "language_servers": ["htmx-analyzer", "..."]
    }
  }
}
```

### Neovim
```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

if not configs.htmx_analyzer then
  configs.htmx_analyzer = {
    default_config = {
      cmd = { 'htmx-analyzer' },
      filetypes = { 'html', 'htm', 'jinja', 'template' },
      root_dir = lspconfig.util.root_pattern('.git', '.'),
    },
  }
end
lspconfig.htmx_analyzer.setup{}
```

### VS Code
Use any generic LSP client extension pointing to `htmx-analyzer`.

## License
Licensed under Apache-2.0.
