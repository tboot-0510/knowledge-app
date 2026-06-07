-- LeetCode-style coding practice attempts (graded by the local LLM).
CREATE TABLE IF NOT EXISTS coding_attempts (
    id            INTEGER PRIMARY KEY,
    category_slug TEXT NOT NULL,
    title         TEXT NOT NULL,
    language      TEXT NOT NULL,
    code          TEXT NOT NULL,
    verdict       TEXT NOT NULL,
    score         INTEGER NOT NULL,
    max_score     INTEGER NOT NULL,
    created_at    TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_coding_category ON coding_attempts(category_slug);
