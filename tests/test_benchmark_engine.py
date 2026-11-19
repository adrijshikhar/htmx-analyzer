import os
from pathlib import Path
import sys
import unittest

# Ensure repo root is on sys.path for direct invocation
repo_root = Path(__file__).resolve().parent.parent
if str(repo_root) not in sys.path:
    sys.path.insert(0, str(repo_root))

from scripts.run_benchmarks import LspProcess, measure_startup


class TestBenchmarkEngine(unittest.TestCase):
    def setUp(self):
        self.binary = (
            Path(__file__).resolve().parent.parent
            / "target"
            / "release"
            / "htmx-analyzer"
        )
        if not self.binary.exists():
            self.skipTest("Release binary not built at " + str(self.binary))

    def test_startup_measurement(self):
        startup_ms, idle_mb = measure_startup(str(self.binary))
        self.assertGreater(startup_ms, 0)
        self.assertLess(startup_ms, 200)  # Must boot under 200ms
        self.assertGreater(idle_mb, 0)
        self.assertLess(idle_mb, 50)  # Idle RSS must be under 50MB

    def test_lsp_handshake_and_completion(self):
        client = LspProcess(str(self.binary))
        client.start()
        init_res = client.initialize()
        self.assertIn("capabilities", init_res)
        uri = "file:///test.html"
        client.did_open(uri, '<button hx-target="#card" id="card">')
        items = client.completion(uri, line=0, character=23)
        self.assertTrue(any("card" in item.get("label", "") for item in items))
        client.shutdown()

    def test_lsp_hover(self):
        client = LspProcess(str(self.binary))
        client.start()
        client.initialize()
        uri = "file:///hover_test.html"
        client.did_open(uri, '<button hx-get="/api">')
        hover_res = client.hover(uri, line=0, character=10)
        self.assertIsNotNone(hover_res)
        client.shutdown()

    def test_lsp_diagnostics_and_did_change(self):
        client = LspProcess(str(self.binary))
        client.start()
        client.initialize()
        uri = "file:///diag_test.html"
        client.did_open(uri, '<button hx-target="#ghost">')
        diags = client.wait_for_diagnostics(uri, timeout=5.0)
        self.assertTrue(len(diags) > 0)

        # Fix ghost ID with did_change
        client.did_change(uri, '<button hx-target="#ghost" id="ghost">', version=2)
        diags2 = client.wait_for_diagnostics(uri, timeout=5.0)
        self.assertEqual(len(diags2), 0)
        client.shutdown()

    def test_get_rss_mb(self):
        client = LspProcess(str(self.binary))
        client.start()
        rss = client.get_rss_mb()
        self.assertGreater(rss, 0.0)
        client.shutdown()


if __name__ == "__main__":
    unittest.main()
