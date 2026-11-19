#!/usr/bin/env python3
"""
Deterministic Multi-Dialect & Workspace Fixture Generator.

Generates single-file stress fixtures (e.g. 1k, 10k, 50k, 100k lines) and
multi-file workspaces across 6 template dialects: html, jinja, blade, astro, svelte, templ.

Zero external python dependencies: uses Python standard library only.
100% deterministic generation via PRNG seed.
"""

from __future__ import annotations

import argparse
from pathlib import Path
import random
import sys
from typing import Any, Dict, List, Union

SUPPORTED_DIALECTS = ["html", "jinja", "blade", "astro", "svelte", "templ"]

DIALECT_EXTENSIONS: Dict[str, str] = {
    "html": ".html",
    "jinja": ".jinja",
    "blade": ".blade.php",
    "astro": ".astro",
    "svelte": ".svelte",
    "templ": ".templ",
}


def _build_lines(
    lines: int,
    header: List[str],
    footer: List[str],
    block_fn: Any,
    single_fn: Any,
    rng: random.Random,
) -> str:
    """Helper to assemble an exact line count fixture."""
    if lines <= 0:
        return ""

    min_frame = len(header) + len(footer)
    if lines <= min_frame:
        items = [single_fn(i, rng) for i in range(lines)]
        return "\n".join(items) + "\n"

    needed = lines - min_frame
    out = list(header)
    item_idx = 0

    while needed > 0:
        block = block_fn(item_idx, rng)
        if len(block) <= needed:
            out.extend(block)
            needed -= len(block)
        else:
            out.append(single_fn(item_idx, rng))
            needed -= 1
        item_idx += 1

    out.extend(footer)
    assert len(out) == lines, f"Expected {lines} lines, generated {len(out)}"
    return "\n".join(out) + "\n"


def _generate_html(lines: int, rng: random.Random) -> str:
    header = [
        "<!DOCTYPE html>",
        '<html lang="en">',
        "<head>",
        '    <meta charset="UTF-8">',
        "    <title>HTMX Stress Benchmark</title>",
        "</head>",
        "<body>",
    ]
    footer = [
        "</body>",
        "</html>",
    ]

    def block_fn(i: int, r: random.Random) -> List[str]:
        method = "hx-get" if i % 2 == 0 else r.choice(["hx-post", "hx-put", "hx-delete"])
        swap = r.choice(["outerHTML", "innerHTML", "beforebegin", "afterend"])
        path = r.choice(["api", "items", "data", "users"])
        return [
            f'    <div id="item-{i}" class="benchmark-card">',
            f'        <button {method}="/{path}/{i}" hx-target="#item-{i}" hx-swap="{swap}">Item {i}</button>',
            "    </div>",
        ]

    def single_fn(i: int, r: random.Random) -> str:
        swap = r.choice(["outerHTML", "innerHTML"])
        return f'<div id="item-{i}"> <button hx-get="/api/{i}" hx-target="#item-{i}" hx-swap="{swap}">Item {i}</button> </div>'

    return _build_lines(lines, header, footer, block_fn, single_fn, rng)


def _generate_jinja(lines: int, rng: random.Random) -> str:
    header = [
        '{% extends "base.html" %}',
        "{% block content %}",
        '<div class="jinja-benchmark">',
    ]
    footer = [
        "</div>",
        "{% endblock %}",
    ]

    def block_fn(i: int, r: random.Random) -> List[str]:
        action = r.choice(["post", "get", "patch", "delete"])
        attr = f"hx-{action}"
        var_name = r.choice(["url", "action_url", "endpoint", "target_url"])
        swap = r.choice(["outerHTML", "innerHTML", "beforeend"])
        return [
            f"    {{% if item_{i} %}}",
            f'        <button {attr}="{{{{ {var_name}_{i} }}}}" hx-target="#target-{{{{ loop.index }}}}" hx-swap="{swap}">{{{{ item.name }}}}</button>',
            "    {% endif %}",
        ]

    def single_fn(i: int, r: random.Random) -> str:
        var_name = r.choice(["url", "action_url"])
        return f'{{% if item_{i} %}} <button hx-post="{{{{ {var_name}_{i} }}}}" hx-target="#target-{{{{ loop.index }}}}">{{{{ item.name }}}}</button> {{% endif %}}'

    return _build_lines(lines, header, footer, block_fn, single_fn, rng)


def _generate_blade(lines: int, rng: random.Random) -> str:
    header = [
        "@extends('layouts.app')",
        "@section('content')",
        '<div class="blade-benchmark">',
    ]
    footer = [
        "</div>",
        "@endsection",
    ]

    def block_fn(i: int, r: random.Random) -> List[str]:
        action = r.choice(["delete", "post", "get", "put"])
        attr = f"hx-{action}"
        resource = r.choice(["items", "cards", "rows", "elements"])
        swap = r.choice(["outerHTML", "innerHTML", "beforebegin"])
        return [
            f"    @if($item_{i})",
            f'        <button {attr}="/{resource}/{{{{ ${i} }}}}" hx-target="#card-{{{{ ${i} }}}}" hx-swap="{swap}">Delete {i}</button>',
            "    @endif",
        ]

    def single_fn(i: int, r: random.Random) -> str:
        resource = r.choice(["items", "cards"])
        return f'@if($item_{i}) <button hx-delete="/{resource}/{{{{ ${i} }}}}" hx-target="#card-{{{{ ${i} }}}}">Delete</button> @endif'

    return _build_lines(lines, header, footer, block_fn, single_fn, rng)


def _generate_astro(lines: int, rng: random.Random) -> str:
    if lines == 1:
        return '--- const id = 0; --- <button hx-get={`/api/${id}`} hx-target={`#box-${id}`}>Click</button>\n'
    if lines == 2:
        return '---\n<button hx-get={`/api/0`} hx-target={`#box-0`}>Click</button>\n'
    if lines == 3:
        return "---\nconst id = 0;\n---\n"

    header = [
        "---",
        "const id = 0;",
        "---",
        '<div class="astro-benchmark">',
    ]
    footer = [
        "</div>",
    ]

    def block_fn(i: int, r: random.Random) -> List[str]:
        action = r.choice(["get", "post", "put"])
        attr = f"hx-{action}"
        resource = r.choice(["api", "items", "views"])
        return [
            f'    <div id={{`box-${{id}}_{i}`}}>',
            f'        <button {attr}={{`/{resource}/${{id}}`}} hx-target={{`#box-${{id}}`}}>Item {i}</button>',
            "    </div>",
        ]

    def single_fn(i: int, r: random.Random) -> str:
        resource = r.choice(["api", "items"])
        return f'    <button hx-get={{`/{resource}/${{id}}`}} hx-target={{`#box-${{id}}`}}>Item {i}</button>'

    return _build_lines(lines, header, footer, block_fn, single_fn, rng)


def _generate_svelte(lines: int, rng: random.Random) -> str:
    header = [
        "<script>",
        "    export let items = [];",
        "</script>",
        '<main class="svelte-benchmark">',
    ]
    footer = [
        "</main>",
    ]

    def block_fn(i: int, r: random.Random) -> List[str]:
        action = r.choice(["get", "post", "delete"])
        attr = f"hx-{action}"
        path = r.choice(["item", "user", "row"])
        return [
            "    {#each items as item}",
            f'        <button {attr}="/{path}/{{item.id}}" hx-target="#item-{{item.id}}">Item {i}</button>',
            "    {/each}",
        ]

    def single_fn(i: int, r: random.Random) -> str:
        path = r.choice(["item", "row"])
        return f'{{#each items as item}} <button hx-get="/{path}/{{item.id}}" hx-target="#item-{{item.id}}"> {{/each}}'

    return _build_lines(lines, header, footer, block_fn, single_fn, rng)


def _generate_templ(lines: int, rng: random.Random) -> str:
    if lines == 1:
        return 'templ Component0(i int) { <div id={ fmt.Sprintf("row-%d", 0) }><button hx-get="/api"></div> }\n'
    if lines == 2:
        return 'package main\ntempl Component(i int) { <div id={ fmt.Sprintf("row-%d", 0) }><button hx-get="/api"></div> }\n'
    if lines == 3:
        return 'package main\nimport "fmt"\ntempl Component(i int) { <div id={ fmt.Sprintf("row-%d", 0) }><button hx-get="/api"></div> }\n'

    header = [
        "package main",
        "",
        'import "fmt"',
        "",
        "templ BenchmarkComponent(items []int) {",
    ]
    footer = [
        "}",
    ]

    def block_fn(i: int, r: random.Random) -> List[str]:
        action = r.choice(["get", "post", "delete"])
        attr = f"hx-{action}"
        resource = r.choice(["api/item", "api/row", "api/data"])
        return [
            f'    <div id={{ fmt.Sprintf("row-%d", {i}) }}>',
            f'        <button {attr}={{ fmt.Sprintf("/{resource}/%d", {i}) }} hx-target={{ fmt.Sprintf("#row-%d", {i}) }}>Click</button>',
            "    </div>",
        ]

    def single_fn(i: int, r: random.Random) -> str:
        endpoint = r.choice(["api", "items", "rows"])
        return f'    templ Component{i}(i int) {{ <div id={{ fmt.Sprintf("row-%d", {i}) }}><button hx-get="/{endpoint}"></div> }}'

    return _build_lines(lines, header, footer, block_fn, single_fn, rng)


def generate_single_file(lines: int, dialect: str, seed: int = 42) -> str:
    """
    Generate a single template file with exact line count for a given dialect.

    Args:
        lines: Exact number of lines to produce.
        dialect: One of 'html', 'jinja', 'blade', 'astro', 'svelte', 'templ'.
        seed: PRNG seed for deterministic output (default: 42).

    Returns:
        The generated file content as a string.
    """
    if lines <= 0:
        raise ValueError(f"lines must be positive, got {lines}")

    dialect_lower = dialect.strip().lower()
    if dialect_lower not in SUPPORTED_DIALECTS:
        raise ValueError(
            f"Unsupported dialect '{dialect}'. Supported dialects: {SUPPORTED_DIALECTS}"
        )

    rng = random.Random(seed)

    if dialect_lower == "html":
        return _generate_html(lines, rng)
    elif dialect_lower == "jinja":
        return _generate_jinja(lines, rng)
    elif dialect_lower == "blade":
        return _generate_blade(lines, rng)
    elif dialect_lower == "astro":
        return _generate_astro(lines, rng)
    elif dialect_lower == "svelte":
        return _generate_svelte(lines, rng)
    elif dialect_lower == "templ":
        return _generate_templ(lines, rng)
    else:
        raise ValueError(f"Unsupported dialect '{dialect}'")


def generate_workspace(
    num_files: int, lines_per_file: int, base_dir: Union[Path, str], seed: int = 42
) -> Dict[str, Any]:
    """
    Generate a multi-file workspace across supported dialects.

    Args:
        num_files: Total number of files to generate.
        lines_per_file: Line count for each generated file.
        base_dir: Directory where workspace files will be written.
        seed: PRNG seed for deterministic workspace generation.

    Returns:
        Dict with metadata: total_files, total_lines, dialects, files.
    """
    if num_files < 0:
        raise ValueError(f"num_files must be non-negative, got {num_files}")
    if lines_per_file <= 0 and num_files > 0:
        raise ValueError(f"lines_per_file must be positive, got {lines_per_file}")

    base_path = Path(base_dir)
    base_path.mkdir(parents=True, exist_ok=True)

    rng = random.Random(seed)
    subdirs = ["", "components", "pages", "views", "partials"]

    file_records: List[str] = []
    total_lines = 0
    dialect_counts: Dict[str, int] = {d: 0 for d in SUPPORTED_DIALECTS}

    for i in range(num_files):
        dialect = SUPPORTED_DIALECTS[i % len(SUPPORTED_DIALECTS)]
        dialect_counts[dialect] += 1
        ext = DIALECT_EXTENSIONS[dialect]
        subdir = subdirs[i % len(subdirs)]

        target_dir = base_path / subdir if subdir else base_path
        target_dir.mkdir(parents=True, exist_ok=True)

        file_path = target_dir / f"item_{i:04d}{ext}"
        file_seed = rng.randint(0, 1_000_000_000)

        content = generate_single_file(lines_per_file, dialect=dialect, seed=file_seed)
        file_path.write_text(content, encoding="utf-8")

        actual_lines = len(content.strip().split("\n"))
        total_lines += actual_lines
        file_records.append(str(file_path.relative_to(base_path)))

    return {
        "total_files": num_files,
        "total_lines": total_lines,
        "lines_per_file": lines_per_file,
        "dialects": dialect_counts,
        "files": file_records,
    }


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Deterministic Multi-Dialect & Workspace Fixture Generator"
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    # Subcommand: file
    file_parser = subparsers.add_parser("file", help="Generate a single template file")
    file_parser.add_argument(
        "--lines", type=int, required=True, help="Number of lines to generate"
    )
    file_parser.add_argument(
        "--dialect",
        type=str,
        required=True,
        choices=SUPPORTED_DIALECTS,
        help="Template dialect",
    )
    file_parser.add_argument(
        "--output", type=Path, required=True, help="Output file path"
    )
    file_parser.add_argument(
        "--seed", type=int, default=42, help="PRNG seed for deterministic generation"
    )

    # Subcommand: workspace
    ws_parser = subparsers.add_parser(
        "workspace", help="Generate a multi-file workspace"
    )
    ws_parser.add_argument(
        "--files", type=int, required=True, help="Total number of files to generate"
    )
    ws_parser.add_argument(
        "--lines-per-file",
        type=int,
        required=True,
        help="Number of lines per file",
    )
    ws_parser.add_argument(
        "--output-dir", type=Path, required=True, help="Output directory path"
    )
    ws_parser.add_argument(
        "--seed", type=int, default=42, help="PRNG seed for deterministic generation"
    )

    args = parser.parse_args()

    if args.command == "file":
        content = generate_single_file(
            lines=args.lines, dialect=args.dialect, seed=args.seed
        )
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(content, encoding="utf-8")
        print(
            f"Generated {args.lines} lines of {args.dialect} fixture at {args.output}"
        )
    elif args.command == "workspace":
        meta = generate_workspace(
            num_files=args.files,
            lines_per_file=args.lines_per_file,
            base_dir=args.output_dir,
            seed=args.seed,
        )
        print(
            f"Generated workspace with {meta['total_files']} files ({meta['total_lines']} total lines) in {args.output_dir}"
        )


if __name__ == "__main__":
    main()
