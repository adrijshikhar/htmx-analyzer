use htmx_service::{Config, HtmxLanguageService, SeverityLevel};
use htmx_vfs::Document;
use lsp_types::{DiagnosticSeverity, Uri};
use serde_json::json;
use std::str::FromStr;

#[test]
fn default_config_htmx004_is_hint() {
    let config = Config::default();
    assert_eq!(config.severity.get("HTMX004"), Some(&SeverityLevel::Hint));
    assert_eq!(
        config.severity("HTMX004", DiagnosticSeverity::WARNING),
        Some(DiagnosticSeverity::HINT)
    );
    assert_eq!(
        config.severity("HTMX002", DiagnosticSeverity::ERROR),
        Some(DiagnosticSeverity::ERROR)
    );
}

#[test]
fn user_override_merges_over_defaults() {
    let val = json!({
        "diagnostics": {
            "severity": {
                "HTMX001": "off"
            }
        }
    });
    let config = Config::from_json(&val);
    assert_eq!(
        config.severity("HTMX001", DiagnosticSeverity::WARNING),
        None
    );
    assert_eq!(
        config.severity("HTMX004", DiagnosticSeverity::WARNING),
        Some(DiagnosticSeverity::HINT)
    );
}

#[test]
fn invalid_entries_ignored() {
    let val = json!({
        "diagnostics": {
            "severity": {
                "HTMX002": "loud",
                "HTMX003": 5
            }
        }
    });
    let config = Config::from_json(&val);
    assert_eq!(
        config.severity("HTMX002", DiagnosticSeverity::ERROR),
        Some(DiagnosticSeverity::ERROR)
    );
    assert_eq!(
        config.severity("HTMX003", DiagnosticSeverity::ERROR),
        Some(DiagnosticSeverity::ERROR)
    );

    assert_eq!(
        Config::from_json(&serde_json::Value::Null),
        Config::default()
    );
}

#[test]
fn merged_layers_in_order() {
    let default = Config::default();
    let c1 = default
        .merged(&json!({ "HTMX004": "warning" }))
        .merged(&json!({}));
    assert_eq!(
        c1.severity("HTMX004", DiagnosticSeverity::HINT),
        Some(DiagnosticSeverity::WARNING)
    );

    let c2 = c1.merged(&json!({ "HTMX004": "error" }));
    assert_eq!(
        c2.severity("HTMX004", DiagnosticSeverity::HINT),
        Some(DiagnosticSeverity::ERROR)
    );
}

#[test]
fn validate_applies_config() {
    let uri = Uri::from_str("file:///test.html").unwrap();
    let doc = Document::new(uri, 1, "<b hx-target=\"#ghost\">");

    // Default: severity HINT
    let service_default = HtmxLanguageService::new();
    let diags = service_default.validate(&doc);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].severity, Some(DiagnosticSeverity::HINT));

    // Overridden to WARNING
    let config_warn = Config::from_json(&json!({
        "diagnostics": {
            "severity": {
                "HTMX004": "warning"
            }
        }
    }));
    let service_warn = HtmxLanguageService::with_config(config_warn);
    let diags = service_warn.validate(&doc);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].severity, Some(DiagnosticSeverity::WARNING));

    // Overridden to OFF
    let config_off = Config::from_json(&json!({
        "diagnostics": {
            "severity": {
                "HTMX004": "off"
            }
        }
    }));
    let service_off = HtmxLanguageService::with_config(config_off);
    let diags = service_off.validate(&doc);
    assert_eq!(diags.len(), 0);
}
