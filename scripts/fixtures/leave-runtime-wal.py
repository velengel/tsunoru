"""Create committed WAL state without a clean connection shutdown, in a test DB."""
import os
import sqlite3
import sys

connection = sqlite3.connect(sys.argv[1])
connection.execute("PRAGMA journal_mode=WAL")
connection.execute("PRAGMA wal_autocheckpoint=0")
connection.execute("CREATE TABLE verification_wal_fixture (value TEXT NOT NULL)")
connection.execute("INSERT INTO verification_wal_fixture VALUES ('committed in WAL')")
connection.execute("INSERT INTO events (public_id, name, time_zone, organizer_capability_hash) VALUES (?, ?, ?, ?)",
                   ("00000000-0000-4000-8000-000000000001", "WAL fixture event", "Asia/Tokyo", "fixture-only-hash"))
connection.commit()
os._exit(0)
