-- Spaced-repetition schedule (SM-2) per question.
CREATE TABLE IF NOT EXISTS reviews (
    question_id   INTEGER PRIMARY KEY REFERENCES questions(id) ON DELETE CASCADE,
    ease_factor   REAL NOT NULL DEFAULT 2.5,
    interval_days INTEGER NOT NULL DEFAULT 0,
    repetitions   INTEGER NOT NULL DEFAULT 0,
    due_date      TEXT NOT NULL,                 -- YYYY-MM-DD next due
    last_reviewed TEXT
);
CREATE INDEX IF NOT EXISTS idx_reviews_due ON reviews(due_date);

-- User-synthesized custom topics, merged with the bundled catalog.
CREATE TABLE IF NOT EXISTS custom_topics (
    slug           TEXT PRIMARY KEY,
    title          TEXT NOT NULL,
    summary        TEXT NOT NULL,
    area           TEXT NOT NULL,
    min_level      TEXT NOT NULL DEFAULT 'senior',
    talking_points TEXT NOT NULL DEFAULT '[]',   -- JSON array of strings
    created_at     TEXT NOT NULL
);

-- Free-response (open-ended) practice attempts graded by the local LLM.
CREATE TABLE IF NOT EXISTS free_responses (
    id         INTEGER PRIMARY KEY,
    topic_slug TEXT NOT NULL,
    prompt     TEXT NOT NULL,
    answer     TEXT NOT NULL,
    score      INTEGER NOT NULL,
    max_score  INTEGER NOT NULL,
    feedback   TEXT NOT NULL,
    created_at TEXT NOT NULL
);
