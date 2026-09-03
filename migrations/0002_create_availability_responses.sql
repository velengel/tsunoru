CREATE UNIQUE INDEX candidates_id_event_public_id_unique
    ON candidates (id, event_public_id);

CREATE TABLE responses (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_public_id TEXT NOT NULL,
    respondent_name TEXT NOT NULL
        CHECK (length(trim(respondent_name)) BETWEEN 1 AND 100),
    response_capability_hash TEXT NOT NULL UNIQUE
        CHECK (
            length(response_capability_hash) = 64
            AND response_capability_hash NOT GLOB '*[^0-9a-f]*'
        ),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (event_public_id) REFERENCES events(public_id) ON DELETE CASCADE,
    UNIQUE (id, event_public_id)
);

CREATE INDEX responses_event_public_id_idx
    ON responses (event_public_id);

CREATE TABLE response_availabilities (
    response_id INTEGER NOT NULL,
    candidate_id INTEGER NOT NULL,
    event_public_id TEXT NOT NULL,
    availability TEXT NOT NULL
        CHECK (availability IN ('available', 'maybe', 'unavailable')),
    PRIMARY KEY (response_id, candidate_id),
    FOREIGN KEY (response_id, event_public_id)
        REFERENCES responses(id, event_public_id) ON DELETE CASCADE,
    FOREIGN KEY (candidate_id, event_public_id)
        REFERENCES candidates(id, event_public_id) ON DELETE CASCADE
);

CREATE INDEX response_availabilities_candidate_event_idx
    ON response_availabilities (candidate_id, event_public_id);
