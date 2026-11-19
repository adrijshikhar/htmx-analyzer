use std::collections::HashSet;
use std::ops::Range;

pub const MAX_FILE_SIZE: usize = 5 * 1024 * 1024; // 5 MiB

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quote {
    Double,
    Single,
    Brace,
    Unquoted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttrValue<'a> {
    pub text: &'a str,
    pub range: Range<usize>,
    pub quote: Quote,
}

impl AttrValue<'_> {
    #[must_use]
    pub fn is_dynamic(&self) -> bool {
        self.quote == Quote::Brace
            || self.text.contains('{')
            || self.text.contains("<%")
            || self.text.contains("<?")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute<'a> {
    pub name: &'a str,
    pub name_range: Range<usize>,
    pub value: Option<AttrValue<'a>>,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag<'a> {
    pub name: &'a str,
    pub start: usize,
    pub end: usize,
    pub closed: bool,
    pub attributes: Vec<Attribute<'a>>,
}

/// An ID definition declared via `<tag id="...">`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdDefinition<'a> {
    pub id: &'a str,
    pub value_range: Range<usize>,
    pub tag_range: Range<usize>,
}

/// An ID reference point, either from an element declaration `id="my-id"`
/// or an HTMX attribute targeting an ID (e.g. `hx-target="#my-id"`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdReference<'a> {
    pub id: &'a str,
    pub span: Range<usize>,         // exact byte range of "my-id"
    pub origin_span: Range<usize>,  // byte range of "#my-id" or "my-id"
    pub is_declaration: bool,
}

/// Check if attribute name can reference or declare an ID.
#[must_use]
pub fn is_id_referencing_attribute(attr_name: &str) -> bool {
    let lower = attr_name.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "id"
            | "hx-target"
            | "data-hx-target"
            | "hx-indicator"
            | "data-hx-indicator"
            | "hx-swap-oob"
            | "data-hx-swap-oob"
            | "hx-sync"
            | "data-hx-sync"
            | "hx-select"
            | "data-hx-select"
            | "hx-select-oob"
            | "data-hx-select-oob"
    )
}

/// Find all ID definitions (`id="..."`) in the parsed tags.
#[must_use]
pub fn find_id_definitions<'a>(tags: &[Tag<'a>]) -> Vec<IdDefinition<'a>> {
    let mut defs = Vec::new();
    for tag in tags {
        for attr in &tag.attributes {
            if attr.name.eq_ignore_ascii_case("id") {
                if let Some(val) = &attr.value {
                    if !val.text.is_empty() && !val.is_dynamic() {
                        defs.push(IdDefinition {
                            id: val.text,
                            value_range: val.range.clone(),
                            tag_range: tag.start..tag.end,
                        });
                    }
                }
            }
        }
    }
    defs
}

/// Collect all ID references from tags across `id` declarations and HTMX attributes.
#[must_use]
pub fn find_id_references<'a>(tags: &[Tag<'a>]) -> Vec<IdReference<'a>> {
    let mut refs = Vec::new();
    for tag in tags {
        collect_tag_id_references(tag, &mut refs);
    }
    refs
}

fn collect_tag_id_references<'a>(tag: &Tag<'a>, refs: &mut Vec<IdReference<'a>>) {
    for attr in &tag.attributes {
        if !is_id_referencing_attribute(attr.name) {
            continue;
        }
        let Some(val) = &attr.value else { continue };
        if val.text.is_empty() || val.is_dynamic() {
            continue;
        }

        if attr.name.eq_ignore_ascii_case("id") {
            refs.push(IdReference {
                id: val.text,
                span: val.range.clone(),
                origin_span: val.range.clone(),
                is_declaration: true,
            });
            continue;
        }

        // Scan for #<id_chars> in target attributes
        let bytes = val.text.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'#' {
                let hash_idx = i;
                let id_start = hash_idx + 1;
                let mut id_end = id_start;
                while id_end < bytes.len() {
                    let b = bytes[id_end];
                    if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' {
                        id_end += 1;
                    } else {
                        break;
                    }
                }
                if id_end > id_start {
                    let id = &val.text[id_start..id_end];
                    let origin_start = val.range.start + hash_idx;
                    let origin_end = val.range.start + id_end;
                    let span_start = val.range.start + id_start;
                    let span_end = origin_end;
                    refs.push(IdReference {
                        id,
                        span: span_start..span_end,
                        origin_span: origin_start..origin_end,
                        is_declaration: false,
                    });
                }
                i = id_end;
            } else {
                i += 1;
            }
        }
    }
}

/// Find the ID reference under the cursor at `offset`, if any.
#[must_use]
pub fn find_id_reference_at<'a>(tags: &[Tag<'a>], offset: usize) -> Option<IdReference<'a>> {
    for tag in tags {
        if offset < tag.start || offset > tag.end {
            continue;
        }
        let mut refs = Vec::new();
        collect_tag_id_references(tag, &mut refs);
        for r in refs {
            if offset >= r.origin_span.start && offset <= r.origin_span.end {
                return Some(r);
            }
        }
    }
    None
}

#[must_use]
pub fn document_ids<'a>(tags: &[Tag<'a>]) -> HashSet<&'a str> {
    find_id_definitions(tags).into_iter().map(|d| d.id).collect()
}

fn skip_balanced_braces(source: &str, start: usize) -> usize {
    let bytes = source.as_bytes();
    let len = bytes.len();
    if start >= len || bytes[start] != b'{' {
        return start;
    }
    let mut i = start + 1;
    let mut depth = 1usize;
    let mut in_quote: Option<u8> = None;

    while i < len && depth > 0 {
        let b = bytes[i];
        if let Some(q) = in_quote {
            if b == b'\\' {
                i = (i + 2).min(len);
                continue;
            } else if b == q {
                in_quote = None;
            }
        } else {
            match b {
                b'"' | b'\'' | b'`' => {
                    in_quote = Some(b);
                }
                b'{' => {
                    depth += 1;
                }
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        i += 1;
                        return i;
                    }
                }
                _ => {}
            }
        }
        i += 1;
    }
    i
}

fn skip_balanced_parens(source: &str, start: usize) -> usize {
    let bytes = source.as_bytes();
    let len = bytes.len();
    if start >= len || bytes[start] != b'(' {
        return start;
    }
    let mut i = start + 1;
    let mut depth = 1usize;
    let mut in_quote: Option<u8> = None;

    while i < len && depth > 0 {
        let b = bytes[i];
        if let Some(q) = in_quote {
            if b == b'\\' {
                i = (i + 2).min(len);
                continue;
            } else if b == q {
                in_quote = None;
            }
        } else {
            match b {
                b'"' | b'\'' | b'`' => {
                    in_quote = Some(b);
                }
                b'(' => {
                    depth += 1;
                }
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        i += 1;
                        return i;
                    }
                }
                _ => {}
            }
        }
        i += 1;
    }
    i
}

fn find_closing_tag(source: &str, start: usize, name: &str) -> usize {
    let bytes = source.as_bytes();
    let name_bytes = name.as_bytes();
    let mut j = start;
    while j < bytes.len() {
        if bytes[j] == b'<' && j + 1 < bytes.len() && bytes[j + 1] == b'/' {
            let candidate_start = j + 2;
            if candidate_start + name_bytes.len() <= bytes.len() {
                let candidate = &bytes[candidate_start..candidate_start + name_bytes.len()];
                if candidate.eq_ignore_ascii_case(name_bytes) {
                    let after = candidate_start + name_bytes.len();
                    if after == bytes.len()
                        || bytes[after].is_ascii_whitespace()
                        || bytes[after] == b'>'
                        || bytes[after] == b'/'
                    {
                        return j;
                    }
                }
            }
        }
        j += 1;
    }
    bytes.len()
}

#[must_use]
pub fn lex(source: &str) -> Vec<Tag<'_>> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut tags = Vec::new();

    while i < len {
        if bytes[i..].starts_with(b"<!--") {
            i += 4;
            if let Some(pos) = source[i..].find("-->") {
                i += pos + 3;
            } else {
                i = len;
            }
        } else if bytes[i..].starts_with(b"<!") {
            i += 2;
            if let Some(pos) = source[i..].find('>') {
                i += pos + 1;
            } else {
                i = len;
            }
        } else if bytes[i..].starts_with(b"<?") {
            i += 2;
            if let Some(pos) = source[i..].find("?>") {
                i += pos + 2;
            } else {
                i = len;
            }
        } else if bytes[i..].starts_with(b"<%") {
            i += 2;
            if let Some(pos) = source[i..].find("%>") {
                i += pos + 2;
            } else {
                i = len;
            }
        } else if bytes[i..].starts_with(b"</") && i + 2 < len && bytes[i + 2].is_ascii_alphabetic()
        {
            i += 2;
            if let Some(pos) = source[i..].find('>') {
                i += pos + 1;
            } else {
                i = len;
            }
        } else if bytes[i] == b'<' && i + 1 < len && bytes[i + 1].is_ascii_alphabetic() {
            let tag_start = i;
            i += 1;
            let name_start = i;
            while i < len
                && !bytes[i].is_ascii_whitespace()
                && bytes[i] != b'/'
                && bytes[i] != b'>'
                && bytes[i] != b'<'
            {
                i += 1;
            }
            let tag_name = &source[name_start..i];
            let mut attributes = Vec::new();
            let mut closed = false;
            let mut tag_end = i;
            let mut self_closing = false;

            while i < len {
                let b = bytes[i];
                if b.is_ascii_whitespace() {
                    i += 1;
                } else if b == b'/' {
                    self_closing = true;
                    i += 1;
                } else if b == b'>' {
                    closed = true;
                    i += 1;
                    tag_end = i;
                    break;
                } else if bytes[i..].starts_with(b"<?") {
                    self_closing = false;
                    i += 2;
                    if let Some(pos) = source[i..].find("?>") {
                        i += pos + 2;
                    } else {
                        i = len;
                    }
                } else if bytes[i..].starts_with(b"<%") {
                    self_closing = false;
                    i += 2;
                    if let Some(pos) = source[i..].find("%>") {
                        i += pos + 2;
                    } else {
                        i = len;
                    }
                } else if b == b'<'
                    && i + 1 < len
                    && (bytes[i + 1].is_ascii_alphabetic()
                        || bytes[i + 1] == b'/'
                        || bytes[i + 1] == b'!')
                {
                    closed = false;
                    tag_end = i;
                    break;
                } else if b == b'{' {
                    self_closing = false;
                    i = skip_balanced_braces(source, i);
                } else if b == b'"'
                    || b == b'\''
                    || b == b'='
                    || b == b'('
                    || (b == b'<'/* unmatched < */)
                {
                    self_closing = false;
                    i += 1;
                } else {
                    self_closing = false;
                    let attr_name_start = i;
                    while i < len {
                        let c = bytes[i];
                        if c.is_ascii_whitespace()
                            || c == b'='
                            || c == b'>'
                            || c == b'/'
                            || c == b'<'
                            || c == b'"'
                            || c == b'\''
                            || c == b'('
                            || c == b'{'
                        {
                            break;
                        }
                        i += 1;
                    }
                    let attr_name = &source[attr_name_start..i];
                    let name_range = attr_name_start..i;

                    if i < len && bytes[i] == b'(' {
                        i = skip_balanced_parens(source, i);
                        let attr_end = i;
                        attributes.push(Attribute {
                            name: attr_name,
                            name_range,
                            value: None,
                            end: attr_end,
                        });
                    } else {
                        let mut peek = i;
                        while peek < len && bytes[peek].is_ascii_whitespace() {
                            peek += 1;
                        }
                        if peek < len && bytes[peek] == b'=' {
                            i = peek + 1; // consume '='
                            let eq_end = i;
                            let mut val_peek = i;
                            while val_peek < len && bytes[val_peek].is_ascii_whitespace() {
                                val_peek += 1;
                            }

                            let is_cut = val_peek == len
                                || bytes[val_peek] == b'>'
                                || (bytes[val_peek] == b'<'
                                    && (val_peek + 1 == len
                                        || bytes[val_peek + 1].is_ascii_alphabetic()
                                        || bytes[val_peek + 1] == b'/'
                                        || bytes[val_peek + 1] == b'!'));

                            if is_cut {
                                let val_range = eq_end..eq_end;
                                attributes.push(Attribute {
                                    name: attr_name,
                                    name_range,
                                    value: Some(AttrValue {
                                        text: "",
                                        range: val_range,
                                        quote: Quote::Unquoted,
                                    }),
                                    end: eq_end,
                                });
                                i = val_peek;
                            } else if bytes[val_peek] == b'"' || bytes[val_peek] == b'\'' {
                                i = val_peek;
                                let quote_char = bytes[i];
                                let quote_kind = if quote_char == b'"' {
                                    Quote::Double
                                } else {
                                    Quote::Single
                                };
                                i += 1;
                                let text_start = i;
                                while i < len {
                                    if bytes[i..].starts_with(b"{{") {
                                        i += 2;
                                        if let Some(pos) = source[i..].find("}}") {
                                            i += pos + 2;
                                        } else {
                                            i = len;
                                        }
                                    } else if bytes[i..].starts_with(b"{%") {
                                        i += 2;
                                        if let Some(pos) = source[i..].find("%}") {
                                            i += pos + 2;
                                        } else {
                                            i = len;
                                        }
                                    } else if bytes[i..].starts_with(b"<%") {
                                        i += 2;
                                        if let Some(pos) = source[i..].find("%>") {
                                            i += pos + 2;
                                        } else {
                                            i = len;
                                        }
                                    } else if bytes[i..].starts_with(b"<?") {
                                        i += 2;
                                        if let Some(pos) = source[i..].find("?>") {
                                            i += pos + 2;
                                        } else {
                                            i = len;
                                        }
                                    } else if bytes[i] == quote_char {
                                        let text_end = i;
                                        i += 1; // consume closing quote
                                        let val_range = text_start..text_end;
                                        let attr_end = i;
                                        attributes.push(Attribute {
                                            name: attr_name,
                                            name_range: name_range.clone(),
                                            value: Some(AttrValue {
                                                text: &source[val_range.clone()],
                                                range: val_range,
                                                quote: quote_kind,
                                            }),
                                            end: attr_end,
                                        });
                                        break;
                                    } else {
                                        i += 1;
                                    }
                                }
                                if i == len
                                    && !attributes.iter().any(|a| a.name_range == name_range)
                                {
                                    let text_end = len;
                                    let val_range = text_start..text_end;
                                    attributes.push(Attribute {
                                        name: attr_name,
                                        name_range,
                                        value: Some(AttrValue {
                                            text: &source[val_range.clone()],
                                            range: val_range,
                                            quote: quote_kind,
                                        }),
                                        end: len,
                                    });
                                }
                            } else if bytes[val_peek] == b'{' {
                                i = val_peek;
                                let text_start = i + 1;
                                let after_brace = skip_balanced_braces(source, i);
                                let (text_end, attr_end) = if after_brace > 0
                                    && after_brace <= len
                                    && bytes[after_brace - 1] == b'}'
                                {
                                    (after_brace - 1, after_brace)
                                } else {
                                    (len, len)
                                };
                                i = after_brace;
                                let val_range = text_start..text_end;
                                attributes.push(Attribute {
                                    name: attr_name,
                                    name_range,
                                    value: Some(AttrValue {
                                        text: &source[val_range.clone()],
                                        range: val_range,
                                        quote: Quote::Brace,
                                    }),
                                    end: attr_end,
                                });
                            } else {
                                i = val_peek;
                                let text_start = i;
                                while i < len && !bytes[i].is_ascii_whitespace() && bytes[i] != b'>'
                                {
                                    i += 1;
                                }
                                let text_end = i;
                                let val_range = text_start..text_end;
                                attributes.push(Attribute {
                                    name: attr_name,
                                    name_range,
                                    value: Some(AttrValue {
                                        text: &source[val_range.clone()],
                                        range: val_range,
                                        quote: Quote::Unquoted,
                                    }),
                                    end: text_end,
                                });
                            }
                        } else {
                            attributes.push(Attribute {
                                name: attr_name,
                                name_range: name_range.clone(),
                                value: None,
                                end: name_range.end,
                            });
                            i = peek;
                        }
                    }
                }
            }

            if !closed && tag_end < i {
                tag_end = i;
            }

            tags.push(Tag {
                name: tag_name,
                start: tag_start,
                end: tag_end,
                closed,
                attributes,
            });

            if closed && !self_closing {
                let is_raw = tag_name.eq_ignore_ascii_case("script")
                    || tag_name.eq_ignore_ascii_case("style")
                    || tag_name.eq_ignore_ascii_case("textarea")
                    || tag_name.eq_ignore_ascii_case("title");
                if is_raw {
                    i = find_closing_tag(source, i, tag_name);
                }
            }
        } else {
            i += 1;
        }
    }

    tags
}
