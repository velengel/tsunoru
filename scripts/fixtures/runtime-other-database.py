"""Run the HTTP verifier with a server intentionally using another fixture DB."""
import importlib.util
from pathlib import Path
import subprocess
import sys

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'scripts'))
spec = importlib.util.spec_from_file_location("runtime_verifier", ROOT / "scripts/verify-runtime.py")
verifier = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verifier)
verifier.ROOT = Path(sys.argv[1])
verifier.SOURCE = verifier.ROOT / "var/tsunoru.sqlite3"
real_popen = subprocess.Popen

class OtherDatabase(real_popen):
    def __init__(self, *args, **kwargs):
        kwargs["cwd"] = Path(sys.argv[2])
        super().__init__(*args, **kwargs)

verifier.subprocess.Popen = OtherDatabase
verifier.main()
