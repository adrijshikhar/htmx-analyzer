use htmx_parser::{
    document_ids, lex, HxSwapParser, HxTarget, HxTargetParser, HxTriggerParser, SwapModifier,
    TriggerModifier, MAX_FILE_SIZE,
};
use htmx_schema::{canonical_name, get_attribute};
use htmx_vfs::Document;
use lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Range};

pub struct DiagnosticEngine;

impl DiagnosticEngine {
    pub fn validate(doc: &Document) -> Vec<Diagnostic> {
        let text = doc.text();
        if text.len() > MAX_FILE_SIZE {
            return Vec::new();
        }

        let tags = lex(&text);
        let doc_ids = document_ids(&tags);
        let mut diagnostics = Vec::new();

        for tag in &tags {
            for attr in &tag.attributes {
                let canon_name = canonical_name(attr.name);

                // HTMX001: Deprecated attributes check
                if let Some(attr_def) = get_attribute(attr.name) {
                    if canon_name == attr_def.name {
                        if let Some(dep_msg) = attr_def.deprecated {
                            if let (Some(start_pos), Some(end_pos)) = (
                                doc.offset_to_position(attr.name_range.start),
                                doc.offset_to_position(attr.name_range.end),
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
                }

                let val = match &attr.value {
                    Some(v) => v,
                    None => continue,
                };

                // Skip static validation if attribute value contains dynamic template expressions
                if val.is_dynamic() {
                    continue;
                }

                match canon_name.as_str() {
                    // HTMX002: Invalid swap styles
                    "hx-swap" => {
                        let spec = HxSwapParser::parse(val.text);
                        if let Some(ref style) = spec.style {
                            if !HxSwapParser::is_valid_style(style) {
                                if let (Some(s), Some(e)) = (
                                    doc.offset_to_position(val.range.start),
                                    doc.offset_to_position(val.range.end),
                                ) {
                                    diagnostics.push(Diagnostic {
                                        range: Range { start: s, end: e },
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
                        }

                        for modifier in spec.modifiers {
                            match modifier {
                                SwapModifier::Swap(d) | SwapModifier::Settle(d)
                                    if !HxTriggerParser::is_valid_duration(&d) =>
                                {
                                    if let (Some(s), Some(e)) = (
                                        doc.offset_to_position(val.range.start),
                                        doc.offset_to_position(val.range.end),
                                    ) {
                                        diagnostics.push(Diagnostic {
                                            range: Range { start: s, end: e },
                                            severity: Some(DiagnosticSeverity::ERROR),
                                            code: Some(NumberOrString::String("HTMX002".to_string())),
                                            source: Some("htmx-analyzer".to_string()),
                                            message: format!(
                                                "Invalid swap timing '{}'. Expected format like '500ms' or '1s'.",
                                                d
                                            ),
                                            ..Default::default()
                                        });
                                    }
                                }
                                SwapModifier::Unknown(u) => {
                                    if let (Some(s), Some(e)) = (
                                        doc.offset_to_position(val.range.start),
                                        doc.offset_to_position(val.range.end),
                                    ) {
                                        diagnostics.push(Diagnostic {
                                            range: Range { start: s, end: e },
                                            severity: Some(DiagnosticSeverity::WARNING),
                                            code: Some(NumberOrString::String(
                                                "HTMX002".to_string(),
                                            )),
                                            source: Some("htmx-analyzer".to_string()),
                                            message: format!("Unknown swap modifier '{}'.", u),
                                            ..Default::default()
                                        });
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    // HTMX003: Invalid trigger durations & modifiers
                    "hx-trigger" => {
                        let triggers = HxTriggerParser::parse(val.text);
                        for t in triggers {
                            for m in t.modifiers {
                                match m {
                                    TriggerModifier::Delay(d)
                                        if !HxTriggerParser::is_valid_duration(&d) =>
                                    {
                                        if let (Some(s), Some(e)) = (
                                            doc.offset_to_position(val.range.start),
                                            doc.offset_to_position(val.range.end),
                                        ) {
                                            diagnostics.push(Diagnostic {
                                                range: Range { start: s, end: e },
                                                severity: Some(DiagnosticSeverity::ERROR),
                                                code: Some(NumberOrString::String(
                                                    "HTMX003".to_string(),
                                                )),
                                                source: Some("htmx-analyzer".to_string()),
                                                message: format!(
                                                    "Invalid delay duration '{}'. Expected format like '500ms' or '1s'.",
                                                    d
                                                ),
                                                ..Default::default()
                                            });
                                        }
                                    }
                                    TriggerModifier::Throttle(d)
                                        if !HxTriggerParser::is_valid_duration(&d) =>
                                    {
                                        if let (Some(s), Some(e)) = (
                                            doc.offset_to_position(val.range.start),
                                            doc.offset_to_position(val.range.end),
                                        ) {
                                            diagnostics.push(Diagnostic {
                                                range: Range { start: s, end: e },
                                                severity: Some(DiagnosticSeverity::ERROR),
                                                code: Some(NumberOrString::String(
                                                    "HTMX003".to_string(),
                                                )),
                                                source: Some("htmx-analyzer".to_string()),
                                                message: format!(
                                                    "Invalid throttle duration '{}'. Expected format like '500ms' or '1s'.",
                                                    d
                                                ),
                                                ..Default::default()
                                            });
                                        }
                                    }
                                    TriggerModifier::Every(d)
                                        if !HxTriggerParser::is_valid_duration(&d) =>
                                    {
                                        if let (Some(s), Some(e)) = (
                                            doc.offset_to_position(val.range.start),
                                            doc.offset_to_position(val.range.end),
                                        ) {
                                            diagnostics.push(Diagnostic {
                                                range: Range { start: s, end: e },
                                                severity: Some(DiagnosticSeverity::ERROR),
                                                code: Some(NumberOrString::String(
                                                    "HTMX003".to_string(),
                                                )),
                                                source: Some("htmx-analyzer".to_string()),
                                                message: format!(
                                                    "Invalid polling interval '{}'. Expected format like '2s'.",
                                                    d
                                                ),
                                                ..Default::default()
                                            });
                                        }
                                    }
                                    TriggerModifier::Unknown(u) => {
                                        if let (Some(s), Some(e)) = (
                                            doc.offset_to_position(val.range.start),
                                            doc.offset_to_position(val.range.end),
                                        ) {
                                            diagnostics.push(Diagnostic {
                                                range: Range { start: s, end: e },
                                                severity: Some(DiagnosticSeverity::WARNING),
                                                code: Some(NumberOrString::String(
                                                    "HTMX003".to_string(),
                                                )),
                                                source: Some("htmx-analyzer".to_string()),
                                                message: format!(
                                                    "Unknown trigger modifier '{}'.",
                                                    u
                                                ),
                                                ..Default::default()
                                            });
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    // HTMX004: Missing target ID in document
                    "hx-target" => {
                        if let Some(HxTarget::Id(target_id)) = HxTargetParser::parse(val.text) {
                            if !doc_ids.contains(target_id.as_str()) {
                                if let (Some(s), Some(e)) = (
                                    doc.offset_to_position(val.range.start),
                                    doc.offset_to_position(val.range.end),
                                ) {
                                    diagnostics.push(Diagnostic {
                                        range: Range { start: s, end: e },
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
                    }
                    _ => {}
                }
            }
        }

        diagnostics
    }
}
