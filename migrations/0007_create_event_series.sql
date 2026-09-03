CREATE UNIQUE INDEX events_public_id_organizer_account_idx
    ON events (public_id, organizer_account_id);

CREATE TABLE event_series (
    id INTEGER PRIMARY KEY,
    owner_account_id INTEGER NOT NULL,
    display_name TEXT NOT NULL
        CHECK (
            length(trim(display_name)) BETWEEN 1 AND 100
            AND instr(display_name, char(0)) = 0
        ),
    created_at INTEGER NOT NULL,
    FOREIGN KEY (owner_account_id) REFERENCES accounts(id) ON DELETE CASCADE,
    UNIQUE (id, owner_account_id)
);

CREATE TABLE event_series_members (
    series_id INTEGER NOT NULL,
    owner_account_id INTEGER NOT NULL,
    event_public_id TEXT PRIMARY KEY NOT NULL,
    position INTEGER NOT NULL CHECK (position >= 0),
    FOREIGN KEY (owner_account_id) REFERENCES accounts(id) ON DELETE CASCADE,
    FOREIGN KEY (series_id, owner_account_id)
        REFERENCES event_series(id, owner_account_id) ON DELETE CASCADE,
    FOREIGN KEY (event_public_id, owner_account_id)
        REFERENCES events(public_id, organizer_account_id) ON DELETE CASCADE,
    UNIQUE (series_id, position)
);

CREATE INDEX event_series_owner_idx
    ON event_series (owner_account_id, id DESC);
