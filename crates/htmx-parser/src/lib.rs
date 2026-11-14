#![forbid(unsafe_code)]
//! Resilient Tree-Sitter HTML parsing and linear-time micro-syntax parsers for HTMX.

pub mod cursor_context;
pub mod html_document;
pub mod swap;
pub mod target;
pub mod trigger;

pub use cursor_context::{CursorContext, CursorPositionKind, CursorScanner};
pub use html_document::{HtmlDocument, MAX_DEPTH, MAX_FILE_SIZE};
pub use swap::{HxSwapParser, ScrollDirection, SwapModifier, SwapSpecification};
pub use target::{HxTarget, HxTargetParser};
pub use trigger::{HxTriggerParser, TriggerModifier, TriggerSpecification};
