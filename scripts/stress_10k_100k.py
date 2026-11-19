#!/usr/bin/env python3
import json
import os
import queue
import subprocess
import sys
import threading
import time

def ms(t):
    return f"{t * 1000:.1f}ms"

def pct(xs, p):
    xs = sorted(xs)
    return xs[min(len(xs) - 1, int(len(xs) * p))]

class LspClient:
    def __init__(self, binary_path):
        self.proc = subprocess.Popen(
            [binary_path, "--stdio"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
        )
        self.q = queue.Queue()
        self.next_id = 1
        self.reader_thread = threading.Thread(target=self._read_loop, daemon=True)
        self.reader_thread.start()

    def _read_loop(self):
        stdout = self.proc.stdout
        while True:
            header = b""
            while not header.endswith(b"\r\n\r\n"):
                c = stdout.read(1)
                if not c:
                    self.q.put(None)
                    return
                header += c
            lines = header.decode("latin1").split("\r\n")
            length = 0
            for line in lines:
                if line.lower().startswith("content-length:"):
                    length = int(line.split(":")[1].strip())
            body = stdout.read(length)
            msg = json.loads(body.decode("utf-8"))
            self.q.put(msg)

    def send(self, msg):
        body = json.dumps(msg).encode("utf-8")
        header = f"Content-Length: {len(body)}\r\n\r\n".encode("utf-8")
        self.proc.stdin.write(header + body)
        self.proc.stdin.flush()

    def notify(self, method, params):
        self.send({"jsonrpc": "2.0", "method": method, "params": params})

    def request(self, method, params):
        req_id = self.next_id
        self.next_id += 1
        self.send({"jsonrpc": "2.0", "id": req_id, "method": method, "params": params})
        return req_id

    def wait_for(self, predicate, timeout=120):
        deadline = time.time() + timeout
        while time.time() < deadline:
            try:
                msg = self.q.get(timeout=max(0.1, deadline - time.time()))
                if msg is None:
                    raise RuntimeError("Server process died")
                if predicate(msg):
                    return msg
            except queue.Empty:
                pass
        raise TimeoutError(f"Timed out waiting for message")

    def wait_id(self, req_id, timeout=120):
        return self.wait_for(lambda m: m.get("id") == req_id, timeout=timeout)

    def wait_diag(self, uri, version=None, timeout=120):
        return self.wait_for(
            lambda m: m.get("method") == "textDocument/publishDiagnostics"
            and m.get("params", {}).get("uri") == uri
            and (version is None or m.get("params", {}).get("version") == version),
            timeout=timeout,
        )

    def rss_mb(self):
        try:
            out = subprocess.run(
                ["ps", "-o", "rss=", "-p", str(self.proc.pid)],
                capture_output=True,
                text=True,
            ).stdout.strip()
            return int(out) / 1024 if out else 0.0
        except Exception:
            return 0.0

    def close(self):
        try:
            self.proc.terminate()
            self.proc.wait(timeout=2)
        except Exception:
            self.proc.kill()

def generate_document(target_lines, compact=False):
    out = ["<!DOCTYPE html>", "<html>", "<head><title>Stress Document</title></head>", "<body>"]
    i = 0
    if compact:
        # Compact lines (~40 bytes/line) to fit 100k lines under 5MB DoS limit (approx 3.9 MB)
        while len(out) < target_lines - 2:
            out.append(f'<div id="row-{i}">')
            out.append(f'  <button hx-get="/api/{i}" hx-target="#row-{i}" hx-swap="outerHTML">Item {i}</button>')
            out.append(f'  <span hx-sse="connect:/events/{i}" hx-swap="invalidSwap">msg</span>')
            out.append('</div>')
            i += 1
    else:
        # Standard realistic lines (~65 bytes/line)
        while len(out) < target_lines - 2:
            out.append(f'<div id="row-{i}" class="table-row">')
            out.append(f'  <button hx-get="/api/items/{i}" hx-target="#row-{i}" hx-swap="outerHTML swap:500ms">Item {i}</button>')
            out.append(f'  <input hx-post="/api/check/{i}" hx-trigger="keyup changed delay:200ms" name="val{i}" id="input-{i}">')
            out.append('</div>')
            i += 1
    out.append("</body>")
    out.append("</html>")
    return "\n".join(out[:target_lines]) + "\n", i

def run_stress_test(binary_path, lines, compact=False, label=""):
    title = f"STRESS TEST: {lines:,} LINES OF CODE" + (f" ({label})" if label else "")
    print(f"\n{'='*70}")
    print(title)
    print(f"{'='*70}")

    print(f"Generating synthetic {lines:,}-line HTML document with HTMX attributes...")
    t_gen_start = time.time()
    doc_text, item_count = generate_document(lines, compact=compact)
    t_gen = time.time() - t_gen_start
    byte_size = len(doc_text.encode('utf-8'))
    mb_size = byte_size / (1024 * 1024)
    print(f"-> Generated {lines:,} lines ({mb_size:.2f} MB / {byte_size:,} bytes, {item_count:,} HTMX items) in {ms(t_gen)}")

    client = LspClient(binary_path)
    
    # 1. Initialize
    init_id = client.request("initialize", {
        "processId": os.getpid(),
        "rootUri": None,
        "capabilities": {
            "textDocument": {
                "hover": {"contentFormat": ["markdown", "plaintext"]},
                "completion": {"completionItem": {"snippetSupport": True}},
            }
        }
    })
    client.wait_id(init_id)
    client.notify("initialized", {})
    rss_init = client.rss_mb()
    print(f"-> LSP initialized. Initial RSS: {rss_init:.1f} MB")

    uri = f"file:///stress_{lines}_{'compact' if compact else 'std'}.html"

    # 2. didOpen & Diagnostics
    t_open_start = time.time()
    client.notify("textDocument/didOpen", {
        "textDocument": {
            "uri": uri,
            "languageId": "html",
            "version": 1,
            "text": doc_text
        }
    })
    diag_msg = client.wait_diag(uri, version=1, timeout=180)
    t_open = time.time() - t_open_start
    diags = diag_msg.get("params", {}).get("diagnostics", [])
    rss_after_open = client.rss_mb()
    throughput_lines_sec = lines / t_open if t_open > 0 else 0
    throughput_mb_sec = mb_size / t_open if t_open > 0 else 0
    print(f"-> didOpen -> Diagnostics: {ms(t_open)} ({len(diags):,} diagnostics emitted, RSS: {rss_after_open:.1f} MB)")
    print(f"   Throughput: {throughput_lines_sec:,.0f} lines/sec ({throughput_mb_sec:.2f} MB/sec)")

    # 3. Incremental didChange (Keystroke simulation)
    change_latencies = []
    num_keystrokes = 20
    print(f"-> Testing {num_keystrokes} incremental edits (typing in middle of document)...")
    for k in range(num_keystrokes):
        v = 2 + k
        target_line = lines // 2 + k
        t0 = time.time()
        client.notify("textDocument/didChange", {
            "textDocument": {"uri": uri, "version": v},
            "contentChanges": [
                {
                    "range": {
                        "start": {"line": target_line, "character": 0},
                        "end": {"line": target_line, "character": 0}
                    },
                    "text": " "
                }
            ]
        })
        client.wait_diag(uri, version=v, timeout=120)
        change_latencies.append(time.time() - t0)

    p50_edit = pct(change_latencies, 0.50)
    p95_edit = pct(change_latencies, 0.95)
    max_edit = max(change_latencies)
    print(f"   didChange -> Diagnostics latency: p50={ms(p50_edit)}, p95={ms(p95_edit)}, max={ms(max_edit)}")

    # 4. Hover Queries
    hover_latencies = []
    num_hovers = 30
    print(f"-> Testing {num_hovers} hover requests across the document...")
    for h in range(num_hovers):
        target_line = (h * (lines // num_hovers)) + 5
        target_col = 10 # on "hx-get"
        t0 = time.time()
        req_id = client.request("textDocument/hover", {
            "textDocument": {"uri": uri},
            "position": {"line": target_line, "character": target_col}
        })
        resp = client.wait_id(req_id, timeout=60)
        hover_latencies.append(time.time() - t0)

    p50_hover = pct(hover_latencies, 0.50)
    p95_hover = pct(hover_latencies, 0.95)
    max_hover = max(hover_latencies)
    print(f"   Hover latency: p50={ms(p50_hover)}, p95={ms(p95_hover)}, max={ms(max_hover)}")

    # 5. Autocompletion Queries inside hx-target
    comp_latencies = []
    num_comps = 20
    print(f"-> Testing {num_comps} autocompletion requests inside hx-target attributes...")
    for c in range(num_comps):
        target_line = (c * (lines // num_comps)) + 5
        target_col = 45 # inside hx-target="#...
        t0 = time.time()
        req_id = client.request("textDocument/completion", {
            "textDocument": {"uri": uri},
            "position": {"line": target_line, "character": target_col}
        })
        resp = client.wait_id(req_id, timeout=60)
        comp_latencies.append(time.time() - t0)

    p50_comp = pct(comp_latencies, 0.50)
    p95_comp = pct(comp_latencies, 0.95)
    max_comp = max(comp_latencies)
    rss_final = client.rss_mb()
    print(f"   Autocompletion latency: p50={ms(p50_comp)}, p95={ms(p95_comp)}, max={ms(max_comp)}")
    print(f"-> Final Peak RSS: {rss_final:.1f} MB (RAM growth: +{rss_final - rss_init:.1f} MB)")

    client.close()
    return {
        "lines": lines,
        "bytes": byte_size,
        "mb": mb_size,
        "diags": len(diags),
        "open_time": t_open,
        "throughput_lines_sec": throughput_lines_sec,
        "throughput_mb_sec": throughput_mb_sec,
        "p50_edit": p50_edit,
        "p95_edit": p95_edit,
        "p50_hover": p50_hover,
        "p50_comp": p50_comp,
        "rss_initial": rss_init,
        "rss_final": rss_final,
    }

def main():
    binary = "/Users/nemesis/Projects/my-projects/htmx-analyzer/target/release/htmx-analyzer"
    if not os.path.isfile(binary):
        print(f"Error: binary {binary} not found")
        sys.exit(1)

    print("Running Ultimate Stress Tests on htmx-analyzer release binary...")
    res_10k = run_stress_test(binary, 10000, compact=False, label="Full AST & Validation")
    res_100k_full = run_stress_test(binary, 100000, compact=True, label="Full Validation under 5MB Cap")

    print("\n" + "="*80)
    print("FINAL BENCHMARK SUMMARY TABLE")
    print("="*80)
    print(f"{'Metric':<35} | {'10,000 Lines':<18} | {'100,000 Lines (Full AST)':<24}")
    print("-" * 83)
    print(f"{'File Size':<35} | {res_10k['mb']:.2f} MB ({res_10k['bytes']:,} B) | {res_100k_full['mb']:.2f} MB ({res_100k_full['bytes']:,} B)")
    print(f"{'Diagnostics Emitted':<35} | {res_10k['diags']:<18,} | {res_100k_full['diags']:<24,}")
    print(f"{'didOpen -> Diagnostics':<35} | {ms(res_10k['open_time']):<18} | {ms(res_100k_full['open_time']):<24}")
    print(f"{'Throughput (Lines/sec)':<35} | {res_10k['throughput_lines_sec']:>14,.0f}/s | {res_100k_full['throughput_lines_sec']:>20,.0f}/s")
    print(f"{'Throughput (MB/sec)':<35} | {res_10k['throughput_mb_sec']:>13.2f}MB/s | {res_100k_full['throughput_mb_sec']:>19.2f}MB/s")
    print(f"{'Incremental didChange (p50)':<35} | {ms(res_10k['p50_edit']):<18} | {ms(res_100k_full['p50_edit']):<24}")
    print(f"{'Incremental didChange (p95)':<35} | {ms(res_10k['p95_edit']):<18} | {ms(res_100k_full['p95_edit']):<24}")
    print(f"{'Hover Latency (p50)':<35} | {ms(res_10k['p50_hover']):<18} | {ms(res_100k_full['p50_hover']):<24}")
    print(f"{'Autocompletion Latency (p50)':<35} | {ms(res_10k['p50_comp']):<18} | {ms(res_100k_full['p50_comp']):<24}")
    print(f"{'Peak Memory (RSS)':<35} | {res_10k['rss_final']:.1f} MB            | {res_100k_full['rss_final']:.1f} MB")
    print("="*80)

if __name__ == "__main__":
    main()
