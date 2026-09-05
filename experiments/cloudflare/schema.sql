-- Synthetic model only. Never apply to a TSUNORU production database.
CREATE TABLE events(id TEXT PRIMARY KEY, name TEXT NOT NULL);
CREATE TABLE candidates(id TEXT PRIMARY KEY, event_id TEXT NOT NULL REFERENCES events(id));
CREATE TABLE responses(id TEXT PRIMARY KEY, event_id TEXT NOT NULL REFERENCES events(id), answer TEXT NOT NULL CHECK(answer IN ('yes','no')));
CREATE TABLE sessions(id TEXT PRIMARY KEY, active INTEGER NOT NULL CHECK(active IN (0,1)));
CREATE TABLE series(id TEXT PRIMARY KEY, tail TEXT NOT NULL REFERENCES events(id));
CREATE TABLE continuations(series_id TEXT NOT NULL REFERENCES series(id), expected_tail TEXT NOT NULL, new_tail TEXT NOT NULL REFERENCES events(id), UNIQUE(series_id,expected_tail));
CREATE TABLE assertions(ok INTEGER NOT NULL CHECK(ok=1));
