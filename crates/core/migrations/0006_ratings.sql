-- Elo ratings: per-topic learner skill + item difficulty (and a '_global' row
-- for overall skill). Updated after every answer to drive adaptive selection.
CREATE TABLE IF NOT EXISTS topic_ratings (
    slug       TEXT PRIMARY KEY,
    skill      REAL NOT NULL,
    difficulty REAL NOT NULL,
    attempts   INTEGER NOT NULL DEFAULT 0,
    streak     INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL
);
