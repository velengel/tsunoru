#!/usr/bin/env python3
"""Execute the real built server against the test harness's other database."""
import os
from pathlib import Path
import sys

repository = Path(__file__).resolve().parents[2]
fixture_bundle = Path(sys.argv[0]).absolute().parent
other_root = Path((fixture_bundle / "foreign-cwd").read_text().strip())
binary = repository / "target/dx/tsunoru/debug/web/server"
os.chdir(other_root)
os.execve(str(binary), [str(binary)], os.environ)
