#!/usr/bin/env python3
"""Stress + robustness harness for htmx-analyzer (stdio LSP)."""
import argparse
import json
import os
import random
import subprocess
import sys
import threading
import time
import queue

random.seed(7)


class Lsp:
    def __init__(self, binary):
        self.p = subprocess.Popen(
            [binary, "--stdio"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=open(os.devnull, "w"),
        )
        self.q = queue.Queue()
        self.next_id = 100
        self.publishes = []
        threading.Thread(target=self._read, daemon=True).start()

    def _read(self):
        f = self.p.stdout
        while True:
            hdr = b""
            while not hdr.endswith(b"\r\n\r\n"):
                c = f.read(1)
                if not c:
                    self.q.put(None)
                    return
                hdr += c
            lines = hdr.decode("latin1").split("\r\n")
            cl_lines = [l for l in lines if l.lower().startswith("content-length")]
            if not cl_lines:
                continue
            n = int(cl_lines[0].split(":")[1])
            msg = json.loads(f.read(n).decode("utf-8"))
            if msg.get("method") == "textDocument/publishDiagnostics":
                self.publishes.append(msg)
            self.q.put(msg)

    def send(self, msg):
        b = json.dumps(msg).encode("utf-8")
        self.p.stdin.write(b"Content-Length: %d\r\n\r\n" % len(b) + b)
        self.p.stdin.flush()

    def raw(self, data):
        self.p.stdin.write(data)
        self.p.stdin.flush()

    def notify(self, method, params):
        self.send({"jsonrpc": "2.0", "method": method, "params": params})

    def request(self, method, params):
        self.next_id += 1
        self.send({"jsonrpc": "2.0", "id": self.next_id, "method": method, "params": params})
        return self.next_id

    def wait(self, pred, timeout=60):
        end = time.time() + timeout
        while True:
            m = self.q.get(timeout=max(0.01, end - time.time()))
            if m is None:
                raise RuntimeError("server died")
            if pred(m):
                return m

    def wait_id(self, i, timeout=60):
        return self.wait(lambda m: m.get("id") == i, timeout)

    def wait_diag(self, uri, version=None, timeout=60):
        return self.wait(
            lambda m: m.get("method") == "textDocument/publishDiagnostics"
            and m.get("params", {}).get("uri") == uri
            and (version is None or m.get("params", {}).get("version") == version),
            timeout,
        )

    def init(self):
        i = self.request("initialize", {"processId": None, "rootUri": None, "capabilities": {}})
        self.wait_id(i)
        self.notify("initialized", {})

    def rss_mb(self):
        out = subprocess.run(
            ["ps", "-o", "rss=", "-p", str(self.p.pid)],
            capture_output=True,
            text=True,
        ).stdout.strip()
        return int(out) / 1024 if out else -1

    def alive(self):
        return self.p.poll() is None


def gen_doc(lines):
    out = ["<!doctype html>", "<html><body>"]
    i = 0
    while len(out) < lines - 2:
        out.append(f'<div id="item-{i}" class="row">')
        out.append(
            f'  <button hx-get="/api/{i}" hx-target="#item-{i}" hx-swap="outerHTML swap:1s" hx-trigger="click delay:500ms">Load {i}</button>'
        )
        out.append(
            f'  <input hx-post="/s/{i}" hx-trigger="keyup changed delay:300ms" hx-target="#missing-{i % 7}" name="q{i}">'
        )
        out.append(f'  <span hx-sse="connect:/events/{i}" hx-swap="bogus">x</span>')
        out.append("</div>")
        i += 1
    out.append("</body></html>")
    return "\n".join(out[:lines]) + "\n"


def utf16_len(s):
    return len(s.encode("utf-16-le")) // 2


def ms(t):
    return f"{t * 1000:.1f}ms"


def pct(xs, p):
    xs = sorted(xs)
    return xs[min(len(xs) - 1, int(len(xs) * p))]


def bench(binary, lines):
    s = Lsp(binary)
    s.init()
    text = gen_doc(lines)
    uri = f"file:///stress{lines}.html"
    t0 = time.time()
    s.notify(
        "textDocument/didOpen",
        {"textDocument": {"uri": uri, "languageId": "html", "version": 1, "text": text}},
    )
    d = s.wait_diag(uri)
    open_t = time.time() - t0
    print(
        f"\n## {lines} lines ({len(text) / 1024:.0f} KB): didOpen->diagnostics {ms(open_t)}, {len(d['params']['diagnostics'])} diags, RSS {s.rss_mb():.1f}MB"
    )

    # sequential keystrokes: each waits for its diagnostics
    lat, v = [], 1
    for k in range(100):
        v += 1
        ln = random.randrange(2, lines - 2)
        t0 = time.time()
        s.notify(
            "textDocument/didChange",
            {
                "textDocument": {"uri": uri, "version": v},
                "contentChanges": [
                    {
                        "range": {
                            "start": {"line": ln, "character": 0},
                            "end": {"line": ln, "character": 0},
                        },
                        "text": " ",
                    }
                ],
            },
        )
        s.wait_diag(uri, v)
        lat.append(time.time() - t0)
    print(
        f"  keystroke->diagnostics (sequential, n=100): p50 {ms(pct(lat, .5))} p95 {ms(pct(lat, .95))} max {ms(max(lat))}"
    )

    # burst typing: 200 changes fired back-to-back, then completion; measures queue backlog
    t0 = time.time()
    pub_before = len(s.publishes)
    for k in range(200):
        v += 1
        s.notify(
            "textDocument/didChange",
            {
                "textDocument": {"uri": uri, "version": v},
                "contentChanges": [
                    {
                        "range": {
                            "start": {"line": 5, "character": 0},
                            "end": {"line": 5, "character": 0},
                        },
                        "text": "x",
                    }
                ],
            },
        )
    cid = s.request(
        "textDocument/completion",
        {"textDocument": {"uri": uri}, "position": {"line": 3, "character": 60}},
    )
    s.wait_id(cid, timeout=300)
    burst_t = time.time() - t0
    time.sleep(0.5)
    pub_count = len(s.publishes) - pub_before
    print(
        f"  burst 200 keystrokes then completion: answered in {ms(burst_t)} (publishes: {pub_count})"
    )

    # completion inside hx-target value
    line3 = text.split("\n")[3]
    col = line3.index('hx-target="#') + len('hx-target="#')
    lat = []
    r = None
    for _ in range(30):
        t0 = time.time()
        cid = s.request(
            "textDocument/completion",
            {"textDocument": {"uri": uri}, "position": {"line": 3, "character": col}},
        )
        r = s.wait_id(cid)
        lat.append(time.time() - t0)
    res = r.get("result", {})
    items = res.get("items", []) if isinstance(res, dict) else (res or [])
    print(
        f"  completion in hx-target (n=30): p50 {ms(pct(lat, .5))} max {ms(max(lat))}, items={len(items)}"
    )

    lat = []
    for _ in range(30):
        t0 = time.time()
        hid = s.request(
            "textDocument/hover",
            {"textDocument": {"uri": uri}, "position": {"line": 3, "character": 12}},
        )
        s.wait_id(hid)
        lat.append(time.time() - t0)
    print(f"  hover (n=30): p50 {ms(pct(lat, .5))} p95 {ms(pct(lat, .95))} max {ms(max(lat))}")
    print(f"  RSS after: {s.rss_mb():.1f}MB")
    s.p.kill()


def desync_check(binary):
    """Random incremental edits incl. emoji/CRLF; compare diagnostics with fresh open of expected text."""
    s = Lsp(binary)
    s.init()
    base = gen_doc(300).replace("Load", "Lo😀ad")
    uri, ref = "file:///sync.html", "file:///ref.html"
    s.notify(
        "textDocument/didOpen",
        {"textDocument": {"uri": uri, "languageId": "html", "version": 1, "text": base}},
    )
    s.wait_diag(uri)
    lines = base.split("\n")
    v = 1
    for _ in range(300):
        v += 1
        ln = random.randrange(len(lines))
        L = lines[ln]
        ci = random.randrange(len(L) + 1)
        ins = random.choice(["a", "😀", "é", "", '"'])
        dele = random.randint(0, min(3, len(L) - ci))
        sc, ec = utf16_len(L[:ci]), utf16_len(L[: ci + dele])
        lines[ln] = L[:ci] + ins + L[ci + dele :]
        s.notify(
            "textDocument/didChange",
            {
                "textDocument": {"uri": uri, "version": v},
                "contentChanges": [
                    {
                        "range": {
                            "start": {"line": ln, "character": sc},
                            "end": {"line": ln, "character": ec},
                        },
                        "text": ins,
                    }
                ],
            },
        )
    got = s.wait_diag(uri, v)["params"]["diagnostics"]
    s.notify(
        "textDocument/didOpen",
        {"textDocument": {"uri": ref, "languageId": "html", "version": 1, "text": "\n".join(lines)}},
    )
    exp = s.wait_diag(ref)["params"]["diagnostics"]
    key = lambda d: (
        d["range"]["start"]["line"],
        d["range"]["start"]["character"],
        d.get("code"),
        d.get("message"),
    )
    status = "OK" if sorted(map(key, got)) == sorted(map(key, exp)) else "MISMATCH"
    print(
        f"\n## incremental-sync desync check (300 random UTF-16 edits): {status} ({len(got)} vs {len(exp)} diags)"
    )
    s.p.kill()


def robustness(binary):
    print("\n## robustness (server must survive each)")
    s = Lsp(binary)
    s.init()
    uri = "file:///r.html"
    s.notify(
        "textDocument/didOpen",
        {
            "textDocument": {
                "uri": uri,
                "languageId": "html",
                "version": 1,
                "text": '<div hx-ws="connect:/x" hx-target="#a">😀</div>\r\n<p hx-swap="x">',
            }
        },
    )
    s.wait_diag(uri)
    cases = {
        "reversed range didChange": (
            "n",
            "textDocument/didChange",
            {
                "textDocument": {"uri": uri, "version": 2},
                "contentChanges": [
                    {
                        "range": {
                            "start": {"line": 1, "character": 5},
                            "end": {"line": 0, "character": 1},
                        },
                        "text": "Z",
                    }
                ],
            },
        ),
        "huge line/char": (
            "n",
            "textDocument/didChange",
            {
                "textDocument": {"uri": uri, "version": 3},
                "contentChanges": [
                    {
                        "range": {
                            "start": {"line": 4294967295, "character": 4294967295},
                            "end": {"line": 4294967295, "character": 4294967295},
                        },
                        "text": "Z",
                    }
                ],
            },
        ),
        "mid-surrogate column": (
            "r",
            "textDocument/hover",
            {"textDocument": {"uri": uri}, "position": {"line": 0, "character": 40}},
        ),
        "completion unknown uri": (
            "r",
            "textDocument/completion",
            {"textDocument": {"uri": "file:///nope"}, "position": {"line": 0, "character": 0}},
        ),
        "code action bogus diag range": (
            "r",
            "textDocument/codeAction",
            {
                "textDocument": {"uri": uri},
                "range": {
                    "start": {"line": 0, "character": 0},
                    "end": {"line": 0, "character": 0},
                },
                "context": {
                    "diagnostics": [
                        {
                            "range": {
                                "start": {"line": 9, "character": 9},
                                "end": {"line": 0, "character": 0},
                            },
                            "message": "x",
                            "code": "HTMX001",
                        }
                    ]
                },
            },
        ),
        "code action on real HTMX001": (
            "r",
            "textDocument/codeAction",
            {
                "textDocument": {"uri": uri},
                "range": {
                    "start": {"line": 0, "character": 5},
                    "end": {"line": 0, "character": 10},
                },
                "context": {
                    "diagnostics": [
                        {
                            "range": {
                                "start": {"line": 0, "character": 5},
                                "end": {"line": 0, "character": 10},
                            },
                            "message": "x",
                            "code": "HTMX001",
                        }
                    ]
                },
            },
        ),
        "invalid params type": ("r", "textDocument/hover", {"textDocument": 5}),
        "unknown method": ("r", "workspace/symbol", {"query": ""}),
    }
    for name, (kind, m, p) in cases.items():
        if kind == "n":
            s.notify(m, p)
            time.sleep(0.1)
            print(f"  {name}: {'alive' if s.alive() else 'DIED'}")
        else:
            i = s.request(m, p)
            try:
                r = s.wait_id(i, timeout=5)
                if "error" in r and r["error"]:
                    print(f"  {name}: alive, error {r['error']['message'][:60]}")
                else:
                    res_str = json.dumps(r.get("result")) if r.get("result") is not None else "null"
                    print(f"  {name}: alive, result {res_str[:110]}")
            except Exception as e:
                print(f"  {name}: {e}")

    # malformed JSON frame
    s.raw(b"Content-Length: 5\r\n\r\n{bad}")
    time.sleep(0.3)
    print(f"  malformed JSON frame: {'alive' if s.alive() else 'DIED (exit ' + str(s.p.returncode) + ')'}")
    s.p.kill()

    # shutdown then a request before exit
    s = Lsp(binary)
    s.init()
    sid = s.request("shutdown", None)
    s.wait_id(sid)
    hid = s.request(
        "textDocument/hover",
        {"textDocument": {"uri": uri}, "position": {"line": 0, "character": 0}},
    )
    time.sleep(0.3)
    resp = s.wait_id(hid)
    err = resp.get("error", {})
    code = err.get("code")
    print(
        f"  request after shutdown: {'alive, error ' + str(code) if s.alive() and code == -32600 else 'FAILED'}"
    )
    s.p.kill()


def big_minified(binary):
    s = Lsp(binary)
    s.init()
    unit = '<div id="a{i}"><button hx-get="/x" hx-target="#a{i}" hx-swap="innerHTML">b</button></div>'
    text = "".join(unit.format(i=i) for i in range(70000))  # ~6MB single line
    uri = "file:///min.html"
    t0 = time.time()
    s.notify(
        "textDocument/didOpen",
        {"textDocument": {"uri": uri, "languageId": "html", "version": 1, "text": text}},
    )
    d = s.wait_diag(uri)
    print(
        f"\n## {len(text) / 1e6:.1f}MB single-line file: didOpen->diag {ms(time.time() - t0)}, {len(d['params']['diagnostics'])} diags (over 5MB cap => silently no analysis), RSS {s.rss_mb():.0f}MB"
    )
    lat = []
    for _ in range(10):
        t0 = time.time()
        cid = s.request(
            "textDocument/completion",
            {"textDocument": {"uri": uri}, "position": {"line": 0, "character": len(text) - 30}},
        )
        s.wait_id(cid)
        lat.append(time.time() - t0)
    print(f"  completion at end of line: p50 {ms(pct(lat, .5))}")
    s.p.kill()


def main():
    parser = argparse.ArgumentParser(description="Stress harness for htmx-analyzer")
    parser.add_argument("binary", help="Path to htmx-analyzer binary")
    parser.add_argument(
        "--lines",
        default="1000,10000,50000",
        help="Comma-separated line counts (default: 1000,10000,50000)",
    )
    args = parser.parse_args()

    line_counts = [int(x.strip()) for x in args.lines.split(",") if x.strip()]
    for n in line_counts:
        bench(args.binary, n)
    desync_check(args.binary)
    robustness(args.binary)
    big_minified(args.binary)


if __name__ == "__main__":
    main()
