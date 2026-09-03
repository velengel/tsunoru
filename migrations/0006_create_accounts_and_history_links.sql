CREATE TABLE accounts (
    id INTEGER PRIMARY KEY,
    login_id TEXT NOT NULL UNIQUE
        CHECK (
            length(login_id) BETWEEN 3 AND 32
            AND login_id = lower(trim(login_id))
            AND login_id GLOB '[a-z0-9]*'
            AND login_id NOT GLOB '*[^a-z0-9._-]*'
        ),
    password_hash_phc TEXT NOT NULL
        CHECK (
            length(password_hash_phc) BETWEEN 1 AND 512
            AND instr(password_hash_phc, char(0)) = 0
        ),
    created_at INTEGER NOT NULL
);

CREATE TABLE account_sessions (
    token_hash BLOB PRIMARY KEY NOT NULL
        CHECK (typeof(token_hash) = 'blob' AND length(token_hash) = 32),
    account_id INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    last_seen_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE,
    CHECK (last_seen_at >= created_at),
    CHECK (expires_at > created_at),
    CHECK (last_seen_at <= expires_at)
);

ALTER TABLE events
ADD COLUMN organizer_account_id INTEGER DEFAULT NULL
    REFERENCES accounts(id) ON DELETE SET NULL;

ALTER TABLE responses
ADD COLUMN respondent_account_id INTEGER DEFAULT NULL
    REFERENCES accounts(id) ON DELETE SET NULL;

CREATE INDEX account_sessions_account_idx
    ON account_sessions (account_id);

CREATE INDEX events_organizer_history_idx
    ON events (organizer_account_id, created_at DESC, public_id DESC)
    WHERE organizer_account_id IS NOT NULL;

CREATE INDEX responses_participant_history_idx
    ON responses (respondent_account_id, event_public_id, id DESC)
    WHERE respondent_account_id IS NOT NULL;
