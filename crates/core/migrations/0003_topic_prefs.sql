-- User topic selection + per-topic target difficulty for the Topics panel.
-- A topic with no row here is treated as enabled at 'medium' by default.

CREATE TABLE IF NOT EXISTS topic_prefs (
    slug              TEXT PRIMARY KEY,
    enabled           INTEGER NOT NULL DEFAULT 1,
    target_difficulty TEXT NOT NULL DEFAULT 'medium'
);
