# Changelog

## [0.3.0](https://github.com/adrijshikhar/htmx-analyzer/compare/htmx-analyzer-v0.2.0...htmx-analyzer-v0.3.0) (2026-11-19)


### Features

* initial implementation of htmx-analyzer LSP workspace ([98e9044](https://github.com/adrijshikhar/htmx-analyzer/commit/98e904435069e96187a14d114822a8565c67e4b6))
* **lsp:** wire definition and references LSP providers with background workspace indexing ([67bd41e](https://github.com/adrijshikhar/htmx-analyzer/commit/67bd41effcd7f3dcb2e266ae5690fbb968db1ae7))


### Bug Fixes

* address all OpenQodex findings and harden workflows ([b914547](https://github.com/adrijshikhar/htmx-analyzer/commit/b914547468aa212e4a4cf2500b0a691c2709b28f))
* configure release-please cargo-workspace and linked-versions plugins ([1a32823](https://github.com/adrijshikhar/htmx-analyzer/commit/1a3282343a9da8ee1bd43160f141d48fad2cd032))
* **lsp:** drain buffered and incoming messages to coalesce rapid didChange bursts ([76699c8](https://github.com/adrijshikhar/htmx-analyzer/commit/76699c88cea2e8f67d2b6d0bb00980dadf978daf))
* **lsp:** spec-compliant shutdown and per-request panic isolation ([fd1aa1a](https://github.com/adrijshikhar/htmx-analyzer/commit/fd1aa1aef6dffc761d17664f8b9df3984572ca51))
* resolve OpenQodex findings across parser, hover, completion, and release workflow ([efc3e30](https://github.com/adrijshikhar/htmx-analyzer/commit/efc3e3064b01258e893a01ded3fe113b76d62a72))


### Performance Improvements

* **lsp:** batch queued changes before validating; wire configuration ([3f65147](https://github.com/adrijshikhar/htmx-analyzer/commit/3f65147fe096c7807825370f1144d438725a9336))
