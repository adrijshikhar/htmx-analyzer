use serde::Serialize;

/// The semantic category of an HTMX attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum AttributeCategory {
    /// Core AJAX and swap attributes (`hx-get`, `hx-post`, `hx-target`, `hx-swap`, `hx-trigger`).
    Core,
    /// Ancillary request and behavior modifiers (`hx-boost`, `hx-indicator`, `hx-params`, etc.).
    Additional,
    /// Attributes supporting inheritance through DOM tree hierarchy (`hx-target`, `hx-boost`, `hx-indicator`).
    Inherited,
    /// Extension-specific attributes (`ws-connect`, `sse-connect`, etc.).
    Extension,
}

/// The expected value syntax / structure for an HTMX attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum AttributeValueType {
    /// URL endpoint (e.g. `/api/items`, `hx-get`).
    Url,
    /// CSS selector or target keyword (`#main`, `.item`, `this`, `closest div`).
    CssSelector,
    /// Swap style with optional modifier tokens (`innerHTML`, `outerHTML`, etc.).
    SwapStyle,
    /// Event name with optional filters and timing modifiers (`click delay:500ms`).
    TriggerSpecification,
    /// Comma-separated list of HTMX extensions (`ws`, `sse`, `preload`).
    ExtensionList,
    /// Strategy specification for synchronization (`closest form:abort`).
    SyncSpecification,
    /// Boolean flag ("true" or "false").
    Boolean,
    /// JSON-formatted string or JavaScript expression.
    JsonOrExpression,
    /// Parameter filter rules (`*`, `none`, `not ...`).
    Params,
    /// Free-form text or CSS class.
    Text,
}

/// Modifier definition for attributes like `hx-swap` or `hx-trigger`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModifierDefinition {
    pub name: &'static str,
    pub description: &'static str,
    pub takes_value: bool,
    pub allowed_values: &'static [&'static str],
}

/// Complete declarative specification for an HTMX attribute.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HtmxAttributeDefinition {
    pub name: &'static str,
    pub category: AttributeCategory,
    pub value_type: AttributeValueType,
    pub summary: &'static str,
    pub description: &'static str,
    pub allowed_values: &'static [&'static str],
    pub modifiers: &'static [ModifierDefinition],
    pub deprecated: Option<&'static str>,
    pub documentation_url: &'static str,
}

impl HtmxAttributeDefinition {
    /// Strip optional `data-` prefix to normalize `data-hx-*` to `hx-*`.
    pub fn normalize_name(name: &str) -> &str {
        if let Some(stripped) = name.strip_prefix("data-") {
            stripped
        } else {
            name
        }
    }

    /// Check if this definition matches the given attribute name (`hx-*` or `data-hx-*`).
    pub fn matches(&self, name: &str) -> bool {
        let norm = Self::normalize_name(name);
        self.name == norm
    }
}
