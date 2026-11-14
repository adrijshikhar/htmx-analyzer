use htmx_parser::{CursorPositionKind, CursorScanner};
use htmx_schema::*;
use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind};

pub struct HoverProvider;

impl HoverProvider {
    pub fn hover(source: &str, offset: usize) -> Option<Hover> {
        let ctx = CursorScanner::scan(source, offset);

        match ctx.position_kind {
            CursorPositionKind::AttributeName { prefix, .. } => Self::hover_attribute_name(&prefix),
            CursorPositionKind::AttributeValue {
                attribute_name,
                value,
                cursor_offset_in_value,
                ..
            } => Self::hover_attribute_value(&attribute_name, &value, cursor_offset_in_value),
            _ => None,
        }
    }

    fn hover_attribute_name(attr_name: &str) -> Option<Hover> {
        let attr = get_attribute(attr_name)?;

        let mut markdown = format!("### `{}`\n\n**{}**\n\n", attr.name, attr.summary);

        if let Some(dep) = attr.deprecated {
            markdown.push_str(&format!("> ⚠️ **Deprecated in HTMX 2.0**\n> {}\n\n", dep));
        }

        markdown.push_str(attr.description);
        markdown.push_str(&format!(
            "\n\n---\n[Official HTMX Documentation]({})",
            attr.documentation_url
        ));

        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: markdown,
            }),
            range: None,
        })
    }

    fn hover_attribute_value(attr_name: &str, value: &str, cursor_offset: usize) -> Option<Hover> {
        let norm_name = HtmxAttributeDefinition::normalize_name(attr_name);

        // Find token under cursor inside the value
        let token = Self::token_under_cursor(value, cursor_offset)?;

        let markdown = match norm_name {
            "hx-swap" => {
                if let Some(mod_def) = SWAP_MODIFIERS.iter().find(|m| token.starts_with(m.name)) {
                    format!(
                        "### Swap Modifier: `{}`\n\n{}",
                        mod_def.name, mod_def.description
                    )
                } else if SWAP_STYLES.contains(&token) {
                    format!("### Swap Style: `{}`\n\nControls DOM replacement method for response content.", token)
                } else {
                    return None;
                }
            }
            "hx-trigger" => {
                if let Some(mod_def) = TRIGGER_MODIFIERS.iter().find(|m| token.starts_with(m.name))
                {
                    format!(
                        "### Trigger Modifier: `{}`\n\n{}",
                        mod_def.name, mod_def.description
                    )
                } else if TRIGGER_EVENTS.contains(&token) {
                    format!(
                        "### Trigger Event: `{}`\n\nDOM event triggering the HTMX request.",
                        token
                    )
                } else {
                    return None;
                }
            }
            "hx-ext" => {
                if let Some(ext) = get_extension(token) {
                    format!(
                        "### HTMX Extension: `{}`\n\n**{}**\n\n{}\n\n[Extension Documentation]({})",
                        ext.name, ext.summary, ext.description, ext.documentation_url
                    )
                } else {
                    return None;
                }
            }
            "hx-target" if TARGET_KEYWORDS.contains(&token) => {
                format!(
                    "### Target Selector Keyword: `{}`\n\nCalculates swap destination relative to triggering element.",
                    token
                )
            }
            _ => return None,
        };

        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: markdown,
            }),
            range: None,
        })
    }

    fn token_under_cursor(value: &str, offset: usize) -> Option<&str> {
        if value.is_empty() {
            return None;
        }

        let clamped = offset.min(value.len());
        let before = &value[..clamped];
        let after = &value[clamped..];

        let start = before
            .rfind(|c: char| c.is_whitespace() || c == ',')
            .map(|i| i + 1)
            .unwrap_or(0);
        let end = after
            .find(|c: char| c.is_whitespace() || c == ',')
            .map(|i| clamped + i)
            .unwrap_or(value.len());

        let tok = value[start..end].trim();
        if tok.is_empty() {
            None
        } else {
            Some(tok)
        }
    }
}
