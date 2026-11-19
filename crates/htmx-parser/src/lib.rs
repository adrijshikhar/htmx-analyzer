#![forbid(unsafe_code)]
//! Resilient HTML attribute lexing and linear-time micro-syntax parsers for HTMX.

pub mod cursor_context;
pub mod lexer;
pub mod swap;
pub mod target;
pub mod trigger;

pub use cursor_context::{CursorContext, CursorPositionKind, CursorScanner};
pub use lexer::{
    document_ids, find_id_definitions, find_id_reference_at, find_id_references,
    is_id_referencing_attribute, lex, AttrValue, Attribute, IdDefinition, IdReference, Quote, Tag,
    MAX_FILE_SIZE,
};
pub use swap::{HxSwapParser, ScrollDirection, SwapModifier, SwapSpecification};
pub use target::{HxTarget, HxTargetParser};
pub use trigger::{HxTriggerParser, TriggerModifier, TriggerSpecification};
