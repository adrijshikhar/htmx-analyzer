use htmx_parser::{lex, Attribute, Tag};
use htmx_schema::canonical_name;
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

    #[allow(clippy::mutable_key_type, clippy::question_mark)]
    fn fix_deprecated_attribute(doc: &Document, diag: &Diagnostic) -> Option<CodeAction> {
        let start_offset = doc.position_to_offset(diag.range.start)?;
        let text = doc.text();

        let tags = lex(&text);
        let mut target: Option<(&Tag, &Attribute)> = None;
        'outer: for tag in &tags {
            for attr in &tag.attributes {
                if attr.name_range.start == start_offset {
                    target = Some((tag, attr));
                    break 'outer;
                }
            }
        }

        let (tag, attr) = target?;
        let norm_name = canonical_name(attr.name);
        if norm_name != "hx-ws" && norm_name != "hx-sse" {
            return None;
        }

        // No action when deprecated attribute's value is dynamic
        if let Some(val) = &attr.value {
            if val.is_dynamic() {
                return None;
            }
        }

        // Or when tag's hx-ext/data-hx-ext value is dynamic
        let existing_ext_attr = tag
            .attributes
            .iter()
            .find(|a| canonical_name(a.name) == "hx-ext");
        if let Some(ext_attr) = existing_ext_attr {
            if let Some(val) = &ext_attr.value {
                if val.is_dynamic() {
                    return None;
                }
            }
        }

        let ext_name = if norm_name == "hx-ws" { "ws" } else { "sse" };

        let mut parts = Vec::new();
        let mut has_connect = false;

        if let Some(val) = &attr.value {
            if norm_name == "hx-ws" {
                for part in val.text.split_whitespace() {
                    if let Some(url) = part.strip_prefix("connect:") {
                        has_connect = true;
                        parts.push(format!("ws-connect=\"{}\"", url.replace('"', "&quot;")));
                    } else if part == "send" {
                        parts.push("ws-send".to_string());
                    } else {
                        // Unrecognized part: cannot safely migrate
                        return None;
                    }
                }
            } else {
                for part in val.text.split_whitespace() {
                    if let Some(url) = part.strip_prefix("connect:") {
                        has_connect = true;
                        parts.push(format!("sse-connect=\"{}\"", url.replace('"', "&quot;")));
                    } else if let Some(evt) = part.strip_prefix("swap:") {
                        parts.push(format!("sse-swap=\"{}\"", evt.replace('"', "&quot;")));
                    } else {
                        // Unrecognized part: cannot safely migrate
                        return None;
                    }
                }
            }
            if parts.is_empty() {
                return None;
            }
        }

        let needs_ext = has_connect || attr.value.is_none();
        let mut edits = Vec::new();

        if needs_ext {
            if let Some(ext_attr) = existing_ext_attr {
                let already_listed = ext_attr
                    .value
                    .as_ref()
                    .is_some_and(|v| v.text.split(',').any(|item| item.trim() == ext_name));

                if !already_listed {
                    let ext_new_text = match &ext_attr.value {
                        None => format!("{}=\"{}\"", ext_attr.name, ext_name),
                        Some(v) => {
                            let trimmed = v.text.trim();
                            if trimmed.is_empty() {
                                format!("{}=\"{}\"", ext_attr.name, ext_name)
                            } else {
                                format!("{}=\"{}, {}\"", ext_attr.name, trimmed, ext_name)
                            }
                        }
                    };
                    let ext_start_pos = doc.offset_to_position(ext_attr.name_range.start)?;
                    let ext_end_pos = doc.offset_to_position(ext_attr.end)?;
                    edits.push(TextEdit {
                        range: Range {
                            start: ext_start_pos,
                            end: ext_end_pos,
                        },
                        new_text: ext_new_text,
                    });
                }

                let deprecated_new_text = parts.join(" ");
                let dep_start_pos = doc.offset_to_position(attr.name_range.start)?;
                let dep_end_pos = doc.offset_to_position(attr.end)?;
                edits.push(TextEdit {
                    range: Range {
                        start: dep_start_pos,
                        end: dep_end_pos,
                    },
                    new_text: deprecated_new_text,
                });
            } else {
                let deprecated_new_text = if parts.is_empty() {
                    format!("hx-ext=\"{}\"", ext_name)
                } else {
                    format!("hx-ext=\"{}\" {}", ext_name, parts.join(" "))
                };
                let dep_start_pos = doc.offset_to_position(attr.name_range.start)?;
                let dep_end_pos = doc.offset_to_position(attr.end)?;
                edits.push(TextEdit {
                    range: Range {
                        start: dep_start_pos,
                        end: dep_end_pos,
                    },
                    new_text: deprecated_new_text,
                });
            }
        } else {
            let deprecated_new_text = parts.join(" ");
            let dep_start_pos = doc.offset_to_position(attr.name_range.start)?;
            let dep_end_pos = doc.offset_to_position(attr.end)?;
            edits.push(TextEdit {
                range: Range {
                    start: dep_start_pos,
                    end: dep_end_pos,
                },
                new_text: deprecated_new_text,
            });
        }

        let mut changes = HashMap::new();
        changes.insert(doc.uri.clone(), edits);

        let title = format!(
            "Migrate {} to HTMX 2.0 extension (hx-ext=\"{}\")",
            norm_name, ext_name
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
