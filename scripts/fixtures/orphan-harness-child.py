"""Leave an owned child in the original process group after its leader exits."""
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time

ready = Path(sys.argv[1])
if '--child' in sys.argv:
    if '--ignore-term' in sys.argv:
        signal.signal(signal.SIGTERM, signal.SIG_IGN)
    ready.write_text(json.dumps({'pid': os.getpid(), 'group': os.getpgrp()}))
    while True:
        time.sleep(1)
else:
    subprocess.Popen([sys.executable, __file__, *sys.argv[1:], '--child'],
                     stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    deadline = time.monotonic() + 5
    while not ready.exists():
        assert time.monotonic() < deadline, 'orphan fixture did not start'
        time.sleep(0.01)
    os._exit(0)
