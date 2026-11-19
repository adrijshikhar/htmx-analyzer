use crate::lexer::{lex, Quote};

/// The position / semantic context of the cursor within an HTML document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CursorPositionKind {
    /// Cursor is on or right after an attribute name (e.g. `<button hx-|` or `<button hx-sw|ap`).
    AttributeName {
        prefix: String,
        range: (usize, usize),
    },
    /// Cursor is inside an attribute value (e.g. `<button hx-swap="in|">`).
    AttributeValue {
        attribute_name: String,
        value: String,
        cursor_offset_in_value: usize,
        value_range: (usize, usize),
    },
    /// Cursor is inside a tag body after tag name whitespace, ready for a new attribute (e.g. `<button |>`).
    TagBody,
    /// Cursor is outside tags (in text or whitespace) or inside the tag name itself.
    None,
}

/// Information about the HTML element and attribute under the cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorContext {
    pub tag_name: Option<String>,
    pub position_kind: CursorPositionKind,
    pub existing_attributes: Vec<String>,
}

pub struct CursorScanner;

impl CursorScanner {
    /// Scan cursor context at byte offset in HTML source.
    #[must_use]
    pub fn scan(source: &str, offset: usize) -> CursorContext {
        let offset = offset.min(source.len());
        let tags = lex(source);

        // Tag containing the offset: last tag with start < offset and (offset < end or !closed && offset <= end)
        let matching_tag = tags.into_iter().rev().find(|tag| {
            tag.start < offset && (offset < tag.end || (!tag.closed && offset <= tag.end))
        });

        let tag = match matching_tag {
            Some(t) => t,
            None => {
                return CursorContext {
                    tag_name: None,
                    position_kind: CursorPositionKind::None,
                    existing_attributes: Vec::new(),
                };
            }
        };

        // If cursor is still inside or directly at tag name without whitespace, don't report TagBody
        let attr_start_abs = tag.start + 1 + tag.name.len();
        if offset < attr_start_abs
            || (offset == attr_start_abs
                && (attr_start_abs >= source.len()
                    || !source.as_bytes()[attr_start_abs].is_ascii_whitespace()))
        {
            return CursorContext {
                tag_name: Some(tag.name.to_string()),
                position_kind: CursorPositionKind::None,
                existing_attributes: Vec::new(),
            };
        }

        let existing_attributes: Vec<String> =
            tag.attributes.iter().map(|a| a.name.to_string()).collect();

        let mut position_kind = CursorPositionKind::TagBody;

        for attr in &tag.attributes {
            // Check attribute name
            if offset >= attr.name_range.start && offset <= attr.name_range.end {
                position_kind = CursorPositionKind::AttributeName {
                    prefix: attr.name.to_string(),
                    range: (attr.name_range.start, attr.name_range.end),
                };
                break;
            }

            // Check attribute value
            if let Some(val) = &attr.value {
                match val.quote {
                    Quote::Brace => {
                        let brace_start = val.range.start.saturating_sub(1);
                        let brace_end = attr.end;
                        if offset >= brace_start && offset <= brace_end {
                            position_kind = CursorPositionKind::None;
                            break;
                        }
                    }
                    Quote::Double | Quote::Single | Quote::Unquoted => {
                        if offset >= val.range.start && offset <= val.range.end {
                            let cursor_offset_in_value = offset - val.range.start;
                            position_kind = CursorPositionKind::AttributeValue {
                                attribute_name: attr.name.to_string(),
                                value: val.text.to_string(),
                                cursor_offset_in_value,
                                value_range: (val.range.start, val.range.end),
                            };
                            break;
                        }
                    }
                }
            }
        }

        CursorContext {
            tag_name: Some(tag.name.to_string()),
            position_kind,
            existing_attributes,
        }
    }
}
