#![forbid(unsafe_code)]
//! Pure decoupled language intelligence service for HTMX 2.0.

pub mod code_action;
pub mod completion;
pub mod diagnostics;
pub mod hover;

pub use code_action::CodeActionProvider;
pub use completion::CompletionProvider;
pub use diagnostics::DiagnosticEngine;
pub use hover::HoverProvider;

use htmx_vfs::Document;
use lsp_types::{CodeActionOrCommand, CompletionItem, Diagnostic, Hover, Position, Range};

/// Pure, decoupled HTMX Language Service implementing LSP language features.
#[derive(Debug, Default, Clone)]
pub struct HtmxLanguageService;

impl HtmxLanguageService {
    pub fn new() -> Self {
        Self
    }

    /// Provide autocompletion items at the given document position.
    pub fn complete(&self, doc: &Document, position: Position) -> Vec<CompletionItem> {
        let offset = doc.position_to_offset(position).unwrap_or(0);
        let text = doc.text();
        CompletionProvider::complete(&text, offset)
    }

    /// Provide hover documentation at the given document position.
    pub fn hover(&self, doc: &Document, position: Position) -> Option<Hover> {
        let offset = doc.position_to_offset(position)?;
        let text = doc.text();
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
        CodeActionProvider::code_actions(doc, range, diagnostics)
    }
}
