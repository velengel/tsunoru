#!/usr/bin/env python3
"""A matching application bundle with another DB must receive no test writes."""
import importlib.util
import os
from pathlib import Path
import sqlite3
import subprocess
import sys

sys.dont_write_bytecode = True
from verification_harness import Harness

H = Harness()
ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("runtime_shutdown_test", ROOT / "scripts/test-runtime-shutdown.py")
fixtures = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fixtures)

with H, H.temporary_directory(prefix="browser-server-identity-") as temporary:
    directory = Path(temporary)
    foreign = directory / "foreign"
    foreign.mkdir()
    fixtures.seed(foreign, H)
    bundle = directory / "bundle"
    bundle.mkdir()
    (bundle / "server").symlink_to(ROOT / "scripts/fixtures/serve-other-database.py")
    (bundle / "public").symlink_to(ROOT / "target/dx/tsunoru/debug/web/public")
    (bundle / "foreign-cwd").write_text(str(foreign))
    env = {**os.environ, "TSUNORU_TEST_BUNDLE": str(bundle)}
    runner = H.popen(["node", str(ROOT / "scripts/verify-calendar-browser.mjs")],
                              cwd=ROOT, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                              text=True, start_new_session=True)
    try:
        output, _ = runner.communicate(timeout=60)
    finally:
        H.stop(runner)
    with sqlite3.connect(foreign / "var/tsunoru.sqlite3") as connection:
        count = connection.execute("SELECT count(*) FROM events").fetchone()[0]
    assert count == 0, f"Wrong database received {count} event writes"
    assert runner.returncode != 0 and "database identity" in output, output
    print("PASS matching server with another database: rejected before browser writes")
