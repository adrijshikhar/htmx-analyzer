use htmx_service::{find_references, goto_definition, WorkspaceIdIndex};
use htmx_vfs::Document;
use lsp_types::{GotoDefinitionResponse, Position, Uri};

#[test]
fn test_goto_definition_same_file() {
    let uri: Uri = "file:///workspace/index.html".parse().unwrap();
    let html = r##"<div id="results">Initial content</div>
<button hx-target="#results">Click</button>"##;
    let doc = Document::new(uri.clone(), 1, html);
    let mut index = WorkspaceIdIndex::new();
    index.update_file(&uri, html);

    // Cursor on "results" inside hx-target="#results" (line 1)
    let button_line = 1;
    let col = html.lines().nth(button_line).unwrap().find("results").unwrap() as u32 + 2;
    let pos = Position::new(button_line as u32, col);

    let res = goto_definition(&uri, pos, &doc, &index, false).expect("definition found");
    match res {
        GotoDefinitionResponse::Scalar(loc) => {
            assert_eq!(loc.uri, uri);
            assert_eq!(loc.range.start.line, 0);
        }
        _ => panic!("Expected scalar location"),
    }
}

#[test]
fn test_goto_definition_cross_file_partial() {
    let base_uri: Uri = "file:///workspace/base.html".parse().unwrap();
    let partial_uri: Uri = "file:///workspace/partials/item.html".parse().unwrap();

    let base_html = r##"<div id="cart-counter">0</div>"##;
    let partial_html = r##"<button hx-target="#cart-counter">Add</button>"##;

    let _base_doc = Document::new(base_uri.clone(), 1, base_html);
    let partial_doc = Document::new(partial_uri.clone(), 1, partial_html);

    let mut index = WorkspaceIdIndex::new();
    index.update_file(&base_uri, base_html);
    index.update_file(&partial_uri, partial_html);

    let pos = Position::new(0, partial_html.find("cart-counter").unwrap() as u32 + 2);
    let res = goto_definition(&partial_uri, pos, &partial_doc, &index, false)
        .expect("cross-file definition found");
    match res {
        GotoDefinitionResponse::Scalar(loc) => {
            assert_eq!(loc.uri, base_uri);
            assert_eq!(loc.range.start.line, 0);
        }
        _ => panic!("Expected scalar location pointing to base.html"),
    }
}

#[test]
fn test_goto_definition_location_link_support() {
    let uri: Uri = "file:///workspace/index.html".parse().unwrap();
    let html = r##"<div id="results">Initial content</div>
<button hx-target="#results">Click</button>"##;
    let doc = Document::new(uri.clone(), 1, html);
    let mut index = WorkspaceIdIndex::new();
    index.update_file(&uri, html);

    let button_line = 1;
    let col = html.lines().nth(button_line).unwrap().find("results").unwrap() as u32 + 2;
    let pos = Position::new(button_line as u32, col);

    let res = goto_definition(&uri, pos, &doc, &index, true).expect("definition link found");
    match res {
        GotoDefinitionResponse::Link(links) => {
            assert_eq!(links.len(), 1);
            assert_eq!(links[0].target_uri, uri);
            assert!(links[0].origin_selection_range.is_some());
        }
        _ => panic!("Expected location links"),
    }
}

#[test]
fn test_find_references_bidirectional() {
    let uri: Uri = "file:///workspace/page.html".parse().unwrap();
    let html = r##"<div id="target-box">Box</div>
<button hx-target="#target-box">Target 1</button>
<button hx-indicator="#target-box">Target 2</button>"##;
    let doc = Document::new(uri.clone(), 1, html);
    let mut index = WorkspaceIdIndex::new();
    index.update_file(&uri, html);

    // Cursor on <div id="target-box"> (line 0)
    let pos = Position::new(0, html.find("target-box").unwrap() as u32 + 2);
    let refs = find_references(&uri, pos, &doc, &[&doc], &index, true);

    // Declaration + 2 references = 3
    assert_eq!(refs.len(), 3);
}

#[test]
fn test_find_references_exclude_declaration() {
    let uri: Uri = "file:///workspace/page.html".parse().unwrap();
    let html = r##"<div id="target-box">Box</div>
<button hx-target="#target-box">Target 1</button>
<button hx-indicator="#target-box">Target 2</button>"##;
    let doc = Document::new(uri.clone(), 1, html);
    let mut index = WorkspaceIdIndex::new();
    index.update_file(&uri, html);

    // Cursor on hx-target="#target-box" (line 1)
    let pos = Position::new(
        1,
        html.lines().nth(1).unwrap().find("target-box").unwrap() as u32 + 2,
    );
    let refs = find_references(&uri, pos, &doc, &[&doc], &index, false);

    // 2 references (excluding declaration)
    assert_eq!(refs.len(), 2);
}

#[test]
fn test_diagnostics_suppressed_by_workspace_index() {
    let partial_uri: Uri = "file:///workspace/partials/button.html".parse().unwrap();
    let partial_html = r##"<button hx-target="#cart-count">Add to Cart</button>"##;
    let partial_doc = Document::new(partial_uri.clone(), 1, partial_html);

    let service = htmx_service::HtmxLanguageService::new();

    // Without workspace index: HTMX004 is emitted because #cart-count is missing in this file
    let diags_without = service.validate(&partial_doc);
    assert!(
        diags_without
            .iter()
            .any(|d| d.code.as_ref().map(|c| match c {
                lsp_types::NumberOrString::String(s) => s == "HTMX004",
                _ => false,
            }).unwrap_or(false)),
        "Expected HTMX004 without workspace index"
    );

    // With workspace index containing #cart-count from base.html
    let base_uri: Uri = "file:///workspace/base.html".parse().unwrap();
    let mut index = WorkspaceIdIndex::new();
    index.update_file(&base_uri, r##"<span id="cart-count">0</span>"##);

    let diags_with = service.validate_with_index(&partial_doc, Some(&index));
    let has_htmx004 = diags_with
        .iter()
        .any(|d| d.code.as_ref().map(|c| match c {
            lsp_types::NumberOrString::String(s) => s == "HTMX004",
            _ => false,
        }).unwrap_or(false));
    assert!(!has_htmx004, "HTMX004 should be suppressed when target ID is defined in workspace index");
}
