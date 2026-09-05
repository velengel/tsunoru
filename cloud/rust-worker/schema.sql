CREATE TABLE IF NOT EXISTS events (id TEXT PRIMARY KEY, name TEXT NOT NULL, organizer_capability TEXT NOT NULL, response_capability TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS answers (event_id TEXT NOT NULL REFERENCES events(id), respondent TEXT NOT NULL, availability TEXT NOT NULL, PRIMARY KEY(event_id, respondent));
