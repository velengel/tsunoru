CREATE TABLE events (
    public_id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    organizer_note TEXT,
    time_zone TEXT NOT NULL CHECK (length(trim(time_zone)) > 0),
    organizer_capability_hash TEXT NOT NULL CHECK (length(trim(organizer_capability_hash)) > 0),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE candidates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_public_id TEXT NOT NULL,
    position INTEGER NOT NULL CHECK (position >= 0),
    local_date TEXT NOT NULL CHECK (length(local_date) = 10),
    local_time TEXT NOT NULL CHECK (length(local_time) = 5),
    FOREIGN KEY (event_public_id) REFERENCES events(public_id) ON DELETE CASCADE,
    UNIQUE (event_public_id, position),
    UNIQUE (event_public_id, local_date, local_time)
);
