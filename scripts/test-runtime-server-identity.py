#!/usr/bin/env python3
"""A live HTTP verifier child with the wrong database must receive no writes."""
import importlib.util
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

with H, H.temporary_directory(prefix="runtime-server-identity-") as temporary:
    own = Path(temporary) / "own"
    foreign = Path(temporary) / "foreign"
    for directory in (own, foreign):
        directory.mkdir()
        fixtures.seed(directory, H)
    result = H.run([sys.executable, str(ROOT / "scripts/fixtures/runtime-other-database.py"), str(own), str(foreign)],
                            stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, timeout=60)
    with sqlite3.connect(foreign / "var/tsunoru.sqlite3") as connection:
        count = connection.execute("SELECT count(*) FROM events").fetchone()[0]
    assert count == 0, f"Wrong database received {count} event writes"
    assert result.returncode != 0 and "database identity" in result.stdout, result.stdout
    print("PASS HTTP verifier with another database: rejected before test writes")
