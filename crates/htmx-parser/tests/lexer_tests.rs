use htmx_parser::{document_ids, lex, AttrValue, Quote};
use std::collections::HashSet;

#[test]
fn lex_basic_attributes() {
    let src = r#"<button hx-get="/a" hx-swap='outerHTML' disabled data-x=1>"#;
    let tags = lex(src);
    assert_eq!(tags.len(), 1);
    let tag = &tags[0];
    assert_eq!(tag.name, "button");
    assert!(tag.closed);
    assert_eq!(tag.end, src.len());

    let names: Vec<&str> = tag.attributes.iter().map(|a| a.name).collect();
    assert_eq!(names, vec!["hx-get", "hx-swap", "disabled", "data-x"]);

    for attr in &tag.attributes {
        assert_eq!(&src[attr.name_range.clone()], attr.name);
    }

    let quotes: Vec<Option<Quote>> = tag
        .attributes
        .iter()
        .map(|a| a.value.as_ref().map(|v| v.quote))
        .collect();
    assert_eq!(
        quotes,
        vec![
            Some(Quote::Double),
            Some(Quote::Single),
            None,
            Some(Quote::Unquoted)
        ]
    );

    let hx_get_val = tag.attributes[0].value.as_ref().unwrap();
    assert_eq!(hx_get_val.text, "/a");
    assert_eq!(&src[hx_get_val.range.clone()], "/a");
}

#[test]
fn lex_skips_comments_doctype_and_end_tags() {
    let src = r#"<!DOCTYPE html><!-- <div hx-get="x"> --></p><b id="k">"#;
    let tags = lex(src);
    let names: Vec<&str> = tags.iter().map(|t| t.name).collect();
    assert_eq!(names, vec!["b"]);
    assert_eq!(tags[0].attributes.len(), 1);
    assert_eq!(tags[0].attributes[0].name, "id");
    assert_eq!(&src[tags[0].attributes[0].name_range.clone()], "id");
    let val = tags[0].attributes[0].value.as_ref().unwrap();
    assert_eq!(val.text, "k");
    assert_eq!(&src[val.range.clone()], "k");
}

#[test]
fn lex_skips_raw_text_elements() {
    let src =
        r#"<script>s='<div hx-swap="bogus">'</script><SCRIPT></SCRIPT><style>a<b{}</style><i>"#;
    let tags = lex(src);
    let names: Vec<&str> = tags.iter().map(|t| t.name).collect();
    assert_eq!(names, vec!["script", "SCRIPT", "style", "i"]);
    for tag in &tags {
        for attr in &tag.attributes {
            assert_ne!(attr.name, "hx-swap");
        }
    }
}

#[test]
fn lex_brace_values_with_inner_quotes() {
    let src = r#"<div id={ i.ID } hx-get={ "/i/" + i.ID } hx-swap="bogus">"#;
    let tags = lex(src);
    assert_eq!(tags.len(), 1);
    let names: Vec<&str> = tags[0].attributes.iter().map(|a| a.name).collect();
    assert_eq!(names, vec!["id", "hx-get", "hx-swap"]);

    for attr in &tags[0].attributes {
        assert_eq!(&src[attr.name_range.clone()], attr.name);
    }

    let hx_get = &tags[0].attributes[1];
    let val = hx_get.value.as_ref().unwrap();
    assert_eq!(val.quote, Quote::Brace);
    assert_eq!(val.text, r#" "/i/" + i.ID "#);
    assert_eq!(&src[val.range.clone()], r#" "/i/" + i.ID "#);

    let hx_swap = &tags[0].attributes[2];
    let val_swap = hx_swap.value.as_ref().unwrap();
    assert_eq!(val_swap.text, "bogus");
    assert_eq!(&src[val_swap.range.clone()], "bogus");
}

#[test]
fn lex_template_blocks_inside_tag() {
    let src = r#"<div {% if a %}hx-get="/a"{% endif %} <?php if($x): ?>hx-post="/p"<?php endif ?> <% if y %>hx-put="/u"<% end %>>"#;
    let tags = lex(src);
    assert_eq!(tags.len(), 1);
    assert!(tags[0].closed);
    let names: Vec<&str> = tags[0].attributes.iter().map(|a| a.name).collect();
    assert_eq!(names, vec!["hx-get", "hx-post", "hx-put"]);
    for attr in &tags[0].attributes {
        assert_eq!(&src[attr.name_range.clone()], attr.name);
    }
}

#[test]
fn lex_template_spans_inside_quotes() {
    let src = r#"<a hx-get="{{ url_for("x") }}" hx-swap="bogus">"#;
    let tags = lex(src);
    assert_eq!(tags.len(), 1);
    let hx_get = &tags[0].attributes[0];
    let val_get = hx_get.value.as_ref().unwrap();
    assert_eq!(val_get.text, r#"{{ url_for("x") }}"#);
    assert_eq!(&src[val_get.range.clone()], r#"{{ url_for("x") }}"#);
    assert!(val_get.is_dynamic());

    let hx_swap = &tags[0].attributes[1];
    let val_swap = hx_swap.value.as_ref().unwrap();
    assert_eq!(val_swap.text, "bogus");
    assert_eq!(&src[val_swap.range.clone()], "bogus");
}

#[test]
fn lex_unclosed_tag_cut_at_next_tag() {
    let src = "<button hx-\n<div id=\"a\">";
    let tags = lex(src);
    assert_eq!(tags.len(), 2);
    assert!(!tags[0].closed);
    assert_eq!(tags[0].end, src.find("<div").unwrap());
    let names0: Vec<&str> = tags[0].attributes.iter().map(|a| a.name).collect();
    assert_eq!(names0, vec!["hx-"]);
    assert_eq!(&src[tags[0].attributes[0].name_range.clone()], "hx-");

    assert_eq!(tags[1].name, "div");
    assert!(tags[1].closed);
}

#[test]
fn lex_text_lt_is_not_a_tag() {
    let src = "a < b and 3<4 <b>";
    let tags = lex(src);
    let names: Vec<&str> = tags.iter().map(|t| t.name).collect();
    assert_eq!(names, vec!["b"]);
}

#[test]
fn lex_equals_at_eof_gives_empty_value() {
    let src = "{% if a < b %}<div hx-target=";
    let tags = lex(src);
    assert_eq!(tags.len(), 1);
    let tag = &tags[0];
    assert_eq!(tag.name, "div");
    assert_eq!(tag.attributes.len(), 1);
    let attr = &tag.attributes[0];
    assert_eq!(attr.name, "hx-target");
    let val = attr.value.as_ref().unwrap();
    assert_eq!(val.quote, Quote::Unquoted);
    assert_eq!(val.text, "");
    assert_eq!(val.range, src.len()..src.len());
    assert_eq!(&src[val.range.clone()], "");
}

#[test]
fn lex_progress_on_stray_quotes() {
    let src1 = r#"<div " ' = hx-swap="x">"#;
    let tags1 = lex(src1);
    assert_eq!(tags1.len(), 1);
    assert!(tags1[0].closed);
    let names1: Vec<&str> = tags1[0].attributes.iter().map(|a| a.name).collect();
    assert_eq!(names1, vec!["hx-swap"]);

    let src2 = r#"<div a <3 ( hx-swap="x">"#;
    let tags2 = lex(src2);
    assert_eq!(tags2.len(), 1);
    assert!(tags2[0].closed);
    let names2: Vec<&str> = tags2[0].attributes.iter().map(|a| a.name).collect();
    assert_eq!(names2, vec!["a", "3", "hx-swap"]);
    for attr in &tags2[0].attributes {
        assert_eq!(&src2[attr.name_range.clone()], attr.name);
    }
}

#[test]
fn lex_brace_stops_attribute_name() {
    let src = r#"<input checked{% if a > b %} hx-swap="x">"#;
    let tags = lex(src);
    assert_eq!(tags.len(), 1);
    assert!(tags[0].closed);
    let names: Vec<&str> = tags[0].attributes.iter().map(|a| a.name).collect();
    assert_eq!(names, vec!["checked", "hx-swap"]);
    assert_eq!(&src[tags[0].attributes[0].name_range.clone()], "checked");
    assert_eq!(&src[tags[0].attributes[1].name_range.clone()], "hx-swap");
}

#[test]
fn lex_self_closing_raw_text_tag() {
    let src = r#"<textarea bind:value={x} /><b hx-swap="bogus">"#;
    let tags = lex(src);
    let names: Vec<&str> = tags.iter().map(|t| t.name).collect();
    assert_eq!(names, vec!["textarea", "b"]);
    assert_eq!(tags[0].name, "textarea");
    assert_eq!(tags[1].name, "b");
}

#[test]
fn lex_equals_then_newline_tag() {
    let src = "<button hx-swap=\n<div id=\"a\">";
    let tags = lex(src);
    assert_eq!(tags.len(), 2);
    let t0 = &tags[0];
    assert_eq!(t0.name, "button");
    assert!(!t0.closed);
    assert_eq!(t0.end, src.find("<div").unwrap());
    assert_eq!(t0.attributes.len(), 1);
    let val0 = t0.attributes[0].value.as_ref().unwrap();
    assert_eq!(val0.quote, Quote::Unquoted);
    assert_eq!(val0.text, "");

    assert_eq!(tags[1].name, "div");
}

#[test]
fn lex_blade_directive_parens() {
    let src = r#"<div @class(['a' => $b]) hx-swap="bogus">"#;
    let tags = lex(src);
    assert_eq!(tags.len(), 1);
    assert!(tags[0].closed);
    let names: Vec<&str> = tags[0].attributes.iter().map(|a| a.name).collect();
    assert_eq!(names, vec!["@class", "hx-swap"]);
    assert_eq!(&src[tags[0].attributes[0].name_range.clone()], "@class");
    assert_eq!(&src[tags[0].attributes[1].name_range.clone()], "hx-swap");
}

#[test]
fn lex_is_dynamic_table() {
    let dynamic_cases = [
        "{{x}}",
        "a{%b%}",
        "<%= x %>",
        "<?= $x ?>",
        "${x}",
        "{style}",
        "{!! $x !!}",
    ];
    for text in dynamic_cases {
        let v = AttrValue {
            text,
            range: 0..text.len(),
            quote: Quote::Double,
        };
        assert!(v.is_dynamic(), "expected dynamic for {:?}", text);
    }
    let brace_v = AttrValue {
        text: "s",
        range: 0..1,
        quote: Quote::Brace,
    };
    assert!(brace_v.is_dynamic(), "expected dynamic for Brace");

    let static_cases = ["#id", "innerHTML", "click delay:1s", "keyup[key=='Enter']"];
    for text in static_cases {
        let v = AttrValue {
            text,
            range: 0..text.len(),
            quote: Quote::Double,
        };
        assert!(!v.is_dynamic(), "expected static for {:?}", text);
    }
}

#[test]
fn document_ids_static_only() {
    let src = r#"<a id="x"></a><b ID='y'></b><c id="r-{{ i }}"></c><d id={v}></d><e id="">"#;
    let tags = lex(src);
    let ids = document_ids(&tags);
    let mut expected = HashSet::new();
    expected.insert("x");
    expected.insert("y");
    assert_eq!(ids, expected);
}

#[test]
fn lex_multibyte_and_crlf() {
    let src = "<p title=\"😀é\">\r\n<b hx-swap=\"x\">";
    let tags = lex(src);
    assert_eq!(tags.len(), 2);
    let val = tags[0].attributes[0].value.as_ref().unwrap();
    assert_eq!(val.text, "😀é");
    assert_eq!(&src[val.range.clone()], "😀é");
}

#[test]
fn lex_never_panics_on_prefixes() {
    let corpus = [
        r#"<button hx-get="/a" hx-swap='outerHTML' disabled data-x=1>"#,
        r#"<!DOCTYPE html><!-- <div hx-get="x"> --></p><b id="k">"#,
        r#"<script>s='<div hx-swap="bogus">'</script><SCRIPT></SCRIPT><style>a<b{}</style><i>"#,
        r#"<div id={ i.ID } hx-get={ "/i/" + i.ID } hx-swap="bogus">"#,
        r#"<div {% if a %}hx-get="/a"{% endif %} <?php if($x): ?>hx-post="/p"<?php endif ?> <% if y %>hx-put="/u"<% end %>>"#,
        r#"<a hx-get="{{ url_for("x") }}" hx-swap="bogus">"#,
        "<button hx-\n<div id=\"a\">",
        "a < b and 3<4 <b>",
        "{% if a < b %}<div hx-target=",
        r#"<div " ' = hx-swap="x">"#,
        r#"<div a <3 ( hx-swap="x">"#,
        r#"<input checked{% if a > b %} hx-swap="x">"#,
        r#"<textarea bind:value={x} /><b hx-swap="bogus">"#,
        "<button hx-swap=\n<div id=\"a\">",
        r#"<div @class(['a' => $b]) hx-swap="bogus">"#,
        "<p title=\"😀é\">\r\n<b hx-swap=\"x\">",
    ]
    .join("\n");

    for (byte_idx, _) in corpus.char_indices() {
        let prefix = &corpus[..byte_idx];
        let tags = lex(prefix);
        for tag in tags {
            assert!(tag.start <= prefix.len());
            assert!(tag.end <= prefix.len());
            assert!(prefix.is_char_boundary(tag.start));
            assert!(prefix.is_char_boundary(tag.end));
            for attr in tag.attributes {
                assert!(attr.name_range.start <= prefix.len());
                assert!(attr.name_range.end <= prefix.len());
                assert!(prefix.is_char_boundary(attr.name_range.start));
                assert!(prefix.is_char_boundary(attr.name_range.end));
                assert_eq!(&prefix[attr.name_range.clone()], attr.name);
                assert!(attr.end <= prefix.len());
                assert!(prefix.is_char_boundary(attr.end));
                if let Some(val) = attr.value {
                    assert!(val.range.start <= prefix.len());
                    assert!(val.range.end <= prefix.len());
                    assert!(prefix.is_char_boundary(val.range.start));
                    assert!(prefix.is_char_boundary(val.range.end));
                    assert_eq!(&prefix[val.range.clone()], val.text);
                }
            }
        }
    }
}
