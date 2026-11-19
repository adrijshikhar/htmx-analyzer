# Changelog

## [0.3.0](https://github.com/adrijshikhar/htmx-analyzer/compare/htmx-service-v0.2.0...htmx-service-v0.3.0) (2026-11-19)


### Features

* configurable diagnostic severity, HTMX004 defaults to hint ([79703c6](https://github.com/adrijshikhar/htmx-analyzer/commit/79703c63731d31b6751d61fb38db11368288ac82))
* initial implementation of htmx-analyzer LSP workspace ([98e9044](https://github.com/adrijshikhar/htmx-analyzer/commit/98e904435069e96187a14d114822a8565c67e4b6))
* **lsp:** wire definition and references LSP providers with background workspace indexing ([67bd41e](https://github.com/adrijshikhar/htmx-analyzer/commit/67bd41effcd7f3dcb2e266ae5690fbb968db1ae7))
* **service:** implement WorkspaceIdIndex, navigation provider, and cross-file diagnostic check ([a0bf00f](https://github.com/adrijshikhar/htmx-analyzer/commit/a0bf00fecf28506838412e616a2a92094ebb7fe9))


### Bug Fixes

* address all OpenQodex findings and harden workflows ([b914547](https://github.com/adrijshikhar/htmx-analyzer/commit/b914547468aa212e4a4cf2500b0a691c2709b28f))
* **ci:** update upload-artifact SHA, remove unused manifest fields, and refine hover ext parsing ([d1c9abe](https://github.com/adrijshikhar/htmx-analyzer/commit/d1c9abe1acbd62cf31d3c2b287baed6739893212))
* **clippy:** allow question_mark on quick fix migration block ([ae09ba4](https://github.com/adrijshikhar/htmx-analyzer/commit/ae09ba426c5e6c036cbb9e63c4d238e228f68e15))
* configure release-please cargo-workspace and linked-versions plugins ([1a32823](https://github.com/adrijshikhar/htmx-analyzer/commit/1a3282343a9da8ee1bd43160f141d48fad2cd032))
* exact hx-on deprecation check, unquoted value cursor boundary, and template tolerance ([c3394f1](https://github.com/adrijshikhar/htmx-analyzer/commit/c3394f1e97f2cfe5e1a297ba17bc3153300136e7))
* match attributes case-insensitively and validate swap/poll durations ([2414dc0](https://github.com/adrijshikhar/htmx-analyzer/commit/2414dc0d95f6ae4083edd9189b58d2901eff69d7))
* merge into existing hx-ext and escape values in the hx-ws/hx-sse quick fix ([0ff8cc1](https://github.com/adrijshikhar/htmx-analyzer/commit/0ff8cc178bfb17c3172a4e876ee9bd5f6c54ed56))
* replace deprecated criterion::black_box with std::hint::black_box ([f4f7a00](https://github.com/adrijshikhar/htmx-analyzer/commit/f4f7a006642e4be5414db03fe2b1ebea291b3651))
* resolve character boundary edge cases, trigger groups, and release permissions ([15a5b1b](https://github.com/adrijshikhar/htmx-analyzer/commit/15a5b1be2bde4036afdb604c5ec8ebf5d46aa9b2))
* resolve OpenQodex findings across parser, hover, completion, and release workflow ([efc3e30](https://github.com/adrijshikhar/htmx-analyzer/commit/efc3e3064b01258e893a01ded3fe113b76d62a72))
* tag scan quote context, duration number validator, modifiers and linear AST cursor ([f374f7c](https://github.com/adrijshikhar/htmx-analyzer/commit/f374f7c25f6d664fbf7ea690d0920b8e49d134fe))


### Performance Improvements

* **service:** add Criterion micro-benchmarks for definition lookup and reference search ([b50fa4b](https://github.com/adrijshikhar/htmx-analyzer/commit/b50fa4b0f7c27203aef7ce574c82edaa150294a8))
* validate with the linear lexer and drop tree-sitter ([064edf5](https://github.com/adrijshikhar/htmx-analyzer/commit/064edf52e5537ce098354691815e2bf43b2a3277))
