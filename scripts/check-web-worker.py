#!/usr/bin/env python3
"""Run Wasm worker or automatic-storage checks in a temporary headless Firefox profile."""
import argparse
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import secrets
import shutil
import subprocess
import tempfile
import threading
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parents[1]


def default_firefox():
    executable = shutil.which("firefox") or "firefox"
    direct = Path("/snap/firefox/current/usr/lib/firefox/firefox")
    try:
        with Path(executable).open("rb") as binary:
            wrapper = binary.read(512)
        if direct.is_file() and wrapper.startswith(b"#!") and b"/snap/bin/firefox" in wrapper:
            return str(direct)
    except OSError:
        pass
    return executable


def verify(dist, firefox, timeout, prefix, smoke_name="worker"):
    if not prefix.startswith("/") or not prefix.endswith("/"):
        raise ValueError("The application prefix must start and end with /")
    if not (dist / prefix.strip("/") / "index.html").is_file():
        raise RuntimeError("Build the site and Wasm app before running browser checks")
    done = threading.Event()
    report = {}
    token = secrets.token_hex(16)
    smoke = ROOT / f"crates/op-app/tests/{smoke_name}-smoke.html"
    # Exercise the same application policy that the Pages build ships.
    header_file = dist / "_headers"
    if not header_file.is_file():
        header_file = ROOT / "website/public/_headers"
    headers = header_file.read_text().split("/app/*", 1)[1]
    csp = next(line.split(":", 1)[1].strip() for line in headers.splitlines() if "Content-Security-Policy:" in line)

    class Handler(SimpleHTTPRequestHandler):
        def __init__(self, *args, **kwargs):
            super().__init__(*args, directory=str(dist), **kwargs)

        def log_message(self, *_args):
            pass

        def end_headers(self):
            if self.path.startswith(prefix):
                self.send_header("Content-Security-Policy", csp)
            super().end_headers()

        def do_GET(self):
            if urlsplit(self.path).path == f"{prefix}{smoke_name}-smoke.html":
                data = smoke.read_bytes()
                self.send_response(200)
                self.send_header("Content-Type", "text/html; charset=utf-8")
                self.send_header("Content-Length", str(len(data)))
                self.end_headers()
                self.wfile.write(data)
                return
            super().do_GET()

        def do_POST(self):
            if self.path != f"/result?token={token}":
                self.send_error(404)
                return
            length = int(self.headers.get("Content-Length", "0"))
            if not 0 < length <= 65536:
                self.send_error(413)
                return
            report.update(json.loads(self.rfile.read(length)))
            self.send_response(200)
            self.end_headers()
            done.set()

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    server.daemon_threads = True
    threading.Thread(target=server.serve_forever, daemon=True).start()
    try:
        # Snap-packaged Firefox has a private /tmp; use the ignored workspace
        # target directory so the browser sees the same profile path as Python.
        profiles = ROOT / "target"
        profiles.mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(prefix="open-process-firefox-", dir=profiles) as profile:
            env = {**os.environ, "XDG_CACHE_HOME": profile, "MOZ_CRASHREPORTER_DISABLE": "1"}
            with tempfile.TemporaryFile(mode="w+") as log:
                browser = subprocess.Popen([
                    firefox, "--headless", "--no-remote", "--profile", profile,
                    f"http://127.0.0.1:{server.server_port}{prefix}{smoke_name}-smoke.html?report={token}",
                ], stdout=log, stderr=log, env=env)
                try:
                    received = done.wait(timeout)
                finally:
                    browser.terminate()
                    try:
                        browser.wait(5)
                    except subprocess.TimeoutExpired:
                        browser.kill()
                        browser.wait()
                if not received:
                    log.seek(0)
                    raise RuntimeError(f"Firefox did not report within {timeout}s:\n{log.read()[-4000:]}")
    finally:
        server.shutdown()
        server.server_close()
    print(json.dumps(report, indent=2))
    if not report.get("ok"):
        raise RuntimeError("The browser checks failed")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dist", type=Path, default=ROOT / "website/dist")
    parser.add_argument("--firefox", default=default_firefox())
    parser.add_argument("--timeout", type=int, default=50)
    parser.add_argument("--app-prefix", default="/app/")
    parser.add_argument("--smoke", choices=["worker", "autosave"], default="worker")
    args = parser.parse_args()
    verify(args.dist.resolve(), args.firefox, args.timeout, args.app_prefix, args.smoke)
