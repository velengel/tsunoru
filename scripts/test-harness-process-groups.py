"""A finished leader must not leave owned group members alive."""
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time

sys.dont_write_bytecode = True
from verification_harness import Harness

ROOT = Path(__file__).resolve().parents[1]

def alive(pid):
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False

with Harness(shutdown_timeout=0.1) as owner:
    for resist in (False, True):
        with owner.temporary_directory(prefix='group-cleanup-test-') as temporary:
            ready = Path(temporary) / 'child.json'
            child = owner.popen([sys.executable, str(ROOT / 'scripts/fixtures/orphan-harness-child.py'), str(ready),
                                 *(['--ignore-term'] if resist else [])], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            info = None
            try:
                child.wait(timeout=10)
                info = json.loads(ready.read_text())
                assert info['group'] == child.pid and os.getpgid(info['pid']) == child.pid
                assert alive(info['pid']), 'Fixture child survives its leader'
                owner.stop(child)
                assert not alive(info['pid']), 'Owned process-group member survived Harness.stop()'
                print(f'PASS Python exited leader, ignore TERM={resist}: group reclaimed', flush=True)
            finally:
                # Reclaim only the fixture member whose group we have validated.
                if info and alive(info['pid']) and os.getpgid(info['pid']) == child.pid:
                    os.kill(info['pid'], signal.SIGKILL)
                    deadline = time.monotonic() + 5
                    while alive(info['pid']) and time.monotonic() < deadline:
                        time.sleep(0.01)

with Harness() as owner:
    for resist in (False, True):
        with owner.temporary_directory(prefix='group-cleanup-test-') as temporary:
            ready = Path(temporary) / 'child.json'
            try:
                owner.run(['node', str(ROOT / 'scripts/fixtures/node-harness-process-group.mjs'), sys.executable, str(ready),
                           *(['--ignore-term'] if resist else [])], check=True, timeout=10)
            finally:
                if ready.exists():
                    info = json.loads(ready.read_text())
                    if alive(info['pid']) and os.getpgid(info['pid']) == info['group']:
                        os.kill(info['pid'], signal.SIGKILL)
