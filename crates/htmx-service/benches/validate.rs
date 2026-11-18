use criterion::{criterion_group, criterion_main, Criterion};
use htmx_service::HtmxLanguageService;
use htmx_vfs::Document;
use lsp_types::{Position, Uri};
use std::str::FromStr;

fn gen_doc(lines: usize) -> String {
    let mut out = Vec::with_capacity(lines);
    out.push("<!doctype html>".to_string());
    out.push("<html><body>".to_string());
    let mut i = 0;
    while out.len() < lines - 2 {
        out.push(format!("<div id=\"item-{}\" class=\"row\">", i));
        out.push(format!(
            "  <button hx-get=\"/api/{}\" hx-target=\"#item-{}\" hx-swap=\"outerHTML swap:1s\" hx-trigger=\"click delay:500ms\">Load {}</button>",
            i, i, i
        ));
        out.push(format!(
            "  <input hx-post=\"/s/{}\" hx-trigger=\"keyup changed delay:300ms\" hx-target=\"#missing-{}\" name=\"q{}\">",
            i,
            i % 7,
            i
        ));
        out.push(format!(
            "  <span hx-sse=\"connect:/events/{}\" hx-swap=\"bogus\">x</span>",
            i
        ));
        out.push("</div>".to_string());
        i += 1;
    }
    out.push("</body></html>".to_string());
    out.truncate(lines);
    out.join("\n") + "\n"
}

fn bench_validate(c: &mut Criterion) {
    let service = HtmxLanguageService::new();
    let doc_10k_text = gen_doc(10000);
    let uri_10k = Uri::from_str("file:///test10k.html").unwrap();
    let doc_10k = Document::new(uri_10k, 1, &doc_10k_text);

    c.bench_function("validate_10k_lines", |b| {
        b.iter(|| service.validate(&doc_10k))
    });

    let line3 = doc_10k_text.lines().nth(3).unwrap();
    let col = line3.find("hx-target=\"#").unwrap() + "hx-target=\"#".len();
    let target_pos = Position {
        line: 3,
        character: col as u32,
    };

    c.bench_function("complete_target_10k_lines", |b| {
        b.iter(|| service.complete(&doc_10k, target_pos))
    });

    let unit =
        "<div id=\"a\"><button hx-get=\"/x\" hx-target=\"#a\" hx-swap=\"innerHTML\">b</button></div>";
    let reps = (4 * 1024 * 1024) / unit.len();
    let doc_4mib_text = unit.repeat(reps);
    let uri_4mib = Uri::from_str("file:///test4mib.html").unwrap();
    let doc_4mib = Document::new(uri_4mib, 1, &doc_4mib_text);

    c.bench_function("validate_4mib_single_line", |b| {
        b.iter(|| service.validate(&doc_4mib))
    });
}

criterion_group!(benches, bench_validate);
criterion_main!(benches);
