use htmx_parser::{document_ids, lex, CursorPositionKind, CursorScanner};
use htmx_schema::*;
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionList, Documentation, InsertTextFormat,
    MarkupContent, MarkupKind,
};

pub const MAX_ID_COMPLETIONS: usize = 200;

pub struct CompletionProvider;

impl CompletionProvider {
    pub fn complete(source: &str, offset: usize) -> CompletionList {
        let ctx = CursorScanner::scan(source, offset);

        match ctx.position_kind {
            CursorPositionKind::AttributeName { prefix, .. } => CompletionList {
                is_incomplete: false,
                items: Self::complete_attribute_names(&prefix, &ctx.existing_attributes),
            },
            CursorPositionKind::TagBody => CompletionList {
                is_incomplete: false,
                items: Self::complete_attribute_names("", &ctx.existing_attributes),
            },
            CursorPositionKind::AttributeValue {
                attribute_name,
                value,
                cursor_offset_in_value,
                ..
            } => Self::complete_attribute_values(
                &attribute_name,
                &value,
                cursor_offset_in_value,
                source,
            ),
            CursorPositionKind::None => CompletionList {
                is_incomplete: false,
                items: Vec::new(),
            },
        }
    }

    fn complete_attribute_names(prefix: &str, existing: &[String]) -> Vec<CompletionItem> {
        let is_data_prefix = prefix.starts_with("data-");
        let mut items = Vec::new();

        let event_prefix = if let Some(p) = prefix.strip_prefix("hx-on:") {
            Some((p, "hx-on:"))
        } else {
            prefix
                .strip_prefix("data-hx-on:")
                .map(|p| (p, "data-hx-on:"))
        };

        if let Some((ev_p, base_prefix)) = event_prefix {
            for event in TRIGGER_EVENTS {
                if ev_p.is_empty() || event.starts_with(ev_p) {
                    let full_name = format!("{}{}", base_prefix, event);
                    items.push(CompletionItem {
                        label: full_name.clone(),
                        kind: Some(CompletionItemKind::EVENT),
                        detail: Some(format!("Inline handler for `{}` event", event)),
                        insert_text: Some(format!("{}=\"$1\"", full_name)),
                        insert_text_format: Some(InsertTextFormat::SNIPPET),
                        deprecated: Some(false),
                        ..Default::default()
                    });
                }
            }
            return items;
        }

        for attr in all_attributes() {
            let candidate_name = if is_data_prefix {
                format!("data-{}", attr.name)
            } else {
                attr.name.to_string()
            };

            // Avoid suggesting attributes already present on this tag, unless it is the attribute currently being typed
            if candidate_name != prefix
                && existing
                    .iter()
                    .any(|e| e == &candidate_name || e == attr.name)
            {
                continue;
            }

            if !prefix.is_empty() && !candidate_name.starts_with(prefix) {
                continue;
            }

            let insert_text = if attr.name == "hx-on" {
                format!("{}:$1=\"$2\"", candidate_name)
            } else {
                format!("{}=\"$1\"", candidate_name)
            };
            let doc = format!(
                "{}\n\n[HTMX Documentation]({})",
                attr.summary, attr.documentation_url
            );

            let is_deprecated = attr.deprecated.is_some() && attr.name != "hx-on";

            items.push(CompletionItem {
                label: candidate_name,
                kind: Some(CompletionItemKind::PROPERTY),
                detail: Some(attr.summary.to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: doc,
                })),
                insert_text: Some(insert_text),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                deprecated: Some(is_deprecated),
                ..Default::default()
            });
        }

        items
    }

    fn complete_attribute_values(
        attr_name: &str,
        value: &str,
        cursor_offset: usize,
        source: &str,
    ) -> CompletionList {
        let norm_name = HtmxAttributeDefinition::normalize_name(attr_name);
        let prefix_so_far = if cursor_offset <= value.len() {
            &value[..cursor_offset]
        } else {
            value
        };

        let last_token = prefix_so_far
            .split(|c: char| c.is_whitespace() || c == ',')
            .next_back()
            .unwrap_or("")
            .trim();

        let mut items = Vec::new();

        match norm_name {
            "hx-swap" => {
                // If cursor is at the first token and no style is present yet
                let words: Vec<&str> = value.split_whitespace().collect();
                if words.is_empty() || (words.len() == 1 && cursor_offset <= words[0].len()) {
                    for style in SWAP_STYLES {
                        if last_token.is_empty() || style.starts_with(last_token) {
                            items.push(CompletionItem {
                                label: style.to_string(),
                                kind: Some(CompletionItemKind::ENUM_MEMBER),
                                detail: Some("HTMX Swap Style".to_string()),
                                ..Default::default()
                            });
                        }
                    }
                }

                // Also suggest modifiers
                for m in SWAP_MODIFIERS {
                    let mod_prefix = format!("{}:", m.name);
                    if last_token.is_empty() || m.name.starts_with(last_token) {
                        items.push(CompletionItem {
                            label: mod_prefix.clone(),
                            kind: Some(CompletionItemKind::KEYWORD),
                            detail: Some(m.description.to_string()),
                            insert_text: Some(format!("{}$1", mod_prefix)),
                            insert_text_format: Some(InsertTextFormat::SNIPPET),
                            ..Default::default()
                        });
                    }
                }
            }
            "hx-trigger" => {
                let current_chunk = prefix_so_far.split(',').next_back().unwrap_or("");
                let words_in_chunk: Vec<&str> = current_chunk.split_whitespace().collect();
                if words_in_chunk.is_empty()
                    || (words_in_chunk.len() == 1 && !current_chunk.ends_with(' '))
                {
                    for event in TRIGGER_EVENTS {
                        if last_token.is_empty() || event.starts_with(last_token) {
                            items.push(CompletionItem {
                                label: event.to_string(),
                                kind: Some(CompletionItemKind::EVENT),
                                detail: Some("DOM Event".to_string()),
                                ..Default::default()
                            });
                        }
                    }
                    if last_token.is_empty() || "every".starts_with(last_token) {
                        items.push(CompletionItem {
                            label: "every".to_string(),
                            kind: Some(CompletionItemKind::KEYWORD),
                            detail: Some("HTMX Polling Trigger".to_string()),
                            insert_text: Some("every $1s".to_string()),
                            insert_text_format: Some(InsertTextFormat::SNIPPET),
                            ..Default::default()
                        });
                    }
                }

                // Modifiers
                for m in TRIGGER_MODIFIERS {
                    if last_token.is_empty() || m.name.starts_with(last_token) {
                        let insert = if m.takes_value {
                            format!("{}:$1", m.name)
                        } else {
                            m.name.to_string()
                        };
                        items.push(CompletionItem {
                            label: m.name.to_string(),
                            kind: Some(CompletionItemKind::KEYWORD),
                            detail: Some(m.description.to_string()),
                            insert_text: Some(insert),
                            insert_text_format: if m.takes_value {
                                Some(InsertTextFormat::SNIPPET)
                            } else {
                                None
                            },
                            ..Default::default()
                        });
                    }
                }
            }
            "hx-target" => {
                // Keywords
                for kw in TARGET_KEYWORDS {
                    if last_token.is_empty() || kw.starts_with(last_token) {
                        let insert = match *kw {
                            "closest" => "closest $1",
                            "find" => "find $1",
                            "next" => "next $1",
                            "previous" => "previous $1",
                            _ => *kw,
                        };
                        items.push(CompletionItem {
                            label: kw.to_string(),
                            kind: Some(CompletionItemKind::KEYWORD),
                            detail: Some("HTMX Target Keyword".to_string()),
                            insert_text: Some(insert.to_string()),
                            insert_text_format: if insert.contains("$1") {
                                Some(InsertTextFormat::SNIPPET)
                            } else {
                                None
                            },
                            ..Default::default()
                        });
                    }
                }

                // Document IDs extracted with the linear lexer
                let tags = lex(source);
                let mut ids: Vec<&str> = document_ids(&tags).into_iter().collect();
                ids.sort_unstable();

                let mut matching_ids = Vec::new();
                for id in ids {
                    let label = format!("#{}", id);
                    if last_token.is_empty()
                        || label.starts_with(last_token)
                        || id.starts_with(last_token)
                    {
                        matching_ids.push((id, label));
                    }
                }

                let is_incomplete = matching_ids.len() > MAX_ID_COMPLETIONS;
                for (id, label) in matching_ids.into_iter().take(MAX_ID_COMPLETIONS) {
                    items.push(CompletionItem {
                        label: label.clone(),
                        kind: Some(CompletionItemKind::REFERENCE),
                        detail: Some(format!("Document ID: {}", id)),
                        insert_text: Some(label),
                        ..Default::default()
                    });
                }

                return CompletionList {
                    is_incomplete,
                    items,
                };
            }
            "hx-ext" => {
                for ext in all_extensions() {
                    if last_token.is_empty() || ext.name.starts_with(last_token) {
                        items.push(CompletionItem {
                            label: ext.name.to_string(),
                            kind: Some(CompletionItemKind::MODULE),
                            detail: Some(ext.summary.to_string()),
                            documentation: Some(Documentation::MarkupContent(MarkupContent {
                                kind: MarkupKind::Markdown,
                                value: ext.description.to_string(),
                            })),
                            ..Default::default()
                        });
                    }
                }
            }
            _ => {
                if let Some(attr) = get_attribute(norm_name) {
                    for val in attr.allowed_values {
                        if last_token.is_empty() || val.starts_with(last_token) {
                            items.push(CompletionItem {
                                label: val.to_string(),
                                kind: Some(CompletionItemKind::VALUE),
                                ..Default::default()
                            });
                        }
                    }
                }
            }
        }

        CompletionList {
            is_incomplete: false,
            items,
        }
    }
}
