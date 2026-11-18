use htmx_schema::*;

#[test]
fn test_attribute_lookup_normalized() {
    assert_eq!(get_attribute("hx-get").map(|a| a.name), Some("hx-get"));
    // data- prefix normalization
    assert_eq!(get_attribute("data-hx-get").map(|a| a.name), Some("hx-get"));
    assert_eq!(
        get_attribute("data-hx-post").map(|a| a.name),
        Some("hx-post")
    );
    assert_eq!(
        get_attribute("data-hx-swap").map(|a| a.name),
        Some("hx-swap")
    );
    assert_eq!(
        get_attribute("data-hx-target").map(|a| a.name),
        Some("hx-target")
    );
}

#[test]
fn test_hx_on_prefix_matching() {
    assert_eq!(get_attribute("hx-on:click").map(|a| a.name), Some("hx-on"));
    assert_eq!(
        get_attribute("data-hx-on:submit").map(|a| a.name),
        Some("hx-on")
    );
}

#[test]
fn test_deprecated_attributes() {
    let ws = get_attribute("hx-ws").expect("hx-ws should exist");
    assert!(ws.deprecated.is_some());

    let sse = get_attribute("hx-sse").expect("hx-sse should exist");
    assert!(sse.deprecated.is_some());

    let get = get_attribute("hx-get").expect("hx-get should exist");
    assert!(get.deprecated.is_none());
}

#[test]
fn test_all_core_attributes_present() {
    let required = [
        "hx-get",
        "hx-post",
        "hx-put",
        "hx-delete",
        "hx-patch",
        "hx-target",
        "hx-swap",
        "hx-trigger",
        "hx-indicator",
        "hx-boost",
        "hx-push-url",
        "hx-select",
        "hx-select-oob",
        "hx-swap-oob",
        "hx-vals",
        "hx-headers",
        "hx-params",
        "hx-include",
        "hx-sync",
        "hx-ext",
        "hx-confirm",
        "hx-prompt",
        "hx-validate",
        "hx-disinherit",
        "hx-disabled-elt",
        "hx-on",
        "hx-replace-url",
        "hx-encoding",
        "hx-preserve",
        "hx-history",
        "hx-history-elt",
    ];

    for attr in required {
        assert!(
            get_attribute(attr).is_some(),
            "Expected attribute {} to be present in schema",
            attr
        );
    }
}

#[test]
fn test_swap_styles_and_modifiers() {
    let swap = get_attribute("hx-swap").expect("hx-swap defined");
    assert!(swap.allowed_values.contains(&"innerHTML"));
    assert!(swap.allowed_values.contains(&"outerHTML"));
    assert!(swap.allowed_values.contains(&"beforeend"));
    assert!(swap.allowed_values.contains(&"afterend"));

    let mod_names: Vec<&str> = swap.modifiers.iter().map(|m| m.name).collect();
    assert!(mod_names.contains(&"transition"));
    assert!(mod_names.contains(&"swap"));
    assert!(mod_names.contains(&"settle"));
    assert!(mod_names.contains(&"scroll"));
    assert!(mod_names.contains(&"show"));
}

#[test]
fn test_extensions_registry() {
    assert!(get_extension("ws").is_some());
    assert!(get_extension("sse").is_some());
    assert!(get_extension("preload").is_some());
    assert!(get_extension("response-targets").is_some());
    assert!(get_extension("idiomorph").is_some());
    assert_eq!(get_extension("nonexistent"), None);
}

#[test]
fn canonical_name_lowercases_and_strips_data() {
    assert_eq!(canonical_name("HX-Get"), "hx-get");
    assert_eq!(canonical_name("DATA-HX-SWAP"), "hx-swap");
    assert_eq!(canonical_name("data-data-x"), "data-x");
}

#[test]
fn attribute_lookup_case_insensitive() {
    assert_eq!(attribute("HX-SWAP").unwrap().name, "hx-swap");
}
