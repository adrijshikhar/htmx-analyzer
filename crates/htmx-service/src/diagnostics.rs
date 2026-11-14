use htmx_parser::{
    HtmlDocument, HxSwapParser, HxTarget, HxTargetParser, HxTriggerParser, SwapModifier,
    TriggerModifier, MAX_DEPTH,
};
use htmx_schema::get_attribute;
use htmx_vfs::Document;
use lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Range};

pub struct DiagnosticEngine;

impl DiagnosticEngine {
    pub fn validate(doc: &Document) -> Vec<Diagnostic> {
        let text = doc.text();
        let parsed = HtmlDocument::parse(&text);

        let mut diagnostics = Vec::new();

        if let Some(ref tree) = parsed.tree {
            let mut stack = vec![(tree.root_node(), 0usize)];

            while let Some((node, depth)) = stack.pop() {
                if depth > MAX_DEPTH {
                    continue;
                }

                if node.kind() == "attribute" {
                    Self::validate_attribute_node(node, &text, doc, &parsed, &mut diagnostics);
                }

                let child_count = node.child_count();
                for i in (0..child_count).rev() {
                    if let Some(child) = node.child(i) {
                        stack.push((child, depth + 1));
                    }
                }
            }
        }

        diagnostics
    }

    fn validate_attribute_node(
        node: tree_sitter::Node,
        source: &str,
        doc: &Document,
        parsed_doc: &HtmlDocument,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let mut attr_name: Option<(&str, (usize, usize))> = None;
        let mut attr_val: Option<(&str, (usize, usize))> = None;

        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.kind() == "attribute_name" {
                    let range = (child.start_byte(), child.end_byte());
                    attr_name = Some((&source[range.0..range.1], range));
                } else if child.kind() == "quoted_attribute_value"
                    || child.kind() == "attribute_value"
                {
                    let range = (child.start_byte(), child.end_byte());
                    let raw = &source[range.0..range.1];
                    let clean = raw.trim_matches(|c| c == '"' || c == '\'');
                    attr_val = Some((clean, range));
                }
            }
        }

        let (name, name_range) = match attr_name {
            Some(n) => n,
            None => return,
        };

        // HTMX001: Deprecated attributes check
        if let Some(attr_def) = get_attribute(name) {
            if let Some(dep_msg) = attr_def.deprecated {
                if let (Some(start_pos), Some(end_pos)) = (
                    doc.offset_to_position(name_range.0),
                    doc.offset_to_position(name_range.1),
                ) {
                    diagnostics.push(Diagnostic {
                        range: Range {
                            start: start_pos,
                            end: end_pos,
                        },
                        severity: Some(DiagnosticSeverity::WARNING),
                        code: Some(NumberOrString::String("HTMX001".to_string())),
                        source: Some("htmx-analyzer".to_string()),
                        message: dep_msg.to_string(),
                        ..Default::default()
                    });
                }
            }
        }

        let (val, val_range) = match attr_val {
            Some(v) => v,
            None => return,
        };

        let val_lsp_range = match (
            doc.offset_to_position(val_range.0),
            doc.offset_to_position(val_range.1),
        ) {
            (Some(s), Some(e)) => Range { start: s, end: e },
            _ => return,
        };

        let norm_name = htmx_schema::HtmxAttributeDefinition::normalize_name(name);

        match norm_name {
            // HTMX002: Invalid swap styles
            "hx-swap" => {
                let spec = HxSwapParser::parse(val);
                if let Some(ref style) = spec.style {
                    if !HxSwapParser::is_valid_style(style) {
                        diagnostics.push(Diagnostic {
                            range: val_lsp_range,
                            severity: Some(DiagnosticSeverity::ERROR),
                            code: Some(NumberOrString::String("HTMX002".to_string())),
                            source: Some("htmx-analyzer".to_string()),
                            message: format!(
                                "Unknown swap style '{}'. Valid styles: innerHTML, outerHTML, beforebegin, afterbegin, beforeend, afterend, delete, none.",
                                style
                            ),
                            ..Default::default()
                        });
                    }
                }

                for modifier in spec.modifiers {
                    if let SwapModifier::Unknown(u) = modifier {
                        diagnostics.push(Diagnostic {
                            range: val_lsp_range,
                            severity: Some(DiagnosticSeverity::WARNING),
                            code: Some(NumberOrString::String("HTMX002".to_string())),
                            source: Some("htmx-analyzer".to_string()),
                            message: format!("Unknown swap modifier '{}'.", u),
                            ..Default::default()
                        });
                    }
                }
            }
            // HTMX003: Invalid trigger durations & modifiers
            "hx-trigger" => {
                let triggers = HxTriggerParser::parse(val);
                for t in triggers {
                    for m in t.modifiers {
                        match m {
                            TriggerModifier::Delay(d)
                                if !HxTriggerParser::is_valid_duration(&d) =>
                            {
                                diagnostics.push(Diagnostic {
                                    range: val_lsp_range,
                                    severity: Some(DiagnosticSeverity::ERROR),
                                    code: Some(NumberOrString::String("HTMX003".to_string())),
                                    source: Some("htmx-analyzer".to_string()),
                                    message: format!(
                                        "Invalid delay duration '{}'. Expected format like '500ms' or '1s'.",
                                        d
                                    ),
                                    ..Default::default()
                                });
                            }
                            TriggerModifier::Throttle(d)
                                if !HxTriggerParser::is_valid_duration(&d) =>
                            {
                                diagnostics.push(Diagnostic {
                                    range: val_lsp_range,
                                    severity: Some(DiagnosticSeverity::ERROR),
                                    code: Some(NumberOrString::String("HTMX003".to_string())),
                                    source: Some("htmx-analyzer".to_string()),
                                    message: format!(
                                        "Invalid throttle duration '{}'. Expected format like '500ms' or '1s'.",
                                        d
                                    ),
                                    ..Default::default()
                                });
                            }
                            TriggerModifier::Unknown(u) => {
                                diagnostics.push(Diagnostic {
                                    range: val_lsp_range,
                                    severity: Some(DiagnosticSeverity::WARNING),
                                    code: Some(NumberOrString::String("HTMX003".to_string())),
                                    source: Some("htmx-analyzer".to_string()),
                                    message: format!("Unknown trigger modifier '{}'.", u),
                                    ..Default::default()
                                });
                            }
                            _ => {}
                        }
                    }
                }
            }
            // HTMX004: Missing target ID in document
            "hx-target" => {
                if let Some(HxTarget::Id(target_id)) = HxTargetParser::parse(val) {
                    if !parsed_doc.document_ids.contains(&target_id) {
                        diagnostics.push(Diagnostic {
                            range: val_lsp_range,
                            severity: Some(DiagnosticSeverity::WARNING),
                            code: Some(NumberOrString::String("HTMX004".to_string())),
                            source: Some("htmx-analyzer".to_string()),
                            message: format!(
                                "Target element ID '#{}' was not found in the current document.",
                                target_id
                            ),
                            ..Default::default()
                        });
                    }
                }
            }
            _ => {}
        }
    }
}
