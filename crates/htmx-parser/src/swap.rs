use htmx_schema::SWAP_STYLES;

/// Direction for scroll and show modifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScrollDirection {
    Top,
    Bottom,
    Unknown(String),
}

/// Parsed `hx-swap` modifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwapModifier {
    Transition(bool),
    Swap(String),
    Settle(String),
    Scroll {
        target: Option<String>,
        direction: ScrollDirection,
    },
    Show {
        target: Option<String>,
        direction: ScrollDirection,
    },
    FocusScroll(bool),
    IgnoreTitle(bool),
    Unknown(String),
}

/// A parsed `hx-swap` attribute definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwapSpecification {
    pub style: Option<String>,
    pub modifiers: Vec<SwapModifier>,
    pub raw: String,
}

/// Linear-time scanner and parser for `hx-swap`.
pub struct HxSwapParser;

impl HxSwapParser {
    pub fn parse(input: &str) -> SwapSpecification {
        let tokens: Vec<&str> = input.split_whitespace().collect();
        if tokens.is_empty() {
            return SwapSpecification {
                style: None,
                modifiers: Vec::new(),
                raw: input.to_string(),
            };
        }

        let mut style = None;
        let mut modifiers = Vec::new();

        for (idx, token) in tokens.iter().enumerate() {
            if idx == 0
                && (SWAP_STYLES.contains(token)
                    || !token.contains(':')
                    || token.starts_with("morph:"))
            {
                style = Some(token.to_string());
                continue;
            }

            if let Some(val) = token.strip_prefix("transition:") {
                let enabled = val == "true";
                modifiers.push(SwapModifier::Transition(enabled));
            } else if let Some(val) = token.strip_prefix("swap:") {
                modifiers.push(SwapModifier::Swap(val.to_string()));
            } else if let Some(val) = token.strip_prefix("settle:") {
                modifiers.push(SwapModifier::Settle(val.to_string()));
            } else if let Some(val) = token.strip_prefix("scroll:") {
                modifiers.push(Self::parse_scroll_or_show(val, true));
            } else if let Some(val) = token.strip_prefix("show:") {
                modifiers.push(Self::parse_scroll_or_show(val, false));
            } else if let Some(val) = token.strip_prefix("focus-scroll:") {
                modifiers.push(SwapModifier::FocusScroll(val == "true"));
            } else if let Some(val) = token.strip_prefix("ignoreTitle:") {
                modifiers.push(SwapModifier::IgnoreTitle(val == "true"));
            } else if style.is_none() && idx == 0 {
                style = Some(token.to_string());
            } else {
                modifiers.push(SwapModifier::Unknown(token.to_string()));
            }
        }

        SwapSpecification {
            style,
            modifiers,
            raw: input.to_string(),
        }
    }

    fn parse_scroll_or_show(value: &str, is_scroll: bool) -> SwapModifier {
        let (target, dir_str) = match value.split_once(':') {
            Some((target, dir)) => (Some(target.to_string()), dir),
            None => (None, value),
        };

        let direction = match dir_str {
            "top" => ScrollDirection::Top,
            "bottom" => ScrollDirection::Bottom,
            other => ScrollDirection::Unknown(other.to_string()),
        };

        if is_scroll {
            SwapModifier::Scroll { target, direction }
        } else {
            SwapModifier::Show { target, direction }
        }
    }

    /// Check if a style string is one of the valid standard HTMX swap styles or recognized extension swap styles.
    pub fn is_valid_style(style: &str) -> bool {
        style.starts_with("morph:")
            || style == "morph"
            || style == "morphdom"
            || SWAP_STYLES.contains(&style)
    }
}
