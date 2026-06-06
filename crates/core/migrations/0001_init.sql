-- Initial schema for knowledge-app.
-- All timestamps are ISO-8601 strings (UTC); dates are local "YYYY-MM-DD".

CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS topics (
    id         INTEGER PRIMARY KEY,
    slug       TEXT NOT NULL,
    title      TEXT NOT NULL,
    summary    TEXT NOT NULL,
    area       TEXT NOT NULL,
    level      TEXT NOT NULL,            -- senior|staff|principal
    source     TEXT NOT NULL,           -- catalog|generated
    talking_points TEXT NOT NULL DEFAULT '[]',  -- JSON array of strings
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_topics_area_level ON topics(area, level);

CREATE TABLE IF NOT EXISTS daily_sessions (
    id         INTEGER PRIMARY KEY,
    date       TEXT NOT NULL UNIQUE,    -- local YYYY-MM-DD
    topic_id   INTEGER NOT NULL REFERENCES topics(id),
    level      TEXT NOT NULL,
    completed  INTEGER NOT NULL DEFAULT 0,
    score      INTEGER                  -- correct count, NULL until completed
);

CREATE TABLE IF NOT EXISTS questions (
    id            INTEGER PRIMARY KEY,
    session_id    INTEGER REFERENCES daily_sessions(id) ON DELETE CASCADE,
    topic_id      INTEGER NOT NULL REFERENCES topics(id),
    prompt        TEXT NOT NULL,
    choices_json  TEXT NOT NULL,        -- JSON array of strings
    correct_index INTEGER NOT NULL,
    explanation   TEXT NOT NULL,
    difficulty    INTEGER NOT NULL,     -- 1..5
    created_at    TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_questions_session ON questions(session_id);

CREATE TABLE IF NOT EXISTS attempts (
    id          INTEGER PRIMARY KEY,
    question_id INTEGER NOT NULL REFERENCES questions(id) ON DELETE CASCADE,
    session_id  INTEGER NOT NULL REFERENCES daily_sessions(id) ON DELETE CASCADE,
    chosen_index INTEGER NOT NULL,
    is_correct  INTEGER NOT NULL,
    answered_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_attempts_session ON attempts(session_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_attempts_question ON attempts(question_id);

-- Singleton streak row (id is always 1).
CREATE TABLE IF NOT EXISTS streaks (
    id               INTEGER PRIMARY KEY CHECK (id = 1),
    current_streak   INTEGER NOT NULL DEFAULT 0,
    longest_streak   INTEGER NOT NULL DEFAULT 0,
    last_active_date TEXT
);
INSERT OR IGNORE INTO streaks (id, current_streak, longest_streak, last_active_date)
VALUES (1, 0, 0, NULL);
