#![forbid(unsafe_code)]
//! Pure decoupled language intelligence service for HTMX 2.0.

pub mod code_action;
pub mod completion;
pub mod config;
pub mod diagnostics;
pub mod hover;
pub mod navigation;

pub use code_action::CodeActionProvider;
pub use completion::{CompletionProvider, MAX_ID_COMPLETIONS};
pub use config::{Config, SeverityLevel};
pub use diagnostics::DiagnosticEngine;
pub use hover::HoverProvider;
pub use navigation::{find_references, goto_definition, IdLocation, WorkspaceIdIndex};

use htmx_parser::MAX_FILE_SIZE;
use htmx_vfs::Document;
use lsp_types::{
    CodeActionOrCommand, CompletionList, Diagnostic, GotoDefinitionResponse, Hover, Location,
    Position, Range, Uri,
};

/// Pure, decoupled HTMX Language Service implementing LSP language features.
#[derive(Debug, Default, Clone)]
pub struct HtmxLanguageService {
    config: Config,
}

impl HtmxLanguageService {
    pub fn new() -> Self {
        Self {
            config: Config::default(),
        }
    }

    pub fn with_config(config: Config) -> Self {
        Self { config }
    }

    pub fn set_config(&mut self, config: Config) {
        self.config = config;
    }

    pub fn config(&self) -> &Config {
        &self.config
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
        self.validate_with_index(doc, None)
    }

    /// Run static analysis with cross-file workspace index resolution.
    pub fn validate_with_index(
        &self,
        doc: &Document,
        index: Option<&WorkspaceIdIndex>,
    ) -> Vec<Diagnostic> {
        DiagnosticEngine::validate_with_index(doc, &self.config, index)
    }

    /// Navigate to the definition of the ID under the cursor.
    pub fn goto_definition(
        &self,
        uri: &Uri,
        position: Position,
        doc: &Document,
        index: &WorkspaceIdIndex,
        link_support: bool,
    ) -> Option<GotoDefinitionResponse> {
        let text = doc.text();
        if text.len() > MAX_FILE_SIZE {
            return None;
        }
        goto_definition(uri, position, doc, index, link_support)
    }

    /// Find all references to the ID under the cursor.
    pub fn find_references(
        &self,
        uri: &Uri,
        position: Position,
        doc: &Document,
        open_docs: &[&Document],
        index: &WorkspaceIdIndex,
        include_declaration: bool,
    ) -> Vec<Location> {
        let text = doc.text();
        if text.len() > MAX_FILE_SIZE {
            return Vec::new();
        }
        find_references(uri, position, doc, open_docs, index, include_declaration)
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
