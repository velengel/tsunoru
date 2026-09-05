#!/usr/bin/env python3
"""Verify that main/WAL/SHM source bytes and file presence stay unchanged."""
import importlib.util
from pathlib import Path
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("runtime_shutdown_test", ROOT / "scripts/test-runtime-shutdown.py")
fixtures = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fixtures)

def file_set(database):
    return {suffix: path.read_bytes() if path.exists() else None
            for suffix in ("", "-wal", "-shm")
            for path in [Path(str(database) + suffix)]}

with tempfile.TemporaryDirectory(prefix="runtime-wal-source-test-") as temporary:
    root = Path(temporary)
    fixtures.seed(root)
    database = root / "var/tsunoru.sqlite3"
    subprocess.run([sys.executable, str(ROOT / "scripts/fixtures/leave-runtime-wal.py"), str(database)], check=True)
    Path(str(database) + "-shm").unlink()
    before = file_set(database)
    assert before["-wal"] and before["-shm"] is None, "Fixture retains WAL without SHM"
    result = subprocess.run([sys.executable, str(ROOT / "scripts/test-runtime-shutdown.py"), "--worker", str(root)],
                            stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, timeout=60)
    assert result.returncode == 0, result.stdout
    assert "PASS migrated event readable" in result.stdout, "The copied DB must retain the event committed only in WAL"
    assert file_set(database) == before, "Source main/WAL/SHM bytes and presence changed"
    print("PASS WAL source without SHM: HTTP verification leaves the complete source file set unchanged")
