#!/usr/bin/env python3
"""Verify the built HTTP server using a disposable copy of the local database.

Run from any directory: python3 scripts/verify-runtime.py
Requires: dx build --web (or a completed dx serve --web build).
Only the child server receives test writes. No credentials are printed or saved.
"""

import hashlib
import json
import os
from pathlib import Path
import secrets
import socket
import sqlite3
import subprocess
import tempfile
import time
import urllib.error
import urllib.request


ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / "target/dx/tsunoru/debug/web/server"
SOURCE = ROOT / "var/tsunoru.sqlite3"


def check(condition, message):
    if not condition:
        raise RuntimeError(message)
    print(f"PASS {message}", flush=True)


def verify():
    check(BINARY.is_file(), "built server exists")
    check(SOURCE.is_file(), "original database exists")
    original_hash = hashlib.sha256(SOURCE.read_bytes()).digest()
    with tempfile.TemporaryDirectory(prefix="runtime-check-", dir=ROOT / "var") as tmp:
        directory = Path(tmp)
        (directory / "var").mkdir()
        with sqlite3.connect(f"file:{SOURCE}?mode=ro", uri=True) as source:
            check(source.execute("PRAGMA integrity_check").fetchone() == ("ok",), "source integrity")
            original_dump = list(source.iterdump())
            public_ids = [r[0] for r in source.execute("SELECT public_id FROM events")]
            with sqlite3.connect(directory / "var/tsunoru.sqlite3") as copy:
                source.backup(copy)
        with socket.socket() as reservation:
            reservation.bind(("127.0.0.1", 0))
            port = reservation.getsockname()[1]
        base = f"http://127.0.0.1:{port}"
        # Keep launch settings explicit; do not inherit public-origin/development settings.
        env = {k: v for k, v in os.environ.items()
               if not k.startswith(("DIOXUS_", "TSUNORU_"))}
        env.update(IP="127.0.0.1", PORT=str(port))
        opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
        with (directory / "server.log").open("w") as log:
            child = subprocess.Popen([str(BINARY)], cwd=directory, env=env,
                                     stdout=log, stderr=subprocess.STDOUT)
            try:
                def request(path, data=None, method=None, expected=200):
                    if child.poll() is not None:
                        raise RuntimeError("owned server exited; refusing HTTP requests")
                    body = None if data is None else json.dumps(data).encode()
                    req = urllib.request.Request(base + path, data=body, method=method,
                                                 headers={"Content-Type": "application/json", "Origin": base})
                    try:
                        with opener.open(req, timeout=10) as response:
                            status, raw = response.status, response.read()
                            content_type = response.headers.get("Content-Type", "")
                    except urllib.error.HTTPError as error:
                        status, raw = error.code, error.read()
                        content_type = error.headers.get("Content-Type", "")
                    if status != expected:
                        raise RuntimeError(f"{path}: expected HTTP {expected}, received HTTP {status}")
                    print(f"PASS {method or ('POST' if body else 'GET')} {path} HTTP {expected}", flush=True)
                    return json.loads(raw) if "json" in content_type else raw

                deadline = time.monotonic() + 20
                while True:
                    try:
                        request("/")
                        break
                    except (urllib.error.URLError, TimeoutError):
                        if time.monotonic() >= deadline:
                            raise RuntimeError("owned server did not become ready") from None
                        time.sleep(0.2)

                def get_event(public_id):
                    # Dioxus 0.7.10's generated JSON extractor reads a body, including GET.
                    return request("/api/events/get", {"public_id": public_id}, method="GET")

                for public_id in public_ids:
                    check(get_event(public_id)["public_id"] == public_id, "migrated event readable")

                def post(path, value, expected=200):
                    return request(path, {"input": value}, expected=expected)

                created = post("/api/events/create", {
                    "name": "Migration runtime verification", "organizer_note": "Disposable database only",
                    "time_zone": "Asia/Tokyo", "candidates": [
                        {"local_date": "2026-09-12", "local_time": "19:00"},
                        {"local_date": "2026-09-13", "local_time": "19:00"}]})
                event = created["event"]
                public_id = event["public_id"]
                authority = {"event_public_id": public_id, "organizer_capability": created["organizer_capability"]}
                check(get_event(public_id)["name"] == event["name"], "created event persisted")
                answer = {"event_public_id": public_id, "response_capability": secrets.token_hex(32),
                          "response": {"respondent_name": "Verification participant", "availabilities": [
                              {"candidate_id": candidate["id"], "availability": choice}
                              for candidate, choice in zip(event["candidates"], ["available", "maybe"])]}}
                matrix = post("/api/answers/submit", answer)
                check(matrix["responses"][0]["respondent_name"] == "Verification participant", "response persisted")
                post("/api/answers/comment", {"event_public_id": public_id,
                     "response_capability": answer["response_capability"], "comment": "Saved after migration"})
                summary = post("/api/organizer/events/summary", authority)
                check(summary["response_count"] == 1 and summary["comment_count"] == 1, "summary matches saved response and comment")
                matrix = post("/api/organizer/events/matrix", authority)
                check(matrix["responses"][0]["availabilities"] == ["available", "maybe"], "matrix matches choices")
                post("/api/organizer/events/summary", {**authority, "organizer_capability": secrets.token_hex(32)}, 404)
                candidate_id = event["candidates"][0]["id"]
                post("/api/organizer/events/decision", {**authority, "candidate_id": candidate_id})
                check(get_event(public_id)["decision"]["candidate_id"] == candidate_id, "decision persisted")
                calendar = request(f"/api/events/{public_id}/calendar.ics")
                check(b"BEGIN:VCALENDAR" in calendar and b"BEGIN:VEVENT" in calendar, "calendar download contains event")
                post("/api/answers/submit", {**answer, "response_capability": secrets.token_hex(32)}, 409)
            finally:
                if child.poll() is None:
                    child.terminate()
                    try:
                        child.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        child.kill()
                        child.wait()
                check(child.poll() is not None, "owned server stopped")
                with sqlite3.connect(f"file:{SOURCE}?mode=ro", uri=True) as source:
                    check(list(source.iterdump()) == original_dump, "original database logical contents unchanged")
                check(hashlib.sha256(SOURCE.read_bytes()).digest() == original_hash, "original database file unchanged")
    print("runtime_verification=PASS", flush=True)


if __name__ == "__main__":
    verify()
