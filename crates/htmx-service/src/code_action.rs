use htmx_schema::HtmxAttributeDefinition;
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
        let norm_name = HtmxAttributeDefinition::normalize_name(attr_name);

        if norm_name != "hx-ws" && norm_name != "hx-sse" {
            return None;
        }

        // Scan full attribute including value
        let rest = &text[end_offset..];
        let mut full_end_offset = end_offset;
        let mut attr_val: Option<&str> = None;

        let trimmed_rest = rest.trim_start();
        if let Some(stripped) = trimmed_rest.strip_prefix('=') {
            let after_eq = stripped.trim_start();
            if after_eq.starts_with('"') || after_eq.starts_with('\'') {
                let quote = after_eq.chars().next()?;
                let inner = &after_eq[1..];
                let close_idx = inner.find(quote)?;
                attr_val = Some(&inner[..close_idx]);
                let prefix_len = text.len() - after_eq.len() + 1;
                full_end_offset = prefix_len + close_idx + 1;
            } else {
                let val_len = after_eq
                    .find(|c: char| c.is_whitespace() || c == '>')
                    .unwrap_or(after_eq.len());
                let val = &after_eq[..val_len];
                if !val.is_empty() {
                    attr_val = Some(val);
                    let prefix_len = text.len() - after_eq.len();
                    full_end_offset = prefix_len + val_len;
                }
            }
        }

        let new_text = if norm_name == "hx-ws" {
            if let Some(val) = attr_val {
                if let Some(url) = val.strip_prefix("connect:") {
                    format!("hx-ext=\"ws\" ws-connect=\"{}\"", url)
                } else if val == "send" {
                    "ws-send".to_string()
                } else {
                    format!("hx-ext=\"ws\" ws-connect=\"{}\"", val)
                }
            } else {
                "hx-ext=\"ws\"".to_string()
            }
        } else if let Some(val) = attr_val {
            if let Some(url) = val.strip_prefix("connect:") {
                format!("hx-ext=\"sse\" sse-connect=\"{}\"", url)
            } else if let Some(evt) = val.strip_prefix("swap:") {
                format!("sse-swap=\"{}\"", evt)
            } else {
                format!("hx-ext=\"sse\" sse-connect=\"{}\"", val)
            }
        } else {
            "hx-ext=\"sse\"".to_string()
        };

        let start_pos = doc.offset_to_position(start_offset)?;
        let end_pos = doc.offset_to_position(full_end_offset)?;
        let full_range = Range {
            start: start_pos,
            end: end_pos,
        };

        let mut changes = HashMap::new();
        changes.insert(
            doc.uri.clone(),
            vec![TextEdit {
                range: full_range,
                new_text,
            }],
        );

        let title = format!(
            "Migrate {} to HTMX 2.0 extension (hx-ext=\"{}\")",
            norm_name,
            if norm_name == "hx-ws" { "ws" } else { "sse" }
        );

        Some(CodeAction {
            title,
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: Some(vec![diag.clone()]),
            edit: Some(WorkspaceEdit {
                changes: Some(changes),
                ..Default::default()
            }),
            is_preferred: Some(true),
            ..Default::default()
        })
    }
}
