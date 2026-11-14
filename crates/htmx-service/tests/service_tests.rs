use htmx_service::HtmxLanguageService;
use htmx_vfs::Document;
use lsp_types::{NumberOrString, Position, Range, Uri};
use std::str::FromStr;

#[test]
fn test_attribute_completion() {
    let service = HtmxLanguageService::new();
    let uri = Uri::from_str("file:///test.html").unwrap();
    let html = r#"<button hx- >Click</button>"#;
    // Cursor right after "hx-" at character 11
    let doc = Document::new(uri, 1, html);
    let items = service.complete(
        &doc,
        Position {
            line: 0,
            character: 11,
        },
    );

    assert!(!items.is_empty());
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"hx-get"));
    assert!(labels.contains(&"hx-post"));
    assert!(labels.contains(&"hx-swap"));
    assert!(labels.contains(&"hx-target"));
}

#[test]
fn test_swap_style_completion() {
    let service = HtmxLanguageService::new();
    let uri = Uri::from_str("file:///test.html").unwrap();
    let html = r#"<button hx-swap="">Click</button>"#;
    // Cursor between quotes at character 17
    let doc = Document::new(uri, 1, html);
    let items = service.complete(
        &doc,
        Position {
            line: 0,
            character: 17,
        },
    );

    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"innerHTML"));
    assert!(labels.contains(&"outerHTML"));
    assert!(labels.contains(&"beforeend"));
}

#[test]
fn test_target_completion_with_document_ids() {
    let service = HtmxLanguageService::new();
    let uri = Uri::from_str("file:///test.html").unwrap();
    let html = r##"
        <div id="target-container"></div>
        <button hx-target="">Click</button>
    "##;
    // Position inside hx-target quotes
    let doc = Document::new(uri, 1, html);
    let target_offset = html.find("hx-target=\"\"").unwrap() + 11;
    let pos = doc.offset_to_position(target_offset).unwrap();
    let items = service.complete(&doc, pos);

    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"#target-container"));
    assert!(labels.contains(&"this"));
    assert!(labels.contains(&"closest"));
}

#[test]
fn test_hover_attribute_doc() {
    let service = HtmxLanguageService::new();
    let uri = Uri::from_str("file:///test.html").unwrap();
    let html = r#"<button hx-get="/api">Click</button>"#;
    let doc = Document::new(uri, 1, html);
    let hover = service
        .hover(
            &doc,
            Position {
                line: 0,
                character: 10,
            },
        )
        .expect("Hover should exist");

    match hover.contents {
        lsp_types::HoverContents::Markup(m) => {
            assert!(m.value.contains("hx-get"));
            assert!(m.value.contains("Issues a GET request"));
            assert!(m.value.contains("https://htmx.org/attributes/hx-get/"));
        }
        _ => panic!("Expected Markup hover contents"),
    }
}

#[test]
fn test_diagnostics_all_rules() {
    let service = HtmxLanguageService::new();
    let uri = Uri::from_str("file:///test.html").unwrap();
    let html = r##"
        <div id="existing-id">
            <!-- HTMX001: deprecated hx-ws -->
            <button hx-ws="connect:/chat">WS</button>

            <!-- HTMX002: invalid swap style -->
            <button hx-swap="badSwapStyle">Swap</button>

            <!-- HTMX003: invalid trigger duration -->
            <button hx-trigger="click delay:notATime">Trigger</button>

            <!-- HTMX004: missing target ID -->
            <button hx-target="#ghost-id">Target</button>
        </div>
    "##;
    let doc = Document::new(uri, 1, html);
    let diagnostics = service.validate(&doc);

    let codes: Vec<String> = diagnostics
        .iter()
        .filter_map(|d| {
            d.code.as_ref().map(|c| match c {
                NumberOrString::String(s) => s.clone(),
                NumberOrString::Number(n) => n.to_string(),
            })
        })
        .collect();

    assert!(
        codes.contains(&"HTMX001".to_string()),
        "Expected HTMX001 for deprecated hx-ws"
    );
    assert!(
        codes.contains(&"HTMX002".to_string()),
        "Expected HTMX002 for invalid swap style"
    );
    assert!(
        codes.contains(&"HTMX003".to_string()),
        "Expected HTMX003 for invalid trigger duration"
    );
    assert!(
        codes.contains(&"HTMX004".to_string()),
        "Expected HTMX004 for missing target ID"
    );
}

#[test]
fn test_code_action_quick_fix() {
    let service = HtmxLanguageService::new();
    let uri = Uri::from_str("file:///test.html").unwrap();
    let html = r#"<button hx-ws="connect:/chat">WS</button>"#;
    let doc = Document::new(uri.clone(), 1, html);
    let diags = service.validate(&doc);

    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].code.as_ref(),
        Some(&NumberOrString::String("HTMX001".to_string()))
    );

    let actions = service.code_actions(
        &doc,
        Range {
            start: Position {
                line: 0,
                character: 8,
            },
            end: Position {
                line: 0,
                character: 13,
            },
        },
        &diags,
    );

    assert_eq!(actions.len(), 1);
    match &actions[0] {
        lsp_types::CodeActionOrCommand::CodeAction(ca) => {
            assert!(ca.title.contains("Migrate hx-ws"));
            let edits = ca
                .edit
                .as_ref()
                .unwrap()
                .changes
                .as_ref()
                .unwrap()
                .get(&uri)
                .unwrap();
            assert_eq!(edits[0].new_text, "hx-ext=\"ws\" ws-connect=\"/chat\"");
        }
        _ => panic!("Expected CodeAction"),
    }
}

#[test]
fn test_text_content_swap_style() {
    let service = HtmxLanguageService::new();
    let uri = Uri::from_str("file:///test.html").unwrap();
    let html = r#"<button hx-swap="textContent">Click</button>"#;
    let doc = Document::new(uri, 1, html);
    let diags = service.validate(&doc);
    assert!(
        diags.is_empty(),
        "textContent should be recognized as a valid swap style"
    );
}

#[test]
fn test_sse_quick_fix() {
    let service = HtmxLanguageService::new();
    let uri = Uri::from_str("file:///test.html").unwrap();
    let html = r#"<button hx-sse="connect:/events">SSE</button>"#;
    let doc = Document::new(uri.clone(), 1, html);
    let diags = service.validate(&doc);
    assert_eq!(diags.len(), 1);

    let actions = service.code_actions(&doc, diags[0].range, &diags);
    assert_eq!(actions.len(), 1);
    if let lsp_types::CodeActionOrCommand::CodeAction(ca) = &actions[0] {
        assert!(ca.title.contains("Migrate hx-sse"));
        let edits = ca
            .edit
            .as_ref()
            .unwrap()
            .changes
            .as_ref()
            .unwrap()
            .get(&uri)
            .unwrap();
        assert_eq!(edits[0].new_text, "hx-ext=\"sse\" sse-connect=\"/events\"");
    }
}

#[test]
fn test_hx_on_event_not_deprecated() {
    let service = HtmxLanguageService::new();
    let uri = Uri::from_str("file:///test.html").unwrap();
    let html = r#"<button hx-on:click="doSomething()">Click</button>"#;
    let doc = Document::new(uri.clone(), 1, html);
    let diags = service.validate(&doc);
    assert!(
        diags.is_empty(),
        "hx-on:click must not trigger deprecation diagnostics"
    );

    // Bare hx-on MUST trigger HTMX001
    let bare_html = r#"<button hx-on="click: doSomething()">Click</button>"#;
    let bare_doc = Document::new(uri.clone(), 1, bare_html);
    let bare_diags = service.validate(&bare_doc);
    assert_eq!(bare_diags.len(), 1);
    assert_eq!(
        bare_diags[0].code.as_ref(),
        Some(&NumberOrString::String("HTMX001".to_string()))
    );

    // Hover on hx-on:click should not show deprecated
    let hover = service
        .hover(
            &doc,
            Position {
                line: 0,
                character: 11,
            },
        )
        .expect("Hover should exist");
    if let lsp_types::HoverContents::Markup(m) = hover.contents {
        assert!(
            !m.value.contains("Deprecated"),
            "hx-on:click hover must not contain deprecation warning"
        );
    }
}

#[test]
fn test_hx_on_completion() {
    let service = HtmxLanguageService::new();
    let uri = Uri::from_str("file:///test.html").unwrap();
    let html = r#"<button hx-on:c >Click</button>"#;
    let doc = Document::new(uri, 1, html);
    let items = service.complete(
        &doc,
        Position {
            line: 0,
            character: 15,
        },
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"hx-on:click"));
    assert!(labels.contains(&"hx-on:change"));
    for item in &items {
        assert_eq!(
            item.deprecated,
            Some(false),
            "hx-on event completions must not be marked deprecated"
        );
    }
}

#[test]
fn test_modifier_hover_exact_match() {
    let service = HtmxLanguageService::new();
    let uri = Uri::from_str("file:///test.html").unwrap();
    // once-custom is a custom event name, should not match 'once' modifier
    let html = r#"<button hx-trigger="once-custom">Click</button>"#;
    let doc = Document::new(uri, 1, html);
    let hover = service.hover(
        &doc,
        Position {
            line: 0,
            character: 22,
        },
    );
    assert!(
        hover.is_none(),
        "once-custom should not match 'once' modifier docs"
    );
}

#[test]
fn test_sse_multi_part_quick_fix() {
    let service = HtmxLanguageService::new();
    let uri = Uri::from_str("file:///test.html").unwrap();
    let html = r#"<div hx-sse="connect:/events swap:msg">SSE</div>"#;
    let doc = Document::new(uri.clone(), 1, html);
    let diags = service.validate(&doc);
    assert_eq!(diags.len(), 1);

    let actions = service.code_actions(&doc, diags[0].range, &diags);
    assert_eq!(actions.len(), 1);
    if let lsp_types::CodeActionOrCommand::CodeAction(ca) = &actions[0] {
        let edits = ca
            .edit
            .as_ref()
            .unwrap()
            .changes
            .as_ref()
            .unwrap()
            .get(&uri)
            .unwrap();
        assert_eq!(
            edits[0].new_text,
            "hx-ext=\"sse\" sse-connect=\"/events\" sse-swap=\"msg\""
        );
    }
}

#[test]
fn test_unknown_hx_ws_value_returns_no_quick_fix() {
    let service = HtmxLanguageService::new();
    let uri = Uri::from_str("file:///test.html").unwrap();
    let html = r#"<button hx-ws="unknown_format">WS</button>"#;
    let doc = Document::new(uri, 1, html);
    let diags = service.validate(&doc);
    assert_eq!(diags.len(), 1);

    let actions = service.code_actions(&doc, diags[0].range, &diags);
    assert!(
        actions.is_empty(),
        "Unrecognized hx-ws value should not produce quick fix"
    );
}
