use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use htmx_service::{find_references, goto_definition, WorkspaceIdIndex};
use htmx_vfs::Document;
use lsp_types::{Position, Uri};
use std::hint::black_box;
use std::str::FromStr;

fn gen_template_corpus(lines: usize) -> (String, Position, Position) {
    let mut out = Vec::with_capacity(lines);
    out.push("<!doctype html>".to_string());
    out.push("<html><body>".to_string());

    let mut hit_pos = Position::new(0, 0);
    let mut miss_pos = Position::new(0, 0);

    let mut i = 0;
    while out.len() < lines - 2 {
        let current_line = out.len();
        out.push(format!("<div id=\"item-{}\" class=\"container\">", i));

        let target_line = format!(
            "  <button hx-get=\"/api/{}\" hx-target=\"#item-{}\">Load {}</button>",
            i, i, i
        );
        let miss_line = format!(
            "  <button hx-post=\"/action/{}\" hx-target=\"#nonexistent-{}\">Act</button>",
            i, i
        );

        if i == lines / 10 {
            // Save hit position on #item-N
            let col = target_line.find(&format!("#item-{}", i)).unwrap() + 2;
            hit_pos = Position::new((current_line + 1) as u32, col as u32);

            let miss_col = miss_line.find(&format!("#nonexistent-{}", i)).unwrap() + 2;
            miss_pos = Position::new((current_line + 2) as u32, miss_col as u32);
        }

        out.push(target_line);
        out.push(miss_line);
        out.push("</div>".to_string());
        i += 1;
    }

    out.push("</body></html>".to_string());
    out.truncate(lines);
    let doc_text = out.join("\n") + "\n";
    (doc_text, hit_pos, miss_pos)
}

fn bench_navigation(c: &mut Criterion) {
    let mut group = c.benchmark_group("navigation");

    // Table-driven sizes: Small (100), Medium (1,000), Large (10,000)
    for &size in &[100, 1_000, 10_000] {
        let (html, hit_pos, miss_pos) = gen_template_corpus(size);
        let uri = Uri::from_str(&format!("file:///doc_{}.html", size)).unwrap();
        let doc = Document::new(uri.clone(), 1, &html);

        let mut index = WorkspaceIdIndex::new();
        index.update_file(&uri, &html);

        group.throughput(Throughput::Elements(size as u64));

        // 1. Go to Definition (Hit)
        group.bench_with_input(
            BenchmarkId::new("goto_definition_hit", size),
            &size,
            |b, _| {
                b.iter(|| {
                    black_box(goto_definition(
                        black_box(&uri),
                        black_box(hit_pos),
                        black_box(&doc),
                        black_box(&index),
                        black_box(false),
                    ))
                })
            },
        );

        // 2. Go to Definition (Miss)
        group.bench_with_input(
            BenchmarkId::new("goto_definition_miss", size),
            &size,
            |b, _| {
                b.iter(|| {
                    black_box(goto_definition(
                        black_box(&uri),
                        black_box(miss_pos),
                        black_box(&doc),
                        black_box(&index),
                        black_box(false),
                    ))
                })
            },
        );

        // 3. Find References
        group.bench_with_input(
            BenchmarkId::new("find_references", size),
            &size,
            |b, _| {
                b.iter(|| {
                    black_box(find_references(
                        black_box(&uri),
                        black_box(hit_pos),
                        black_box(&doc),
                        black_box(&[&doc]),
                        black_box(&index),
                        black_box(true),
                    ))
                })
            },
        );
    }

    group.finish();

    // 4. Workspace Index Build (100 simulated template files)
    let mut index_group = c.benchmark_group("workspace_index");
    let mut files = Vec::with_capacity(100);
    let mut total_bytes = 0u64;
    for i in 0..100 {
        let uri = Uri::from_str(&format!("file:///workspace/partials/partial_{}.html", i)).unwrap();
        let html = format!(
            r##"<div id="partial-block-{}" class="card">
  <button hx-target="#partial-block-{}">Click</button>
  <span id="counter-{}">0</span>
</div>"##,
            i, i, i
        );
        total_bytes += html.len() as u64;
        files.push((uri, html));
    }

    index_group.throughput(Throughput::Bytes(total_bytes));
    index_group.bench_function("build_100_files", |b| {
        b.iter(|| {
            let mut idx = WorkspaceIdIndex::new();
            for (uri, text) in &files {
                idx.update_file(uri, text);
            }
            black_box(idx)
        })
    });

    index_group.finish();
}

criterion_group!(benches, bench_navigation);
criterion_main!(benches);
