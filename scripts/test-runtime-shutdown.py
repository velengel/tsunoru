#!/usr/bin/env python3
"""Use a fresh real-server database to test verifier SIGTERM/SIGINT cleanup."""
import importlib.util
import json
import os
from pathlib import Path
import selectors
import signal
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("runtime_verifier", ROOT / "scripts/verify-runtime.py")
VERIFIER = importlib.util.module_from_spec(SPEC)
sys.dont_write_bytecode = True
SPEC.loader.exec_module(VERIFIER)


def alive(pid):
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False


def seed(root):
    (root / "var").mkdir()
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        port = sock.getsockname()[1]
    env = {k: v for k, v in os.environ.items() if not k.startswith(("DIOXUS_", "TSUNORU_"))}
    env.update(IP="127.0.0.1", PORT=str(port))
    child = subprocess.Popen([str(VERIFIER.BINARY)], cwd=root, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        request = urllib.request.Request(f"http://127.0.0.1:{port}/api/events/get", data=json.dumps({"public_id": "00000000-0000-0000-0000-000000000000"}).encode(), method="GET", headers={"Content-Type": "application/json"})
        opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
        for _ in range(100):
            try:
                with opener.open(request, timeout=1) as response:
                    assert response.status == 200
                return
            except urllib.error.URLError:
                assert child.poll() is None
                time.sleep(0.1)
        raise RuntimeError("fixture server did not become ready")
    finally:
        child.terminate()
        try:
            child.wait(timeout=10)
        except subprocess.TimeoutExpired:
            child.kill()
            child.wait()


def test_signals():
    for sig in (signal.SIGTERM, signal.SIGINT):
        with tempfile.TemporaryDirectory(prefix="runtime-signal-test-") as tmp:
            root = Path(tmp)
            seed(root)
            runner = subprocess.Popen([sys.executable, __file__, "--worker", str(root), "--shutdown-probe"], stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
            info = None
            try:
                output = b""
                with selectors.DefaultSelector() as selector:
                    selector.register(runner.stdout, selectors.EVENT_READ)
                    deadline = time.monotonic() + 30
                    while info is None and time.monotonic() < deadline:
                        for key, _ in selector.select(timeout=1):
                            chunk = os.read(key.fileobj.fileno(), 65536)
                            assert chunk, "verifier exited before probe readiness"
                            output += chunk
                            for line in output.splitlines():
                                if line.startswith(b"shutdown_probe_ready="):
                                    info = json.loads(line.split(b"=", 1)[1])
                assert info is not None, "verifier probe timed out"
                runner.send_signal(sig)
                runner.wait(timeout=15)
                assert not alive(info["server_pid"]), f"{sig.name}: server leaked"
                assert not Path(info["temp"]).exists(), f"{sig.name}: temporary data leaked"
                assert runner.returncode == 128 + sig, (sig.name, runner.returncode)
                print(f"PASS {sig.name}: owned server and temporary data removed")
            finally:
                if runner.poll() is None:
                    runner.kill()
                    runner.wait()
                if info and alive(info["server_pid"]):
                    os.kill(info["server_pid"], signal.SIGKILL)
                runner.stdout.close()
    with tempfile.TemporaryDirectory(prefix="runtime-normal-test-") as tmp:
        root = Path(tmp)
        seed(root)
        subprocess.run([sys.executable, __file__, "--worker", str(root)], check=True)


if __name__ == "__main__":
    if "--worker" in sys.argv:
        VERIFIER.ROOT = Path(sys.argv[2])
        VERIFIER.SOURCE = VERIFIER.ROOT / "var/tsunoru.sqlite3"
        VERIFIER.main()
    else:
        test_signals()
