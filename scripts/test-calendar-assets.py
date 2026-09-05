#!/usr/bin/env python3
"""Exercise the real checker with valid, stale and interrupted local responses."""
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import os
from pathlib import Path
import signal
import subprocess
import sys
import threading
import time

sys.dont_write_bytecode = True
from verification_harness import Harness

ROOT = Path(__file__).resolve().parents[1]
HTML = '<link href="/calendar.css" rel="stylesheet">カレンダーの日を押すと candidate-direct-entry'
CSS = '.candidate-calendar-toolbar .candidate-calendar-grid .candidate-calendar-day {grid-template-columns:repeat(7,minmax(0,1fr))}'
requested = threading.Event()
release = threading.Event()
mode = 'valid'

class Handler(BaseHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def do_GET(self):
        requested.set()
        if mode == 'interrupt':
            release.wait(timeout=10)
        css = self.path == '/calendar.css'
        self.send_response(200)
        self.send_header('Content-Type', 'text/css' if css and mode != 'wrong-type' else 'text/html; charset=utf-8')
        self.end_headers()
        body = CSS if css else HTML
        if mode == 'stale' and css:
            body = 'body { color: black; }'
        if mode == 'missing-html' and not css:
            body = '<link href="/calendar.css" rel="stylesheet">'
        try:
            self.wfile.write(body.encode())
        except (BrokenPipeError, ConnectionResetError):
            pass

with Harness() as owner, ThreadingHTTPServer(('127.0.0.1', 0), Handler) as server:
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        origin = f'http://127.0.0.1:{server.server_port}'
        command = ['sh', str(ROOT / 'scripts/verify_served_calendar_assets.sh'), origin]
        for sig in (signal.SIGTERM, signal.SIGINT):
            with owner.temporary_directory(prefix='asset-check-test-') as temporary:
                directory = Path(temporary)
                probe = directory / 'publication.json'
                mode = 'interrupt'
                requested.clear()
                release.clear()
                env = {**os.environ, 'TMPDIR': temporary, 'TSUNORU_ASSET_TEST_PROBE': str(probe),
                       'PATH': str(ROOT / 'scripts/fixtures/asset-process-tools') + os.pathsep + os.environ['PATH']}
                runner = owner.popen(command, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
                try:
                    deadline = time.monotonic() + 10
                    while not probe.exists() and not requested.is_set():
                        assert runner.poll() is None, 'checker exited before readiness'
                        assert time.monotonic() < deadline, 'checker readiness timed out'
                        time.sleep(0.01)
                    runner.send_signal(sig)
                    probe.with_suffix('.release').touch()
                    runner.wait(timeout=10)
                    assert not list(directory.glob('tsunoru-calendar-assets.*')), f'{sig.name}: acquisition left disposable data behind'
                    assert runner.returncode in (-sig, 128 + sig), runner.returncode
                    print(f'PASS real asset checker {sig.name}: no disposable data remains')
                finally:
                    release.set()
                    owner.stop(runner)
                    runner.stdout.close()
        for mode in ('valid', 'stale', 'wrong-type', 'missing-html'):
            result = owner.run(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, timeout=10)
            assert (result.returncode == 0) == (mode == 'valid'), result.stdout
            print(f'PASS real asset checker {mode}: expected verification result')
    finally:
        release.set()
        server.shutdown()
        thread.join(timeout=5)
