use htmx_parser::*;

#[test]
fn test_trigger_parser() {
    let triggers = HxTriggerParser::parse("keyup changed delay:500ms");
    assert_eq!(triggers.len(), 1);
    let t = &triggers[0];
    assert_eq!(t.event, "keyup");
    assert_eq!(t.filter, None);
    assert!(t.modifiers.contains(&TriggerModifier::Changed));
    assert!(t
        .modifiers
        .contains(&TriggerModifier::Delay("500ms".to_string())));

    // Multiple triggers
    let multi = HxTriggerParser::parse("click, keyup delay:200ms");
    assert_eq!(multi.len(), 2);
    assert_eq!(multi[0].event, "click");
    assert_eq!(multi[1].event, "keyup");

    // Filter syntax
    let with_filter = HxTriggerParser::parse("click[ctrlKey] once");
    assert_eq!(with_filter.len(), 1);
    assert_eq!(with_filter[0].event, "click");
    assert_eq!(with_filter[0].filter.as_deref(), Some("ctrlKey"));
    assert!(with_filter[0].modifiers.contains(&TriggerModifier::Once));

    // Duration validation
    assert!(HxTriggerParser::is_valid_duration("500ms"));
    assert!(HxTriggerParser::is_valid_duration("1s"));
    assert!(HxTriggerParser::is_valid_duration("0.5s"));
    assert!(HxTriggerParser::is_valid_duration("1m"));
    assert!(HxTriggerParser::is_valid_duration("500"));
    assert!(!HxTriggerParser::is_valid_duration("invalid"));
    assert!(!HxTriggerParser::is_valid_duration(""));
    assert!(!HxTriggerParser::is_valid_duration("infs"));
    assert!(!HxTriggerParser::is_valid_duration("nans"));
    assert!(!HxTriggerParser::is_valid_duration("-5s"));

    // Polling trigger
    let polling = HxTriggerParser::parse("every 2s");
    assert_eq!(polling.len(), 1);
    assert_eq!(polling[0].event, "every 2s");
    assert!(polling[0]
        .modifiers
        .contains(&TriggerModifier::Every("2s".to_string())));

    // Compound filter with spaces
    let compound_filter = HxTriggerParser::parse("click[ctrlKey && shiftKey] once");
    assert_eq!(compound_filter.len(), 1);
    assert_eq!(compound_filter[0].event, "click");
    assert_eq!(
        compound_filter[0].filter.as_deref(),
        Some("ctrlKey && shiftKey")
    );

    // Extended selector modifier
    let from_closest = HxTriggerParser::parse("submit from:closest form");
    assert_eq!(from_closest.len(), 1);
    assert!(from_closest[0]
        .modifiers
        .contains(&TriggerModifier::From("closest form".to_string())));
}

#[test]
fn test_swap_parser() {
    let swap = HxSwapParser::parse("innerHTML transition:true swap:500ms settle:100ms");
    assert_eq!(swap.style.as_deref(), Some("innerHTML"));
    assert!(swap.modifiers.contains(&SwapModifier::Transition(true)));
    assert!(swap
        .modifiers
        .contains(&SwapModifier::Swap("500ms".to_string())));
    assert!(swap
        .modifiers
        .contains(&SwapModifier::Settle("100ms".to_string())));

    // Scroll & show
    let scroll_swap = HxSwapParser::parse("outerHTML scroll:top show:#details:bottom");
    assert_eq!(scroll_swap.style.as_deref(), Some("outerHTML"));
    assert!(scroll_swap.modifiers.contains(&SwapModifier::Scroll {
        target: None,
        direction: ScrollDirection::Top,
    }));
    assert!(scroll_swap.modifiers.contains(&SwapModifier::Show {
        target: Some("#details".to_string()),
        direction: ScrollDirection::Bottom,
    }));
}

#[test]
fn test_target_parser() {
    assert_eq!(HxTargetParser::parse("this"), Some(HxTarget::This));
    assert_eq!(
        HxTargetParser::parse("closest tr"),
        Some(HxTarget::Closest("tr".to_string()))
    );
    assert_eq!(
        HxTargetParser::parse("find .card"),
        Some(HxTarget::Find(".card".to_string()))
    );
    assert_eq!(
        HxTargetParser::parse("#main-content"),
        Some(HxTarget::Id("main-content".to_string()))
    );
    assert_eq!(
        HxTargetParser::parse("#list > li"),
        Some(HxTarget::Selector("#list > li".to_string()))
    );
    assert_eq!(
        HxTargetParser::parse(".active-tab"),
        Some(HxTarget::Class("active-tab".to_string()))
    );
}

#[test]
fn test_cursor_after_closed_tag() {
    let html = "<div></div>";
    // Cursor right after </div> at offset 11
    let ctx = CursorScanner::scan(html, 11);
    assert_eq!(ctx.position_kind, CursorPositionKind::None);
}

#[test]
fn test_tag_scan_after_prose_apostrophe() {
    let html = "<p>Don't worry</p><button hx-";
    // Cursor at hx-
    let ctx = CursorScanner::scan(html, html.len());
    assert_eq!(ctx.tag_name.as_deref(), Some("button"));
    match ctx.position_kind {
        CursorPositionKind::AttributeName { prefix, .. } => {
            assert_eq!(prefix, "hx-");
        }
        _ => panic!("Expected AttributeName after prose with apostrophe"),
    }
}

#[test]
fn test_stray_less_than_in_template_text() {
    let html = "{% if a < b %}<div hx-target=";
    let offset = html.len();
    let ctx = CursorScanner::scan(html, offset);
    assert_eq!(ctx.tag_name.as_deref(), Some("div"));
    match ctx.position_kind {
        CursorPositionKind::AttributeValue { attribute_name, .. } => {
            assert_eq!(attribute_name, "hx-target");
        }
        _ => panic!(
            "Expected AttributeValue for div following '<' in template, got {:?}",
            ctx.position_kind
        ),
    }
}

#[test]
fn test_cursor_scanner_attribute_name() {
    let html = r#"<button hx-sw>"#;
    // Cursor at "hx-sw|" (offset 13)
    let ctx = CursorScanner::scan(html, 13);
    assert_eq!(ctx.tag_name.as_deref(), Some("button"));
    match ctx.position_kind {
        CursorPositionKind::AttributeName { prefix, range } => {
            assert_eq!(prefix, "hx-sw");
            assert_eq!(range, (8, 13));
        }
        _ => panic!(
            "Expected CursorPositionKind::AttributeName, got {:?}",
            ctx.position_kind
        ),
    }
}

#[test]
fn test_cursor_scanner_attribute_value() {
    let html = r#"<button hx-swap="innerHTML">"#;
    // Cursor at "inner|" (offset 22)
    let ctx = CursorScanner::scan(html, 22);
    assert_eq!(ctx.tag_name.as_deref(), Some("button"));
    match ctx.position_kind {
        CursorPositionKind::AttributeValue {
            attribute_name,
            value,
            cursor_offset_in_value,
            ..
        } => {
            assert_eq!(attribute_name, "hx-swap");
            assert_eq!(value, "innerHTML");
            assert_eq!(cursor_offset_in_value, 5);
        }
        _ => panic!(
            "Expected CursorPositionKind::AttributeValue, got {:?}",
            ctx.position_kind
        ),
    }
}

#[test]
fn test_cursor_scanner_unclosed_tag_typing() {
    // User typed "<button hx-" without closing '>'
    let html = "<button hx-";
    let ctx = CursorScanner::scan(html, 11);
    assert_eq!(ctx.tag_name.as_deref(), Some("button"));
    match ctx.position_kind {
        CursorPositionKind::AttributeName { prefix, .. } => {
            assert_eq!(prefix, "hx-");
        }
        _ => panic!("Expected AttributeName for unclosed tag typing"),
    }
}

#[test]
fn test_cursor_scanner_unquoted_attribute_value_end() {
    let html = "<button hx-swap=inner >";
    // Index of 'r' in 'inner' is 20. End of 'inner' is offset 21.
    let ctx = CursorScanner::scan(html, 21);
    match ctx.position_kind {
        CursorPositionKind::AttributeValue {
            attribute_name,
            value,
            cursor_offset_in_value,
            ..
        } => {
            assert_eq!(attribute_name, "hx-swap");
            assert_eq!(value, "inner");
            assert_eq!(cursor_offset_in_value, 5);
        }
        _ => panic!(
            "Expected AttributeValue at end of unquoted value, got {:?}",
            ctx.position_kind
        ),
    }
}

#[test]
fn test_html_document_id_extraction() {
    let html = r#"
        <!DOCTYPE html>
        <html>
            <body>
                <header id="main-header"></header>
                <main id="content">
                    <button id="submit-btn" hx-get="/clicked">Click</button>
                    <div id="result"></div>
                </main>
            </body>
        </html>
    "#;

    let tags = lex(html);
    let ids = document_ids(&tags);
    assert!(ids.contains("main-header"));
    assert!(ids.contains("content"));
    assert!(ids.contains("submit-btn"));
    assert!(ids.contains("result"));
    assert_eq!(ids.len(), 4);
}

#[test]
fn cursor_in_brace_value_is_none() {
    let html = "<div hx-swap={s}>";
    let offset = html.find("{s").unwrap() + 2;
    let ctx = CursorScanner::scan(html, offset);
    assert_eq!(ctx.position_kind, CursorPositionKind::None);
}

#[test]
fn cursor_in_templ_tag_finds_attribute() {
    let sample = "package v\ntempl Row(i Item) {\n\t<div id={ i.ID } hx-get={ \"/i/\" + i.ID } hx-swap=\"bogus\" hx-target=\"this\">{ i.Name }</div>\n}";
    let offset = sample.find("hx-target").unwrap() + 2;
    let ctx = CursorScanner::scan(sample, offset);
    match ctx.position_kind {
        CursorPositionKind::AttributeName { prefix, .. } => {
            assert_eq!(prefix, "hx-target");
        }
        _ => panic!(
            "Expected AttributeName for hx-target in templ tag, got {:?}",
            ctx.position_kind
        ),
    }
}
