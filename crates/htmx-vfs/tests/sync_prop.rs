use htmx_vfs::Document;
use lsp_types::{Position, Range, TextDocumentContentChangeEvent, Uri};
use proptest::prelude::*;
use std::str::FromStr;

fn pos_to_byte_offset(s: &str, pos: Position) -> usize {
    let mut current_line = 0;

    let chars: Vec<(usize, char)> = s.char_indices().collect();
    let mut i = 0;
    while i < chars.len() {
        if current_line == pos.line {
            break;
        }
        let (_, c) = chars[i];
        if c == '\n' {
            current_line += 1;
        }
        i += 1;
    }

    if current_line < pos.line {
        return s.len();
    }

    // Now find the character (UTF-16 offset) within this line
    let mut utf16_col = 0;
    let mut j = i;
    while j < chars.len() {
        let (byte_idx, c) = chars[j];
        if c == '\r' || c == '\n' {
            return byte_idx;
        }
        let next_cu = utf16_col + c.len_utf16() as u32;
        if pos.character < next_cu {
            return byte_idx;
        }
        utf16_col = next_cu;
        j += 1;
    }

    s.len()
}

fn apply_to_string_model(model: &mut String, change: &TextDocumentContentChangeEvent) {
    if let Some(range) = &change.range {
        let start_off = pos_to_byte_offset(model, range.start);
        let end_off = pos_to_byte_offset(model, range.end);
        let (min_off, max_off) = if start_off <= end_off {
            (start_off, end_off)
        } else {
            (end_off, start_off)
        };
        model.replace_range(min_off..max_off, &change.text);
    } else {
        *model = change.text.clone();
    }
}

// Generate characters including ASCII, é, 😀, and newlines \r\n
fn prop_char() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("a".to_string()),
        Just("b".to_string()),
        Just(" ".to_string()),
        Just("é".to_string()),
        Just("😀".to_string()),
        Just("\n".to_string()),
        Just("\r\n".to_string()),
        Just("<div id=\"x\">".to_string()),
        Just("</div>".to_string()),
    ]
}

#[derive(Debug, Clone)]
struct EditOp {
    line: u32,
    col: u32,
    del_chars: u32,
    insert: String,
}

fn prop_edit() -> impl Strategy<Value = EditOp> {
    (
        0u32..10,
        0u32..20,
        0u32..5,
        prop::collection::vec(prop_char(), 0..3),
    )
        .prop_map(|(line, col, del_chars, ins_parts)| EditOp {
            line,
            col,
            del_chars,
            insert: ins_parts.join(""),
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]
    #[test]
    fn incremental_edits_match_string_model(
        initial_parts in prop::collection::vec(prop_char(), 1..10),
        edits in prop::collection::vec(prop_edit(), 1..50),
    ) {
        let initial_text = initial_parts.join("");
        let uri = Uri::from_str("file:///prop_test.html").unwrap();
        let mut doc = Document::new(uri, 1, &initial_text);
        let mut model = initial_text;

        for (step, edit) in edits.into_iter().enumerate() {
            let start = Position { line: edit.line, character: edit.col };
            let end = Position { line: edit.line, character: edit.col + edit.del_chars };
            let change = TextDocumentContentChangeEvent {
                range: Some(Range { start, end }),
                range_length: None,
                text: edit.insert,
            };

            apply_to_string_model(&mut model, &change);
            doc.apply_changes(vec![change], (step + 2) as i32);

            prop_assert_eq!(&doc.text(), &model, "Mismatch at step {}", step);
        }
    }
}
