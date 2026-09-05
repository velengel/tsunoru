#!/usr/bin/env python3
"""Terminate outer drivers and verify their owned processes and paths disappear."""
import json
import os
from pathlib import Path
import selectors
import shutil
import signal
import subprocess
import sys
import tempfile
import time

from verification_harness import Harness

H = Harness()
ROOT = Path(__file__).resolve().parents[1]
TARGETS = [
    [sys.executable, "scripts/test-runtime-shutdown.py"],
    [sys.executable, "scripts/test-runtime-source-snapshot.py"],
    [sys.executable, "scripts/test-runtime-server-identity.py"],
    [sys.executable, "scripts/test-browser-server-identity.py"],
    ["node", "scripts/test-calendar-browser-shutdown.mjs"],
]

def alive(pid):
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False

def descendants(pid):
    pairs = [tuple(map(int, line.split())) for line in
             subprocess.check_output(["ps", "-axo", "pid=,ppid="], text=True).splitlines()]
    found = {pid}
    for _ in pairs:
        for child, parent in pairs:
            if parent in found:
                found.add(child)
    return found - {pid}

def owned_directory(value):
    path = Path(value).resolve()
    prefixes = ("runtime-signal-test-", "runtime-wal-source-test-", "runtime-server-identity-", "browser-server-identity-", "calendar-browser-")
    assert path.name.startswith(prefixes), path
    assert path.parent in (Path(tempfile.gettempdir()).resolve(), (ROOT / "var").resolve()), path
    return path

def main():
    for command in TARGETS:
        for sig in (signal.SIGTERM, signal.SIGINT):
            runner = H.popen([*command, "--harness-probe"], cwd=ROOT, stdout=subprocess.PIPE,
                                      stderr=subprocess.STDOUT, start_new_session=True)
            info = None
            owned_pids = []
            paths = []
            try:
                output = b""
                with selectors.DefaultSelector() as selector:
                    selector.register(runner.stdout, selectors.EVENT_READ)
                    deadline = time.monotonic() + 30
                    while info is None and time.monotonic() < deadline:
                        for key, _ in selector.select(timeout=1):
                            chunk = os.read(key.fileobj.fileno(), 65536)
                            assert chunk, output.decode(errors="replace")
                            output += chunk
                            for line in output.splitlines():
                                if line.startswith(b"harness_probe_ready="):
                                    info = json.loads(line.split(b"=", 1)[1])
                assert info is not None, "outer harness probe timed out"
                assert set(info["pids"]) <= descendants(runner.pid), "Probe PIDs belong to this invocation"
                owned_pids = info["pids"]
                paths = [owned_directory(path) for path in info["directories"]]
                runner.send_signal(sig)
                runner.wait(timeout=25)
                assert all(not alive(pid) for pid in info["pids"]), f"{command[-1]} {sig.name}: descendant leaked"
                assert all(not path.exists() for path in paths), f"{command[-1]} {sig.name}: directory leaked"
                assert runner.returncode == 128 + sig, (command, runner.returncode)
                print(f"PASS outer {Path(command[-1]).name} {sig.name}: owned processes and directories removed")
            finally:
                H.stop(runner)
                for pid in reversed(owned_pids):
                    if alive(pid):
                        try:
                            os.kill(pid, signal.SIGKILL)
                        except ProcessLookupError:
                            pass
                for path in paths:
                    shutil.rmtree(path, ignore_errors=True)
                runner.stdout.close()

if __name__ == "__main__":
    with H:
        main()
