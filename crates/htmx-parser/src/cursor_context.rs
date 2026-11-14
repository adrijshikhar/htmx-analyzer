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
    /// Cursor is inside a tag, ready for a new attribute (e.g. `<button |>`).
    TagBody,
    /// Cursor is outside tags (in text or whitespace).
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
    pub fn scan(source: &str, offset: usize) -> CursorContext {
        let offset = offset.min(source.len());

        // Find the start of the current tag by scanning backwards from offset
        let tag_start = match Self::find_tag_start(source, offset) {
            Some(pos) => pos,
            None => {
                return CursorContext {
                    tag_name: None,
                    position_kind: CursorPositionKind::None,
                    existing_attributes: Vec::new(),
                };
            }
        };

        // Slice from tag_start to find tag end or end of relevant context
        let tag_content = &source[tag_start..];
        let tag_end_rel = Self::find_tag_end(tag_content);
        let tag_end = tag_start + tag_end_rel;

        // If offset is strictly after tag_end, cursor is not inside this tag
        if offset > tag_end {
            return CursorContext {
                tag_name: None,
                position_kind: CursorPositionKind::None,
                existing_attributes: Vec::new(),
            };
        }

        // Parse tag name
        let after_bracket = &source[tag_start + 1..];
        let tag_name_len = after_bracket
            .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
            .unwrap_or(after_bracket.len());
        let tag_name = after_bracket[..tag_name_len].to_string();

        if tag_name.is_empty() || tag_name.starts_with('!') || tag_name.starts_with('/') {
            return CursorContext {
                tag_name: None,
                position_kind: CursorPositionKind::None,
                existing_attributes: Vec::new(),
            };
        }

        // Now parse attributes between tag_name_end and tag_end
        let attr_start_abs = tag_start + 1 + tag_name_len;
        let attr_region_end = tag_end.min(source.len());

        Self::parse_tag_attributes(source, attr_start_abs, attr_region_end, offset, tag_name)
    }

    fn find_tag_start(source: &str, offset: usize) -> Option<usize> {
        let bytes = source.as_bytes();
        let mut i = offset;

        // Look back for '<'
        while i > 0 {
            i -= 1;
            if bytes[i] == b'>' {
                // If there's an unquoted '>' before '<', we're outside a tag
                return None;
            }
            if bytes[i] == b'<' {
                // Check if this is a comment <!--
                if source[i..].starts_with("<!--") {
                    return None;
                }
                return Some(i);
            }
        }
        None
    }

    fn find_tag_end(tag_slice: &str) -> usize {
        let mut in_quote = None;
        for (idx, ch) in tag_slice.char_indices() {
            if let Some(q) = in_quote {
                if ch == q {
                    in_quote = None;
                }
            } else if ch == '"' || ch == '\'' {
                in_quote = Some(ch);
            } else if ch == '>' {
                return idx + 1;
            }
        }
        tag_slice.len()
    }

    fn parse_tag_attributes(
        source: &str,
        start: usize,
        end: usize,
        offset: usize,
        tag_name: String,
    ) -> CursorContext {
        let slice = if start <= end && end <= source.len() {
            &source[start..end]
        } else {
            ""
        };

        let mut existing_attributes = Vec::new();
        let mut current_position = CursorPositionKind::TagBody;

        let bytes = slice.as_bytes();
        let mut i = 0;

        while i < bytes.len() {
            // Skip whitespace
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                let abs_pos = start + i;
                if abs_pos == offset {
                    current_position = CursorPositionKind::TagBody;
                }
                i += 1;
            }

            if i >= bytes.len() || bytes[i] == b'>' || bytes[i] == b'/' {
                break;
            }

            let attr_name_start = i;
            while i < bytes.len()
                && !bytes[i].is_ascii_whitespace()
                && bytes[i] != b'='
                && bytes[i] != b'>'
                && bytes[i] != b'/'
            {
                i += 1;
            }
            let attr_name_end = i;
            let attr_name = &slice[attr_name_start..attr_name_end];
            if !attr_name.is_empty() {
                existing_attributes.push(attr_name.to_string());
            }

            let abs_name_start = start + attr_name_start;
            let abs_name_end = start + attr_name_end;

            // Check if cursor is on this attribute name
            if offset >= abs_name_start && offset <= abs_name_end {
                current_position = CursorPositionKind::AttributeName {
                    prefix: attr_name.to_string(),
                    range: (abs_name_start, abs_name_end),
                };
            }

            // Skip whitespace before '='
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }

            // Check for '='
            if i < bytes.len() && bytes[i] == b'=' {
                i += 1;
                // Skip whitespace after '='
                while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }

                if i < bytes.len() && (bytes[i] == b'"' || bytes[i] == b'\'') {
                    let quote_char = bytes[i];
                    i += 1; // Skip quote
                    let val_start = i;
                    while i < bytes.len() && bytes[i] != quote_char {
                        i += 1;
                    }
                    let val_end = i;
                    let val = &slice[val_start..val_end];

                    let abs_val_start = start + val_start;
                    let abs_val_end = start + val_end;

                    // If closing quote exists, skip it
                    if i < bytes.len() && bytes[i] == quote_char {
                        i += 1;
                    }

                    if offset >= abs_val_start && offset <= abs_val_end {
                        let cursor_offset_in_val = offset - abs_val_start;
                        current_position = CursorPositionKind::AttributeValue {
                            attribute_name: attr_name.to_string(),
                            value: val.to_string(),
                            cursor_offset_in_value: cursor_offset_in_val,
                            value_range: (abs_val_start, abs_val_end),
                        };
                    }
                } else {
                    // Unquoted attribute value
                    let val_start = i;
                    while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'>' {
                        i += 1;
                    }
                    let val_end = i;
                    let val = &slice[val_start..val_end];
                    let abs_val_start = start + val_start;
                    let abs_val_end = start + val_end;

                    if offset >= abs_val_start && offset <= abs_val_end {
                        let cursor_offset_in_val = offset - abs_val_start;
                        current_position = CursorPositionKind::AttributeValue {
                            attribute_name: attr_name.to_string(),
                            value: val.to_string(),
                            cursor_offset_in_value: cursor_offset_in_val,
                            value_range: (abs_val_start, abs_val_end),
                        };
                    }
                }
            }
        }

        CursorContext {
            tag_name: Some(tag_name),
            position_kind: current_position,
            existing_attributes,
        }
    }
}
