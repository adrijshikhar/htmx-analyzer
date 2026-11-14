use htmx_vfs::Document;
use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, Diagnostic, NumberOrString, Range, TextEdit,
    WorkspaceEdit,
};
use std::collections::HashMap;

pub struct CodeActionProvider;

impl CodeActionProvider {
    pub fn code_actions(
        doc: &Document,
        _range: Range,
        diagnostics: &[Diagnostic],
    ) -> Vec<CodeActionOrCommand> {
        let mut actions = Vec::new();

        for diag in diagnostics {
            if let Some(NumberOrString::String(ref code)) = diag.code {
                if code == "HTMX001" {
                    // Deprecated hx-ws or hx-sse quick-fix
                    if let Some(action) = Self::fix_deprecated_attribute(doc, diag) {
                        actions.push(CodeActionOrCommand::CodeAction(action));
                    }
                }
            }
        }

        actions
    }

    #[allow(clippy::mutable_key_type)]
    fn fix_deprecated_attribute(doc: &Document, diag: &Diagnostic) -> Option<CodeAction> {
        let start_offset = doc.position_to_offset(diag.range.start)?;
        let end_offset = doc.position_to_offset(diag.range.end)?;
        let text = doc.text();

        if start_offset > end_offset || end_offset > text.len() {
            return None;
        }

        let attr_name = &text[start_offset..end_offset];
        if attr_name == "hx-ws" {
            let mut changes = HashMap::new();
            changes.insert(
                doc.uri.clone(),
                vec![TextEdit {
                    range: diag.range,
                    new_text: "hx-ext=\"ws\" ws-connect".to_string(),
                }],
            );

            return Some(CodeAction {
                title: "Migrate hx-ws to HTMX 2.0 extension (hx-ext=\"ws\")".to_string(),
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: Some(vec![diag.clone()]),
                edit: Some(WorkspaceEdit {
                    changes: Some(changes),
                    ..Default::default()
                }),
                is_preferred: Some(true),
                ..Default::default()
            });
        }

        None
    }
}
