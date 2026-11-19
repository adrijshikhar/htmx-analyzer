use crate::config::Config;
use crate::navigation::WorkspaceIdIndex;
use htmx_parser::{
    document_ids, lex, HxSwapParser, HxTarget, HxTargetParser, HxTriggerParser, SwapModifier,
    TriggerModifier, MAX_FILE_SIZE,
};
use htmx_schema::{canonical_name, get_attribute};
use htmx_vfs::Document;
use lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Range};

pub struct DiagnosticEngine;

impl DiagnosticEngine {
    pub fn validate(doc: &Document, config: &Config) -> Vec<Diagnostic> {
        Self::validate_with_index(doc, config, None)
    }

    pub fn validate_with_index(
        doc: &Document,
        config: &Config,
        index: Option<&WorkspaceIdIndex>,
    ) -> Vec<Diagnostic> {
        let text = doc.text();
        if text.len() > MAX_FILE_SIZE {
            return Vec::new();
        }

        let tags = lex(&text);
        let doc_ids = document_ids(&tags);
        let mut diagnostics = Vec::new();

        let mut push_diag =
            |code: &str, builtin: DiagnosticSeverity, range: Range, message: String| {
                if let Some(severity) = config.severity(code, builtin) {
                    diagnostics.push(Diagnostic {
                        range,
                        severity: Some(severity),
                        code: Some(NumberOrString::String(code.to_string())),
                        source: Some("htmx-analyzer".to_string()),
                        message,
                        ..Default::default()
                    });
                }
            };

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
                                push_diag(
                                    "HTMX001",
                                    DiagnosticSeverity::WARNING,
                                    Range {
                                        start: start_pos,
                                        end: end_pos,
                                    },
                                    dep_msg.to_string(),
                                );
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
                                    push_diag(
                                        "HTMX002",
                                        DiagnosticSeverity::ERROR,
                                        Range { start: s, end: e },
                                        format!(
                                            "Unknown swap style '{}'. Valid styles: innerHTML, outerHTML, beforebegin, afterbegin, beforeend, afterend, delete, none.",
                                            style
                                        ),
                                    );
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
                                        push_diag(
                                            "HTMX002",
                                            DiagnosticSeverity::ERROR,
                                            Range { start: s, end: e },
                                            format!(
                                                "Invalid swap timing '{}'. Expected format like '500ms' or '1s'.",
                                                d
                                            ),
                                        );
                                    }
                                }
                                SwapModifier::Unknown(u) => {
                                    if let (Some(s), Some(e)) = (
                                        doc.offset_to_position(val.range.start),
                                        doc.offset_to_position(val.range.end),
                                    ) {
                                        push_diag(
                                            "HTMX002",
                                            DiagnosticSeverity::WARNING,
                                            Range { start: s, end: e },
                                            format!("Unknown swap modifier '{}'.", u),
                                        );
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
                                            push_diag(
                                                "HTMX003",
                                                DiagnosticSeverity::ERROR,
                                                Range { start: s, end: e },
                                                format!(
                                                    "Invalid delay duration '{}'. Expected format like '500ms' or '1s'.",
                                                    d
                                                ),
                                            );
                                        }
                                    }
                                    TriggerModifier::Throttle(d)
                                        if !HxTriggerParser::is_valid_duration(&d) =>
                                    {
                                        if let (Some(s), Some(e)) = (
                                            doc.offset_to_position(val.range.start),
                                            doc.offset_to_position(val.range.end),
                                        ) {
                                            push_diag(
                                                "HTMX003",
                                                DiagnosticSeverity::ERROR,
                                                Range { start: s, end: e },
                                                format!(
                                                    "Invalid throttle duration '{}'. Expected format like '500ms' or '1s'.",
                                                    d
                                                ),
                                            );
                                        }
                                    }
                                    TriggerModifier::Every(d)
                                        if !HxTriggerParser::is_valid_duration(&d) =>
                                    {
                                        if let (Some(s), Some(e)) = (
                                            doc.offset_to_position(val.range.start),
                                            doc.offset_to_position(val.range.end),
                                        ) {
                                            push_diag(
                                                "HTMX003",
                                                DiagnosticSeverity::ERROR,
                                                Range { start: s, end: e },
                                                format!(
                                                    "Invalid polling interval '{}'. Expected format like '2s'.",
                                                    d
                                                ),
                                            );
                                        }
                                    }
                                    TriggerModifier::Unknown(u) => {
                                        if let (Some(s), Some(e)) = (
                                            doc.offset_to_position(val.range.start),
                                            doc.offset_to_position(val.range.end),
                                        ) {
                                            push_diag(
                                                "HTMX003",
                                                DiagnosticSeverity::WARNING,
                                                Range { start: s, end: e },
                                                format!("Unknown trigger modifier '{}'.", u),
                                            );
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    // HTMX004: Missing target ID in document or workspace
                    "hx-target" => {
                        if let Some(HxTarget::Id(target_id)) = HxTargetParser::parse(val.text) {
                            let in_workspace = index
                                .map(|idx| idx.contains_id(&target_id))
                                .unwrap_or(false);
                            if !doc_ids.contains(target_id.as_str()) && !in_workspace {
                                if let (Some(s), Some(e)) = (
                                    doc.offset_to_position(val.range.start),
                                    doc.offset_to_position(val.range.end),
                                ) {
                                    push_diag(
                                        "HTMX004",
                                        DiagnosticSeverity::WARNING,
                                        Range { start: s, end: e },
                                        format!(
                                            "Target element ID '#{}' was not found in the current document.",
                                            target_id
                                        ),
                                    );
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
