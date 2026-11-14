# `htmx-analyzer`

A fast, production-grade HTMX 2.0 language server and static analyzer written in Rust.

[![CI](https://github.com/adrijshikhar/htmx-analyzer/actions/workflows/ci.yml/badge.svg)](https://github.com/adrijshikhar/htmx-analyzer/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

`htmx-analyzer` provides IDE-grade language features for [HTMX](https://htmx.org/) (including HTMX 2.0):
- **Intelligent Autocompletion**: Attributes (`hx-*`, `data-hx-*`), swap styles, trigger events, modifiers, CSS selector targets, and extensions (`sse`, `ws`, `preload`, `response-targets`).
- **Context-Aware Hover Documentation**: Full Markdown documentation, syntax guides, and examples directly from the HTMX specification.
- **Static Analysis & Diagnostics**: Real-time linting for deprecated HTMX 1.x patterns (`HTMX001`), invalid swap styles (`HTMX002`), malformed trigger syntax (`HTMX003`), and broken target selector IDs (`HTMX004`).
- **Quick-Fix Code Actions**: One-click refactorings (e.g. migrating `hx-ws` to HTMX 2.0 `hx-ext="ws"`).
- **Fast & Resilient**: Incremental synchronization with Tree-Sitter parsing and fallback micro-scanners that never hang or crash on incomplete markup.

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
You can configure `htmx-analyzer` as an external language server in your Zed `settings.json`:
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
