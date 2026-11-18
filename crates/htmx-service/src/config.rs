use lsp_types::DiagnosticSeverity;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configurable severity level for diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SeverityLevel {
    Error,
    Warning,
    Information,
    Hint,
    Off,
}

/// Service configuration controlling diagnostic severities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub severity: HashMap<String, SeverityLevel>,
}

impl Default for Config {
    fn default() -> Self {
        let mut severity = HashMap::new();
        severity.insert("HTMX004".to_string(), SeverityLevel::Hint);
        Self { severity }
    }
}

impl Config {
    /// Create a new configuration by merging JSON overrides over the default configuration.
    pub fn from_json(value: &serde_json::Value) -> Self {
        Self::default().merged(value)
    }

    /// Merge JSON overrides over an existing configuration.
    pub fn merged(mut self, value: &serde_json::Value) -> Self {
        if value.is_null() {
            return self;
        }
        let mut target = value;
        if let Some(settings) = target.get("settings") {
            target = settings;
        }
        if let Some(diag_sev) = target.get("diagnostics").and_then(|d| d.get("severity")) {
            target = diag_sev;
        }

        if let Some(obj) = target.as_object() {
            for (k, v) in obj {
                if let Ok(level) = serde_json::from_value::<SeverityLevel>(v.clone()) {
                    self.severity.insert(k.clone(), level);
                }
            }
        }
        self
    }

    /// Determine the effective severity for a diagnostic code, returning `None` if disabled (`off`).
    pub fn severity(&self, code: &str, builtin: DiagnosticSeverity) -> Option<DiagnosticSeverity> {
        match self.severity.get(code) {
            Some(SeverityLevel::Off) => None,
            Some(SeverityLevel::Error) => Some(DiagnosticSeverity::ERROR),
            Some(SeverityLevel::Warning) => Some(DiagnosticSeverity::WARNING),
            Some(SeverityLevel::Information) => Some(DiagnosticSeverity::INFORMATION),
            Some(SeverityLevel::Hint) => Some(DiagnosticSeverity::HINT),
            None => Some(builtin),
        }
    }
}
