CREATE TABLE event_decisions (
    event_public_id TEXT PRIMARY KEY NOT NULL,
    candidate_id INTEGER NOT NULL,
    decided_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (event_public_id)
        REFERENCES events(public_id) ON DELETE CASCADE,
    FOREIGN KEY (candidate_id, event_public_id)
        REFERENCES candidates(id, event_public_id) ON DELETE RESTRICT
);
