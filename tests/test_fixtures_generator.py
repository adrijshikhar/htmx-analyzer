import os
from pathlib import Path
import sys
import tempfile
import unittest

# Ensure repo root is on sys.path for direct invocation
repo_root = Path(__file__).resolve().parent.parent
if str(repo_root) not in sys.path:
    sys.path.insert(0, str(repo_root))

try:
    import pytest
except ImportError:
    pytest = None

from scripts.generate_fixtures import generate_single_file, generate_workspace


class TestFixtureGenerator(unittest.TestCase):
    def test_generate_single_file_line_count(self):
        content = generate_single_file(lines=1000, dialect="html", seed=42)
        lines = content.strip().split("\n")
        self.assertEqual(len(lines), 1000)
        self.assertIn("hx-get=", content)
        self.assertIn("hx-target=", content)

    def test_generate_multi_dialect_syntax(self):
        jinja = generate_single_file(lines=100, dialect="jinja", seed=42)
        self.assertTrue("{%" in jinja or "{{" in jinja)

        blade = generate_single_file(lines=100, dialect="blade", seed=42)
        self.assertTrue("@if" in blade or "@foreach" in blade or "{{" in blade)

        astro = generate_single_file(lines=100, dialect="astro", seed=42)
        self.assertIn("---", astro)

        svelte = generate_single_file(lines=100, dialect="svelte", seed=42)
        self.assertTrue("{#" in svelte or "{" in svelte)

        templ = generate_single_file(lines=100, dialect="templ", seed=42)
        self.assertIn("templ ", templ)

    def test_generate_workspace_structure(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            meta = generate_workspace(num_files=10, lines_per_file=50, base_dir=Path(tmpdir), seed=42)
            self.assertEqual(meta["total_files"], 10)
            self.assertEqual(meta["total_lines"], 500)
            self.assertEqual(len(list(Path(tmpdir).rglob("*.*"))), 10)


def test_generate_single_file_line_count():
    TestFixtureGenerator().test_generate_single_file_line_count()


def test_generate_multi_dialect_syntax():
    TestFixtureGenerator().test_generate_multi_dialect_syntax()


def test_generate_workspace_structure():
    TestFixtureGenerator().test_generate_workspace_structure()


if __name__ == "__main__":
    unittest.main()
