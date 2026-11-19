use htmx_parser::{find_id_definitions, find_id_reference_at, find_id_references, lex};

#[test]
fn test_find_id_definitions() {
    let html = r##"<div id="first-item" class="p-4"><span id="second-item"></span></div>"##;
    let tags = lex(html);
    let defs = find_id_definitions(&tags);
    assert_eq!(defs.len(), 2);
    assert_eq!(defs[0].id, "first-item");
    assert_eq!(&html[defs[0].value_range.clone()], "first-item");
    assert_eq!(
        &html[defs[0].tag_range.clone()],
        r##"<div id="first-item" class="p-4">"##
    );
    assert_eq!(defs[1].id, "second-item");
    assert_eq!(&html[defs[1].value_range.clone()], "second-item");
    assert_eq!(
        &html[defs[1].tag_range.clone()],
        r##"<span id="second-item">"##
    );
}

#[test]
fn test_find_id_definitions_skips_dynamic() {
    let html = r##"<div id="item-{{ .ID }}"></div><div id="static-item"></div>"##;
    let tags = lex(html);
    let defs = find_id_definitions(&tags);
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].id, "static-item");
}

#[test]
fn test_find_id_reference_at_hx_target() {
    let html = r##"<button hx-target="#results" hx-indicator="#spinner">Click</button>"##;
    let tags = lex(html);

    // Offset inside "results"
    let offset_target = html.find("results").unwrap() + 2;
    let ref_item = find_id_reference_at(&tags, offset_target).expect("reference found");
    assert_eq!(ref_item.id, "results");
    assert_eq!(&html[ref_item.span.clone()], "results");
    assert_eq!(&html[ref_item.origin_span.clone()], "#results");

    // Offset inside "spinner"
    let offset_ind = html.find("spinner").unwrap() + 1;
    let ref_ind = find_id_reference_at(&tags, offset_ind).expect("reference found");
    assert_eq!(ref_ind.id, "spinner");
    assert_eq!(&html[ref_ind.span.clone()], "spinner");
    assert_eq!(&html[ref_ind.origin_span.clone()], "#spinner");
}

#[test]
fn test_find_id_reference_at_element_id() {
    let html = r##"<div id="target-div">Content</div>"##;
    let tags = lex(html);
    let offset = html.find("target-div").unwrap() + 3;
    let ref_item = find_id_reference_at(&tags, offset).expect("element id found as reference");
    assert_eq!(ref_item.id, "target-div");
    assert_eq!(&html[ref_item.span.clone()], "target-div");
    assert_eq!(&html[ref_item.origin_span.clone()], "target-div");
}

#[test]
fn test_find_id_reference_at_hx_swap_oob() {
    let html = r##"<div hx-swap-oob="true:#alerts">Alert</div>"##;
    let tags = lex(html);
    let offset = html.find("alerts").unwrap() + 2;
    let ref_item = find_id_reference_at(&tags, offset).expect("reference found in hx-swap-oob");
    assert_eq!(ref_item.id, "alerts");
    assert_eq!(&html[ref_item.span.clone()], "alerts");
    assert_eq!(&html[ref_item.origin_span.clone()], "#alerts");
}

#[test]
fn test_find_id_reference_at_hx_sync() {
    let html = r##"<form hx-sync="#search-form:abort">Search</form>"##;
    let tags = lex(html);
    let offset = html.find("search-form").unwrap() + 2;
    let ref_item = find_id_reference_at(&tags, offset).expect("reference found in hx-sync");
    assert_eq!(ref_item.id, "search-form");
    assert_eq!(&html[ref_item.span.clone()], "search-form");
    assert_eq!(&html[ref_item.origin_span.clone()], "#search-form");
}

#[test]
fn test_find_id_references_collection() {
    let html = r##"
<div id="main-content">
  <button hx-target="#main-content" hx-indicator="#loading">Load</button>
</div>
"##;
    let tags = lex(html);
    let refs = find_id_references(&tags);
    let ids: Vec<&str> = refs.iter().map(|r| r.id).collect();
    assert!(ids.contains(&"main-content"));
    assert!(ids.contains(&"loading"));
}
