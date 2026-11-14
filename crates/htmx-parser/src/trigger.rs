/// Representation of a parsed `hx-trigger` modifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TriggerModifier {
    Once,
    Changed,
    Delay(String),
    Throttle(String),
    From(String),
    Target(String),
    Root(String),
    Threshold(String),
    Consume,
    Queue(String),
    Every(String),
    Unknown(String),
}

/// A parsed single event trigger specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggerSpecification {
    pub event: String,
    pub filter: Option<String>,
    pub modifiers: Vec<TriggerModifier>,
    pub raw: String,
}

/// Linear-time scanner and parser for `hx-trigger`.
/// Never uses backtracking regular expressions (ReDoS-safe).
pub struct HxTriggerParser;

impl HxTriggerParser {
    /// Parse an `hx-trigger` attribute value into a list of trigger specifications.
    pub fn parse(input: &str) -> Vec<TriggerSpecification> {
        let mut results = Vec::new();
        for chunk in Self::split_triggers_by_comma(input) {
            let trimmed = chunk.trim();
            if trimmed.is_empty() {
                continue;
            }

            if let Some(spec) = Self::parse_single_trigger(trimmed) {
                results.push(spec);
            }
        }
        results
    }

    /// Split trigger specifications on commas without breaking bracketed/parenthesised filters or selectors.
    fn split_triggers_by_comma(input: &str) -> Vec<String> {
        let mut chunks = Vec::new();
        let mut current = String::new();
        let mut group_depth: usize = 0;

        for ch in input.chars() {
            match ch {
                '[' | '(' | '{' => {
                    group_depth += 1;
                    current.push(ch);
                }
                ']' | ')' | '}' => {
                    group_depth = group_depth.saturating_sub(1);
                    current.push(ch);
                }
                ',' if group_depth == 0 => {
                    chunks.push(std::mem::take(&mut current));
                }
                _ => current.push(ch),
            }
        }

        if !current.trim().is_empty() {
            chunks.push(current);
        }

        chunks
    }

    /// Tokenize while keeping bracketed, parenthesized, and braced filters/selectors intact.
    fn tokenize_trigger(input: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut group_depth: usize = 0;

        for ch in input.chars() {
            if ch == '[' || ch == '(' || ch == '{' {
                group_depth += 1;
                current.push(ch);
            } else if ch == ']' || ch == ')' || ch == '}' {
                group_depth = group_depth.saturating_sub(1);
                current.push(ch);
            } else if ch.is_whitespace() && group_depth == 0 {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            } else {
                current.push(ch);
            }
        }

        if !current.is_empty() {
            tokens.push(current);
        }

        tokens
    }

    fn parse_single_trigger(input: &str) -> Option<TriggerSpecification> {
        let tokens = Self::tokenize_trigger(input);
        if tokens.is_empty() {
            return None;
        }

        let mut token_idx = 0;
        let first_token = &tokens[token_idx];
        token_idx += 1;

        // Check for polling syntax: "every 2s" or "every 2s [condition]"
        if first_token == "every" && token_idx < tokens.len() {
            let duration = tokens[token_idx].clone();
            token_idx += 1;

            let mut filter = None;
            if token_idx < tokens.len() && tokens[token_idx].starts_with('[') {
                let filter_token = &tokens[token_idx];
                token_idx += 1;
                filter = Some(
                    filter_token
                        .trim_start_matches('[')
                        .trim_end_matches(']')
                        .to_string(),
                );
            }

            let mut modifiers = vec![TriggerModifier::Every(duration.clone())];

            while token_idx < tokens.len() {
                let tok = &tokens[token_idx];
                token_idx += 1;
                modifiers.push(Self::parse_modifier(tok, &tokens, &mut token_idx));
            }

            return Some(TriggerSpecification {
                event: format!("every {}", duration),
                filter,
                modifiers,
                raw: input.to_string(),
            });
        }

        let (event, filter) = if let Some(bracket_start) = first_token.find('[') {
            let event_name = &first_token[..bracket_start];
            let after_open = &first_token[bracket_start + 1..];
            let filter_content = if let Some(bracket_end_rel) = after_open.find(']') {
                &after_open[..bracket_end_rel]
            } else {
                after_open
            };
            (event_name.to_string(), Some(filter_content.to_string()))
        } else if token_idx < tokens.len() && tokens[token_idx].starts_with('[') {
            let filter_token = &tokens[token_idx];
            token_idx += 1;
            let filter_trimmed = filter_token.trim_start_matches('[').trim_end_matches(']');
            (first_token.to_string(), Some(filter_trimmed.to_string()))
        } else {
            (first_token.to_string(), None)
        };

        let mut modifiers = Vec::new();
        while token_idx < tokens.len() {
            let tok = &tokens[token_idx];
            token_idx += 1;
            modifiers.push(Self::parse_modifier(tok, &tokens, &mut token_idx));
        }

        Some(TriggerSpecification {
            event,
            filter,
            modifiers,
            raw: input.to_string(),
        })
    }

    fn parse_modifier(tok: &str, tokens: &[String], token_idx: &mut usize) -> TriggerModifier {
        if tok == "once" {
            TriggerModifier::Once
        } else if tok == "changed" {
            TriggerModifier::Changed
        } else if tok == "consume" {
            TriggerModifier::Consume
        } else if let Some(val) = tok.strip_prefix("delay:") {
            TriggerModifier::Delay(val.to_string())
        } else if let Some(val) = tok.strip_prefix("throttle:") {
            TriggerModifier::Throttle(val.to_string())
        } else if let Some(val) = tok.strip_prefix("from:") {
            let selector = Self::consume_extended_selector(val, tokens, token_idx);
            TriggerModifier::From(selector)
        } else if let Some(val) = tok.strip_prefix("target:") {
            let selector = Self::consume_extended_selector(val, tokens, token_idx);
            TriggerModifier::Target(selector)
        } else if let Some(val) = tok.strip_prefix("root:") {
            let selector = Self::consume_extended_selector(val, tokens, token_idx);
            TriggerModifier::Root(selector)
        } else if let Some(val) = tok.strip_prefix("threshold:") {
            TriggerModifier::Threshold(val.to_string())
        } else if let Some(val) = tok.strip_prefix("queue:") {
            TriggerModifier::Queue(val.to_string())
        } else {
            TriggerModifier::Unknown(tok.to_string())
        }
    }

    fn consume_extended_selector(
        initial: &str,
        tokens: &[String],
        token_idx: &mut usize,
    ) -> String {
        let mut selector = initial.to_string();
        if (initial == "closest" || initial == "find" || initial == "next" || initial == "previous")
            && *token_idx < tokens.len()
        {
            selector.push(' ');
            selector.push_str(&tokens[*token_idx]);
            *token_idx += 1;
        }
        selector
    }

    fn is_valid_number(s: &str) -> bool {
        if s.is_empty() {
            return false;
        }
        let mut has_digit = false;
        let mut dot_count = 0;
        for c in s.chars() {
            if c == '.' {
                dot_count += 1;
                if dot_count > 1 {
                    return false;
                }
            } else if c.is_ascii_digit() {
                has_digit = true;
            } else {
                return false;
            }
        }
        has_digit
    }

    /// Validates duration formats like "500ms", "2s", "0.5s", "1m", or bare numbers like "500".
    pub fn is_valid_duration(duration: &str) -> bool {
        let trimmed = duration.trim();
        if trimmed.is_empty() {
            return false;
        }

        if let Some(stripped) = trimmed.strip_suffix("ms") {
            Self::is_valid_number(stripped)
        } else if let Some(stripped) = trimmed.strip_suffix('s') {
            Self::is_valid_number(stripped)
        } else if let Some(stripped) = trimmed.strip_suffix('m') {
            Self::is_valid_number(stripped)
        } else {
            trimmed.chars().all(|c| c.is_ascii_digit())
        }
    }
}
