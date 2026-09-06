-- Fresh isolated D1 only. Fail on existing tables. This is not a migration.
CREATE TABLE events (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL CHECK(length(name) BETWEEN 1 AND 100),
    organizer_note TEXT CHECK(organizer_note IS NULL OR length(organizer_note) <= 500),
    time_zone TEXT NOT NULL CHECK(length(time_zone) BETWEEN 1 AND 64),
    organizer_capability_hash TEXT NOT NULL CHECK(length(organizer_capability_hash) = 64),
    creation_payload_hash TEXT NOT NULL CHECK(length(creation_payload_hash) = 64)
);
CREATE TABLE candidates (
    event_id TEXT NOT NULL REFERENCES events(id),
    id TEXT NOT NULL,
    local_date TEXT NOT NULL CHECK(local_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
    local_time TEXT NOT NULL CHECK(local_time GLOB '[0-9][0-9]:[0-9][0-9]'),
    PRIMARY KEY(event_id, id),
    UNIQUE(event_id, local_date, local_time)
);
CREATE TABLE responses (
    id TEXT PRIMARY KEY NOT NULL DEFAULT (lower(hex(randomblob(16)))),
    event_id TEXT NOT NULL REFERENCES events(id),
    response_capability_hash TEXT NOT NULL UNIQUE CHECK(length(response_capability_hash) = 64),
    respondent_name TEXT NOT NULL CHECK(length(respondent_name) BETWEEN 1 AND 100),
    payload_hash TEXT NOT NULL CHECK(length(payload_hash) = 64),
    UNIQUE(event_id, id)
);
CREATE TABLE answers (
    event_id TEXT NOT NULL,
    response_id TEXT NOT NULL,
    candidate_id TEXT NOT NULL,
    availability TEXT NOT NULL CHECK(availability IN ('available', 'maybe', 'unavailable')),
    PRIMARY KEY(response_id, candidate_id),
    FOREIGN KEY(event_id, response_id) REFERENCES responses(event_id, id),
    FOREIGN KEY(event_id, candidate_id) REFERENCES candidates(event_id, id)
);
