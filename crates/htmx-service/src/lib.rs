#![forbid(unsafe_code)]
//! Pure decoupled language intelligence service for HTMX 2.0.

pub mod code_action;
pub mod completion;
pub mod diagnostics;
pub mod hover;

pub use code_action::CodeActionProvider;
pub use completion::{CompletionProvider, MAX_ID_COMPLETIONS};
pub use diagnostics::DiagnosticEngine;
pub use hover::HoverProvider;

use htmx_parser::MAX_FILE_SIZE;
use htmx_vfs::Document;
use lsp_types::{CodeActionOrCommand, CompletionList, Diagnostic, Hover, Position, Range};

/// Pure, decoupled HTMX Language Service implementing LSP language features.
#[derive(Debug, Default, Clone)]
pub struct HtmxLanguageService;

impl HtmxLanguageService {
    pub fn new() -> Self {
        Self
    }

    /// Provide autocompletion items at the given document position.
    pub fn complete(&self, doc: &Document, position: Position) -> CompletionList {
        let text = doc.text();
        if text.len() > MAX_FILE_SIZE {
            return CompletionList {
                is_incomplete: false,
                items: Vec::new(),
            };
        }
        let offset = doc.position_to_offset(position).unwrap_or(0);
        CompletionProvider::complete(&text, offset)
    }

    /// Provide hover documentation at the given document position.
    pub fn hover(&self, doc: &Document, position: Position) -> Option<Hover> {
        let text = doc.text();
        if text.len() > MAX_FILE_SIZE {
            return None;
        }
        let offset = doc.position_to_offset(position)?;
        HoverProvider::hover(&text, offset)
    }

    /// Run static analysis and return diagnostics for the document.
    pub fn validate(&self, doc: &Document) -> Vec<Diagnostic> {
        DiagnosticEngine::validate(doc)
    }

    /// Provide quick-fix code actions for diagnostics in a range.
    pub fn code_actions(
        &self,
        doc: &Document,
        range: Range,
        diagnostics: &[Diagnostic],
    ) -> Vec<CodeActionOrCommand> {
        let text = doc.text();
        if text.len() > MAX_FILE_SIZE {
            return Vec::new();
        }
        CodeActionProvider::code_actions(doc, range, diagnostics)
    }
}
