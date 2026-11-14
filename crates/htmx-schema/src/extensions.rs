use crate::attribute::HtmxAttributeDefinition;
use serde::Serialize;

/// Definition of an HTMX official extension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExtensionDefinition {
    pub name: &'static str,
    pub summary: &'static str,
    pub description: &'static str,
    pub documentation_url: &'static str,
    pub attributes: &'static [HtmxAttributeDefinition],
}

pub static EXTENSIONS: &[ExtensionDefinition] = &[
    ExtensionDefinition {
        name: "ws",
        summary: "WebSockets support for HTMX 2.0",
        description: "The `ws` extension enables WebSocket connectivity in HTMX 2.0, replacing the legacy `hx-ws` attribute.",
        documentation_url: "https://htmx.org/extensions/web-sockets/",
        attributes: &[],
    },
    ExtensionDefinition {
        name: "sse",
        summary: "Server-Sent Events support for HTMX 2.0",
        description: "The `sse` extension enables Server-Sent Events in HTMX 2.0, replacing the legacy `hx-sse` attribute.",
        documentation_url: "https://htmx.org/extensions/server-sent-events/",
        attributes: &[],
    },
    ExtensionDefinition {
        name: "preload",
        summary: "Preload HTML fragments on mouseover or mousedown",
        description: "The `preload` extension allows preloading pages and fragments before the user clicks.",
        documentation_url: "https://htmx.org/extensions/preload/",
        attributes: &[],
    },
    ExtensionDefinition {
        name: "response-targets",
        summary: "Configure target elements based on HTTP status codes",
        description: "The `response-targets` extension allows specifying different swap targets based on response status codes (e.g. `hx-target-404=\"#not-found\"`).",
        documentation_url: "https://htmx.org/extensions/response-targets/",
        attributes: &[],
    },
    ExtensionDefinition {
        name: "head-support",
        summary: "Merge `<head>` tag updates into the document",
        description: "The `head-support` extension allows merging title, meta, link, and script tags from responses into the `<head>` of the page.",
        documentation_url: "https://htmx.org/extensions/head-support/",
        attributes: &[],
    },
    ExtensionDefinition {
        name: "idiomorph",
        summary: "Idiomorph morphing swap algorithm",
        description: "The `idiomorph` extension enables high-fidelity DOM morphing swaps preserving focus and transient state.",
        documentation_url: "https://github.com/bigskysoftware/idiomorph",
        attributes: &[],
    },
];

/// Look up an extension definition by name.
pub fn extension(name: &str) -> Option<&'static ExtensionDefinition> {
    EXTENSIONS.iter().find(|ext| ext.name == name)
}

/// Alias for [`extension`] adhering to legacy `get_` convention.
pub fn get_extension(name: &str) -> Option<&'static ExtensionDefinition> {
    extension(name)
}

pub fn all_extensions() -> &'static [ExtensionDefinition] {
    EXTENSIONS
}
