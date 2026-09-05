"""Owned-resource scope for regression drivers, including outer cancellation."""
from contextlib import contextmanager, ExitStack
import json
import os
import signal
import subprocess
import sys
import tempfile
import time


class Harness:
    def __init__(self):
        self.resources = ExitStack()
        self.depth = 0
        self.pending = None
        self.stops = {}
        self.directories = set()

    def terminate(self, sig, _frame):
        if self.depth:
            self.pending = self.pending or sig
            return
        for value in self.previous:
            signal.signal(value, signal.SIG_IGN)
        raise SystemExit(128 + sig)

    def __enter__(self):
        self.previous = {sig: signal.getsignal(sig) for sig in (signal.SIGINT, signal.SIGTERM)}
        for sig in self.previous:
            signal.signal(sig, self.terminate)
        return self

    def __exit__(self, *exception):
        for sig in self.previous:
            signal.signal(sig, signal.SIG_IGN)
        try:
            self.resources.close()
        finally:
            for sig, handler in self.previous.items():
                signal.signal(sig, handler)

    @contextmanager
    def publication(self):
        self.depth += 1
        try:
            yield
        finally:
            self.depth -= 1
            if not self.depth and self.pending is not None:
                sig, self.pending = self.pending, None
                self.terminate(sig, None)

    def register(self, cleanup):
        finished = False

        def once():
            nonlocal finished
            with self.publication():
                if not finished:
                    cleanup()
                    finished = True
        self.resources.callback(once)
        return once

    @contextmanager
    def temporary_directory(self, **kwargs):
        with self.publication():
            temporary = tempfile.TemporaryDirectory(**kwargs)
            self.directories.add(temporary.name)
            existing = set(self.stops)

            def cleanup():
                # A context can unwind before the outer ExitStack. Stop children
                # acquired in this scope before removing their working directory.
                try:
                    with ExitStack() as children:
                        for child in self.stops:
                            if child not in existing:
                                children.callback(self.stop, child)
                finally:
                    temporary.cleanup()
                    self.directories.discard(temporary.name)
            remove = self.register(cleanup)
        try:
            yield temporary.name
        finally:
            remove()

    def popen(self, command, **kwargs):
        with self.publication():
            kwargs['start_new_session'] = True
            child = subprocess.Popen(command, **kwargs)

            def cleanup():
                if child.poll() is None:
                    try:
                        os.killpg(child.pid, signal.SIGTERM)
                    except ProcessLookupError:
                        pass
                    try:
                        child.wait(timeout=15)
                    except subprocess.TimeoutExpired:
                        try:
                            os.killpg(child.pid, signal.SIGKILL)
                        except ProcessLookupError:
                            pass
                        child.wait(timeout=5)
            self.stops[child] = self.register(cleanup)
        if '--harness-probe' in sys.argv:
            print('harness_probe_ready=' + json.dumps({'pids': [child.pid], 'directories': sorted(self.directories)}), flush=True)
            while True:
                time.sleep(1)
        return child

    def stop(self, child):
        self.stops[child]()

    def run(self, command, *, check=False, timeout=None, **kwargs):
        child = self.popen(command, **kwargs)
        try:
            stdout, stderr = child.communicate(timeout=timeout)
        finally:
            self.stop(child)
        result = subprocess.CompletedProcess(command, child.returncode, stdout, stderr)
        if check:
            result.check_returncode()
        return result
