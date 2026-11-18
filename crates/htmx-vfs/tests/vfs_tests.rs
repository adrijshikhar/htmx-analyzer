use htmx_vfs::*;
use lsp_types::{Position, Range, TextDocumentContentChangeEvent, Uri};
use std::str::FromStr;

#[test]
fn test_document_open_and_text() {
    let uri = Uri::from_str("file:///workspace/test.html").unwrap();
    let text = "<button hx-get=\"/api\">Click</button>";
    let doc = Document::new(uri.clone(), 1, text);

    assert_eq!(doc.text(), text);
    assert_eq!(doc.version, 1);
}

#[test]
fn test_position_to_offset_ascii() {
    let uri = Uri::from_str("file:///workspace/test.html").unwrap();
    let text = "Line 0\nLine 1\nLine 2";
    let doc = Document::new(uri, 1, text);

    // Line 0, character 0 -> offset 0
    assert_eq!(
        doc.position_to_offset(Position {
            line: 0,
            character: 0
        }),
        Some(0)
    );
    // Line 1, character 0 -> offset 7
    assert_eq!(
        doc.position_to_offset(Position {
            line: 1,
            character: 0
        }),
        Some(7)
    );
    // Line 1, character 5 -> offset 12
    assert_eq!(
        doc.position_to_offset(Position {
            line: 1,
            character: 5
        }),
        Some(12)
    );
}

#[test]
fn test_out_of_range_character_clamped() {
    let uri = Uri::from_str("file:///workspace/clamp.html").unwrap();
    let text = "short\nsecond line";
    let doc = Document::new(uri, 1, text);

    // character 999 on line 0 must clamp to end of "short" (offset 5), before the '\n'
    let offset = doc.position_to_offset(Position {
        line: 0,
        character: 999,
    });
    assert_eq!(offset, Some(5));
}

#[test]
fn test_utf16_surrogate_pairs() {
    let uri = Uri::from_str("file:///workspace/emoji.html").unwrap();
    // '😀' is 4 bytes in UTF-8, 2 code units in UTF-16.
    let text = "<div>😀 Hello</div>";
    let doc = Document::new(uri, 1, text);

    // Offset of 'H' in "Hello":
    // "<div>" is 5 chars / UTF-16 code units (0..5)
    // "😀" is 1 char, 2 UTF-16 code units (5, 6)
    // " " is 1 char, 1 UTF-16 code unit (7)
    // So character 8 starts "Hello" (UTF-8 byte offset 10)
    let offset = doc
        .position_to_offset(Position {
            line: 0,
            character: 8,
        })
        .unwrap();
    assert_eq!(&text[offset..offset + 5], "Hello");

    // Convert back from byte offset to LSP position
    let pos = doc.offset_to_position(offset).unwrap();
    assert_eq!(pos.character, 8);
}

#[test]
fn test_incremental_edit() {
    let uri = Uri::from_str("file:///workspace/test.html").unwrap();
    let mut doc = Document::new(uri, 1, "<button hx-get=\"/\">Click</button>");

    // Replace "/" with "/api/data"
    // "/" is at line 0, char 16 to 17
    let change = TextDocumentContentChangeEvent {
        range: Some(Range {
            start: Position {
                line: 0,
                character: 16,
            },
            end: Position {
                line: 0,
                character: 17,
            },
        }),
        range_length: None,
        text: "/api/data".to_string(),
    };

    doc.apply_changes(vec![change], 2);
    assert_eq!(doc.text(), "<button hx-get=\"/api/data\">Click</button>");
    assert_eq!(doc.version, 2);
}

#[test]
fn test_document_store() {
    let mut store = DocumentStore::new();
    let uri = Uri::from_str("file:///test.html").unwrap();

    assert!(store.is_empty());
    store.open(uri.clone(), 1, "<div></div>");
    assert_eq!(store.len(), 1);

    store.change(
        &uri,
        2,
        vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "<span></span>".to_string(),
        }],
    );

    assert_eq!(store.get(&uri).unwrap().text(), "<span></span>");
    assert_eq!(store.get(&uri).unwrap().version, 2);

    store.close(&uri);
    assert!(store.is_empty());
}

#[test]
fn document_store_lists_uris() {
    let mut store = DocumentStore::new();
    let uri1 = Uri::from_str("file:///test1.html").unwrap();
    let uri2 = Uri::from_str("file:///test2.html").unwrap();
    store.open(uri1.clone(), 1, "test1");
    store.open(uri2.clone(), 1, "test2");
    let uris: Vec<&Uri> = store.uris().collect();
    assert_eq!(uris.len(), 2);
    assert!(uris.contains(&&uri1));
    assert!(uris.contains(&&uri2));
}
