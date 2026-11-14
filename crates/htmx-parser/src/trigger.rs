/// Representation of a parsed `hx-trigger` modifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TriggerModifier {
    Once,
    Changed,
    Delay(String),
    Throttle(String),
    From(String),
    Target(String),
    Consume,
    Queue(String),
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
    /// Multiple triggers are separated by commas (e.g. `click, keyup delay:500ms`).
    pub fn parse(input: &str) -> Vec<TriggerSpecification> {
        let mut results = Vec::new();
        for chunk in input.split(',') {
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

    fn parse_single_trigger(input: &str) -> Option<TriggerSpecification> {
        let tokens: Vec<&str> = input.split_whitespace().collect();
        if tokens.is_empty() {
            return None;
        }

        let mut token_idx = 0;
        let first_token = tokens[token_idx];
        token_idx += 1;

        let (event, filter) = if let Some(bracket_start) = first_token.find('[') {
            let event_name = &first_token[..bracket_start];
            let filter_content = if let Some(bracket_end) = first_token.find(']') {
                &first_token[bracket_start + 1..bracket_end]
            } else {
                &first_token[bracket_start + 1..]
            };
            (event_name.to_string(), Some(filter_content.to_string()))
        } else if token_idx < tokens.len() && tokens[token_idx].starts_with('[') {
            // Filter might be separated by space: "click [ctrlKey]"
            let filter_token = tokens[token_idx];
            token_idx += 1;
            let filter_trimmed = filter_token.trim_start_matches('[').trim_end_matches(']');
            (first_token.to_string(), Some(filter_trimmed.to_string()))
        } else {
            (first_token.to_string(), None)
        };

        let mut modifiers = Vec::new();
        while token_idx < tokens.len() {
            let tok = tokens[token_idx];
            token_idx += 1;

            if tok == "once" {
                modifiers.push(TriggerModifier::Once);
            } else if tok == "changed" {
                modifiers.push(TriggerModifier::Changed);
            } else if tok == "consume" {
                modifiers.push(TriggerModifier::Consume);
            } else if let Some(val) = tok.strip_prefix("delay:") {
                modifiers.push(TriggerModifier::Delay(val.to_string()));
            } else if let Some(val) = tok.strip_prefix("throttle:") {
                modifiers.push(TriggerModifier::Throttle(val.to_string()));
            } else if let Some(val) = tok.strip_prefix("from:") {
                modifiers.push(TriggerModifier::From(val.to_string()));
            } else if let Some(val) = tok.strip_prefix("target:") {
                modifiers.push(TriggerModifier::Target(val.to_string()));
            } else if let Some(val) = tok.strip_prefix("queue:") {
                modifiers.push(TriggerModifier::Queue(val.to_string()));
            } else {
                modifiers.push(TriggerModifier::Unknown(tok.to_string()));
            }
        }

        Some(TriggerSpecification {
            event,
            filter,
            modifiers,
            raw: input.to_string(),
        })
    }

    /// Validates duration formats like "500ms", "2s", "0.5s".
    pub fn is_valid_duration(duration: &str) -> bool {
        let (num_part, unit) = if let Some(stripped) = duration.strip_suffix("ms") {
            (stripped, "ms")
        } else if let Some(stripped) = duration.strip_suffix('s') {
            (stripped, "s")
        } else {
            return false;
        };

        if num_part.is_empty() {
            return false;
        }

        if unit == "ms" {
            num_part.parse::<u64>().is_ok()
        } else {
            num_part.parse::<f64>().is_ok()
        }
    }
}
