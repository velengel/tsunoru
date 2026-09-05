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
    def __init__(self, shutdown_timeout=15):
        self.resources = ExitStack()
        self.depth = 0
        self.pending = None
        self.stops = {}
        self.directories = set()
        self.shutdown_timeout = shutdown_timeout

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
                def signal_group(sig):
                    child.poll()
                    try:
                        os.killpg(child.pid, sig)
                        return True
                    except ProcessLookupError:
                        return False
                    except PermissionError as error:
                        # A terminating leader can become a zombie between poll
                        # and killpg. Reap it, but never ignore a live-group denial.
                        child.poll()
                        try:
                            os.killpg(child.pid, 0)
                        except ProcessLookupError:
                            return False
                        raise RuntimeError(f'Cannot signal owned group {child.pid} with {sig}') from error

                def wait_group(timeout):
                    deadline = time.monotonic() + timeout
                    while True:
                        child.poll()  # Reap the leader without equating it to the group.
                        if not signal_group(0):
                            return True
                        if time.monotonic() >= deadline:
                            return False
                        time.sleep(0.02)

                signal_group(signal.SIGTERM)
                if not wait_group(self.shutdown_timeout):
                    signal_group(signal.SIGKILL)
                    if not wait_group(5):
                        raise RuntimeError('Owned process group survived forced cleanup')
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
