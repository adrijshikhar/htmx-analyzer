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
    let items = service
        .complete(
            &doc,
            Position {
                line: 0,
                character: 11,
            },
        )
        .items;

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
    let items = service
        .complete(
            &doc,
            Position {
                line: 0,
                character: 17,
            },
        )
        .items;

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
    let items = service.complete(&doc, pos).items;

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
    let items = service
        .complete(
            &doc,
            Position {
                line: 0,
                character: 15,
            },
        )
        .items;
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

#[test]
fn hover_inside_templ_tag() {
    let service = HtmxLanguageService::new();
    let sample = "package v\ntempl Row(i Item) {\n\t<div id={ i.ID } hx-get={ \"/i/\" + i.ID } hx-swap=\"bogus\" hx-target=\"this\">{ i.Name }</div>\n}";
    let doc = Document::new(Uri::from_str("file:///test.html").unwrap(), 1, sample);
    let offset = sample.find("hx-target").unwrap() + 2;
    let pos = doc.offset_to_position(offset).unwrap();
    let hover = service.hover(&doc, pos).expect("Hover should exist");
    match hover.contents {
        lsp_types::HoverContents::Markup(m) => {
            assert!(m.value.contains("hx-target"));
        }
        _ => panic!("Expected Markup hover contents"),
    }
}

#[test]
fn target_completion_capped() {
    let service = HtmxLanguageService::new();
    let mut html = String::new();
    for i in 0..500 {
        html.push_str(&format!("<span id=\"id-{:03}\"></span>", i));
    }
    html.push_str("<b hx-target=\"#\">");
    let doc = Document::new(Uri::from_str("file:///test.html").unwrap(), 1, &html);
    let offset = html.rfind('#').unwrap() + 1;
    let pos = doc.offset_to_position(offset).unwrap();
    let list = service.complete(&doc, pos);
    assert_eq!(list.items.len(), 200);
    assert!(list.is_incomplete);
    assert_eq!(list.items[0].label, "#id-000");
}

#[test]
fn uppercase_attribute_validated() {
    let service = HtmxLanguageService::new();
    let doc = Document::new(
        Uri::from_str("file:///test.html").unwrap(),
        1,
        "<b HX-SWAP=\"bogus\">x</b>",
    );
    let diags = service.validate(&doc);
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].code,
        Some(lsp_types::NumberOrString::String("HTMX002".to_string()))
    );
}

#[test]
fn swap_timing_validated() {
    let service = HtmxLanguageService::new();
    let doc = Document::new(
        Uri::from_str("file:///test.html").unwrap(),
        1,
        "<b hx-swap=\"innerHTML swap:abc settle:1s\">",
    );
    let diags = service.validate(&doc);
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].code,
        Some(lsp_types::NumberOrString::String("HTMX002".to_string()))
    );
    assert!(diags[0].message.contains("'abc'"));
}

#[test]
fn polling_interval_validated() {
    let service = HtmxLanguageService::new();
    let doc_bad = Document::new(
        Uri::from_str("file:///test.html").unwrap(),
        1,
        "<b hx-trigger=\"every abc\">",
    );
    let diags_bad = service.validate(&doc_bad);
    assert_eq!(diags_bad.len(), 1);
    assert_eq!(
        diags_bad[0].code,
        Some(lsp_types::NumberOrString::String("HTMX003".to_string()))
    );
    assert!(diags_bad[0].message.contains("'abc'"));

    let doc_ok = Document::new(
        Uri::from_str("file:///test.html").unwrap(),
        1,
        "<b hx-trigger=\"every 2s\">",
    );
    let diags_ok = service.validate(&doc_ok);
    assert_eq!(diags_ok.len(), 0);
}

fn apply_edits(text: &str, edits: &[lsp_types::TextEdit]) -> String {
    let doc = Document::new(Uri::from_str("file:///test.html").unwrap(), 1, text);
    let mut sorted_edits = edits.to_vec();
    sorted_edits.sort_by(|a, b| {
        let a_start = doc.position_to_offset(a.range.start).unwrap();
        let b_start = doc.position_to_offset(b.range.start).unwrap();
        b_start.cmp(&a_start)
    });

    let mut result = text.to_string();
    for edit in sorted_edits {
        let start = doc.position_to_offset(edit.range.start).unwrap();
        let end = doc.position_to_offset(edit.range.end).unwrap();
        result.replace_range(start..end, &edit.new_text);
    }
    result
}

#[allow(clippy::mutable_key_type)]
fn run_quick_fix(html: &str) -> Option<String> {
    let service = HtmxLanguageService::new();
    let uri = Uri::from_str("file:///test.html").unwrap();
    let doc = Document::new(uri.clone(), 1, html);
    let diags = service.validate(&doc);
    let htmx001 = diags
        .iter()
        .find(|d| d.code == Some(lsp_types::NumberOrString::String("HTMX001".to_string())))?;
    let actions = service.code_actions(&doc, htmx001.range, &diags);
    if actions.is_empty() {
        return None;
    }
    match &actions[0] {
        lsp_types::CodeActionOrCommand::CodeAction(ca) => {
            let changes = ca.edit.as_ref()?.changes.as_ref()?;
            let edits = changes.get(&uri)?;
            Some(apply_edits(html, edits))
        }
        _ => None,
    }
}

#[test]
fn quick_fix_merges_existing_hx_ext() {
    let html = r#"<div hx-ext="json-enc" hx-ws="connect:/chat send">x</div>"#;
    let fixed = run_quick_fix(html).expect("expected quick fix");
    assert_eq!(
        fixed,
        r#"<div hx-ext="json-enc, ws" ws-connect="/chat" ws-send>x</div>"#
    );
}

#[test]
fn quick_fix_skips_ext_already_listed() {
    let html = r#"<div hx-ext="ws" hx-ws="connect:/c">x</div>"#;
    let fixed = run_quick_fix(html).expect("expected quick fix");
    assert_eq!(fixed, r#"<div hx-ext="ws" ws-connect="/c">x</div>"#);
}

#[test]
fn quick_fix_escapes_quotes() {
    let html = r#"<div hx-ws='connect:/a?q="x"'>x</div>"#;
    let fixed = run_quick_fix(html).expect("expected quick fix");
    assert_eq!(
        fixed,
        r#"<div hx-ext="ws" ws-connect="/a?q=&quot;x&quot;">x</div>"#
    );
}

#[test]
fn quick_fix_preserves_ampersands() {
    let html = r#"<div hx-ws="connect:/a?x=1&amp;y=2">x</div>"#;
    let fixed = run_quick_fix(html).expect("expected quick fix");
    assert_eq!(
        fixed,
        r#"<div hx-ext="ws" ws-connect="/a?x=1&amp;y=2">x</div>"#
    );
}

#[test]
fn quick_fix_rewrites_unquoted_hx_ext() {
    let html = r#"<div hx-ext=json-enc hx-ws="connect:/c">x</div>"#;
    let fixed = run_quick_fix(html).expect("expected quick fix");
    assert_eq!(
        fixed,
        r#"<div hx-ext="json-enc, ws" ws-connect="/c">x</div>"#
    );
}

#[test]
fn quick_fix_send_only_keeps_hx_ext() {
    let html = r#"<div hx-ext="json-enc" hx-ws="send">x</div>"#;
    let fixed = run_quick_fix(html).expect("expected quick fix");
    assert_eq!(fixed, r#"<div hx-ext="json-enc" ws-send>x</div>"#);
}

#[test]
fn quick_fix_skips_dynamic_values() {
    let html1 = r#"<div hx-ext="{{ e }}" hx-ws="connect:/c">"#;
    assert_eq!(run_quick_fix(html1), None);

    let html2 = r#"<div hx-ws="connect:{{ u }}">"#;
    assert_eq!(run_quick_fix(html2), None);
}

#[test]
fn quick_fix_handles_data_prefix() {
    let html = r#"<div data-hx-ext="a" data-hx-ws="connect:/c">x</div>"#;
    let fixed = run_quick_fix(html).expect("expected quick fix");
    assert_eq!(fixed, r#"<div data-hx-ext="a, ws" ws-connect="/c">x</div>"#);
}

#[test]
fn quick_fix_with_crlf_line_endings() {
    let html = "<p>\r\n<div hx-sse=\"connect:/e swap:m\">x</div>";
    let fixed = run_quick_fix(html).expect("expected quick fix");
    assert_eq!(
        fixed,
        "<p>\r\n<div hx-ext=\"sse\" sse-connect=\"/e\" sse-swap=\"m\">x</div>"
    );
}

#[test]
fn quick_fix_empty_hx_ext_becomes_ws() {
    let html = r#"<div hx-ext="" hx-ws="connect:/c">x</div>"#;
    let fixed = run_quick_fix(html).expect("expected quick fix");
    assert_eq!(fixed, r#"<div hx-ext="ws" ws-connect="/c">x</div>"#);
}

#[test]
fn quick_fix_bare_attribute() {
    let html = r#"<div hx-ws>x</div>"#;
    let fixed = run_quick_fix(html).expect("expected quick fix");
    assert_eq!(fixed, r#"<div hx-ext="ws">x</div>"#);
}
