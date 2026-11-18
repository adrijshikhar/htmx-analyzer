use htmx_service::HtmxLanguageService;
use htmx_vfs::Document;

fn codes(text: &str) -> Vec<String> {
    let service = HtmxLanguageService::new();
    let doc = Document::new("inmemory://test.html".parse().unwrap(), 1, text);
    let mut c: Vec<String> = service
        .validate(&doc)
        .into_iter()
        .filter_map(|d| match d.code {
            Some(lsp_types::NumberOrString::String(s)) => Some(s),
            _ => None,
        })
        .collect();
    c.sort();
    c
}

#[test]
fn template_corpus_diagnostics() {
    let cases: [(&str, Vec<&str>); 11] = [
        // jinja
        (
            "<ul>{% for i in items %}<li id=\"row-{{ i.id }}\"><button hx-get=\"/i/{{ i.id }}\" hx-target=\"#row-{{ i.id }}\" hx-swap=\"outerHTML\">x</button></li>{% endfor %}</ul>\n<div {% if a %}hx-get=\"/a\"{% endif %} hx-swap=\"bogus\">y</div>",
            vec!["HTMX002"],
        ),
        // django
        (
            "{% extends \"base.html\" %}{% block c %}<form hx-post=\"{% url 'save' %}\" hx-target=\"#result\" hx-trigger=\"submit delay:abc\">{% csrf_token %}</form>{% endblock %}",
            vec!["HTMX003", "HTMX004"],
        ),
        // erb
        (
            "<% @items.each do |i| %><div id=\"r<%= i.id %>\" hx-get=\"<%= item_path(i) %>\" hx-swap=\"bogus\"></div><% end %>",
            vec!["HTMX002"],
        ),
        // php
        (
            "<?php foreach ($items as $i): ?><div hx-get=\"/i/<?= $i->id ?>\" hx-target=\"#x-<?= $i->id ?>\" hx-swap=\"bogus\"></div><?php endforeach; ?>",
            vec!["HTMX002"],
        ),
        // blade
        (
            "@foreach($items as $i)<div hx-get=\"{{ route('i', $i) }}\" hx-swap=\"bogus\">@csrf</div>@endforeach",
            vec!["HTMX002"],
        ),
        // templ
        (
            "package v\ntempl Row(i Item) {\n\t<div id={ i.ID } hx-get={ \"/i/\" + i.ID } hx-swap=\"bogus\" hx-target=\"this\">{ i.Name }</div>\n}",
            vec!["HTMX002"],
        ),
        // astro
        (
            "---\nconst s = \"outerHTML\";\n---\n<div hx-get={url} hx-swap={s} hx-target=\"#nope\"></div>",
            vec!["HTMX004"],
        ),
        // vue
        (
            "<template><div :hx-get=\"url\" hx-swap=\"bogus\" v-if=\"ok\">x</div></template>",
            vec!["HTMX002"],
        ),
        // svelte
        (
            "<script>let s = \"innerHTML\";</script>\n{#if ok}<div hx-get={url} hx-swap={s} hx-trigger=\"click\">x</div>{/if}",
            vec![],
        ),
        // svelte-quoted
        (
            "<div hx-swap=\"{style}\" hx-trigger=\"keyup delay:{d}ms\">x</div>",
            vec![],
        ),
        // blade-directive
        (
            "<div @class(['a' => $b]) hx-swap=\"bogus\">x</div>",
            vec!["HTMX002"],
        ),
    ];

    for (sample, expected) in cases {
        let actual = codes(sample);
        assert_eq!(
            actual, expected,
            "failed on sample:\n{}\ngot: {:?}, expected: {:?}",
            sample, actual, expected
        );
    }
}

#[test]
fn deep_nesting_still_reports() {
    let src = format!(
        "{}<b hx-swap=\"bogus\">x</b>{}",
        "<div>".repeat(300),
        "</div>".repeat(300)
    );
    assert_eq!(codes(&src), vec!["HTMX002"]);
}

#[test]
fn script_content_ignored() {
    let src = "<script>x='<b hx-swap=\"bogus\">'</script>";
    assert_eq!(codes(src), Vec::<String>::new());
}

#[test]
fn unterminated_quote_recovers_after_fix() {
    let unterminated = "<a hx-get=\"/x>t</a><b hx-swap=\"bogus\">";
    let _ = codes(unterminated); // must not panic

    let closed = "<a hx-get=\"/x\">t</a><b hx-swap=\"bogus\">";
    assert_eq!(codes(closed), vec!["HTMX002"]);
}
