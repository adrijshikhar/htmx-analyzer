/// Parsed target expression for `hx-target` and related attributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HxTarget {
    /// The element itself.
    This,
    /// Closest ancestor matching CSS selector.
    Closest(String),
    /// Descendant matching CSS selector.
    Find(String),
    /// Next sibling matching CSS selector.
    Next(String),
    /// Previous sibling matching CSS selector.
    Previous(String),
    /// ID selector `#some-id`.
    Id(String),
    /// Class selector `.some-class`.
    Class(String),
    /// General CSS selector.
    Selector(String),
}

pub struct HxTargetParser;

impl HxTargetParser {
    pub fn parse(input: &str) -> Option<HxTarget> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return None;
        }

        if trimmed == "this" {
            return Some(HxTarget::This);
        }

        if let Some(rest) = trimmed.strip_prefix("closest ") {
            return Some(HxTarget::Closest(rest.trim().to_string()));
        }

        if let Some(rest) = trimmed.strip_prefix("find ") {
            return Some(HxTarget::Find(rest.trim().to_string()));
        }

        if let Some(rest) = trimmed.strip_prefix("next ") {
            return Some(HxTarget::Next(rest.trim().to_string()));
        }

        if let Some(rest) = trimmed.strip_prefix("previous ") {
            return Some(HxTarget::Previous(rest.trim().to_string()));
        }

        if let Some(id) = trimmed.strip_prefix('#') {
            return Some(HxTarget::Id(id.trim().to_string()));
        }

        if let Some(cls) = trimmed.strip_prefix('.') {
            return Some(HxTarget::Class(cls.trim().to_string()));
        }

        Some(HxTarget::Selector(trimmed.to_string()))
    }
}
