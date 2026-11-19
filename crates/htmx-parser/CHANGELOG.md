# Changelog

## [0.3.0](https://github.com/adrijshikhar/htmx-analyzer/compare/htmx-parser-v0.2.0...htmx-parser-v0.3.0) (2026-11-19)


### Features

* initial implementation of htmx-analyzer LSP workspace ([98e9044](https://github.com/adrijshikhar/htmx-analyzer/commit/98e904435069e96187a14d114822a8565c67e4b6))
* **parser:** add IdDefinition and find_id_reference_at token extractors ([79baf16](https://github.com/adrijshikhar/htmx-analyzer/commit/79baf168e5e24dd749a9115106c383e10e2b1a0c))
* **parser:** add linear attribute lexer ([6549a4a](https://github.com/adrijshikhar/htmx-analyzer/commit/6549a4a08e7374c4e970dfa8a17d15fe3900766e))
* **service:** implement WorkspaceIdIndex, navigation provider, and cross-file diagnostic check ([a0bf00f](https://github.com/adrijshikhar/htmx-analyzer/commit/a0bf00fecf28506838412e616a2a92094ebb7fe9))


### Bug Fixes

* address all OpenQodex findings and harden workflows ([b914547](https://github.com/adrijshikhar/htmx-analyzer/commit/b914547468aa212e4a4cf2500b0a691c2709b28f))
* configure release-please cargo-workspace and linked-versions plugins ([1a32823](https://github.com/adrijshikhar/htmx-analyzer/commit/1a3282343a9da8ee1bd43160f141d48fad2cd032))
* exact hx-on deprecation check, unquoted value cursor boundary, and template tolerance ([c3394f1](https://github.com/adrijshikhar/htmx-analyzer/commit/c3394f1e97f2cfe5e1a297ba17bc3153300136e7))
* resolve character boundary edge cases, trigger groups, and release permissions ([15a5b1b](https://github.com/adrijshikhar/htmx-analyzer/commit/15a5b1be2bde4036afdb604c5ec8ebf5d46aa9b2))
* resolve OpenQodex findings across parser, hover, completion, and release workflow ([efc3e30](https://github.com/adrijshikhar/htmx-analyzer/commit/efc3e3064b01258e893a01ded3fe113b76d62a72))
* tag scan quote context, duration number validator, modifiers and linear AST cursor ([f374f7c](https://github.com/adrijshikhar/htmx-analyzer/commit/f374f7c25f6d664fbf7ea690d0920b8e49d134fe))


### Performance Improvements

* validate with the linear lexer and drop tree-sitter ([064edf5](https://github.com/adrijshikhar/htmx-analyzer/commit/064edf52e5537ce098354691815e2bf43b2a3277))
