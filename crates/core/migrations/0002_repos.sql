-- Linked GitHub repositories, their chunks, and local embedding vectors.

CREATE TABLE IF NOT EXISTS repos (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    url         TEXT NOT NULL,
    local_path  TEXT NOT NULL,
    status      TEXT NOT NULL,          -- pending|cloning|indexing|ready|error
    file_count  INTEGER NOT NULL DEFAULT 0,
    chunk_count INTEGER NOT NULL DEFAULT 0,
    head_commit TEXT,
    indexed_at  TEXT,
    error       TEXT,
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS repo_chunks (
    id         INTEGER PRIMARY KEY,
    repo_id    INTEGER NOT NULL REFERENCES repos(id) ON DELETE CASCADE,
    file_path  TEXT NOT NULL,
    start_line INTEGER NOT NULL,
    end_line   INTEGER NOT NULL,
    content    TEXT NOT NULL,
    token_estimate INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_repo_chunks_repo ON repo_chunks(repo_id);

-- Pure-Rust cosine fallback store: f32 vectors as little-endian BLOBs.
-- (sqlite-vec vec0 virtual table is an optional future optimization; the
--  in-Rust cosine path keeps retrieval testable without native extensions.)
CREATE TABLE IF NOT EXISTS repo_embeddings (
    chunk_id INTEGER PRIMARY KEY REFERENCES repo_chunks(id) ON DELETE CASCADE,
    repo_id  INTEGER NOT NULL REFERENCES repos(id) ON DELETE CASCADE,
    dim      INTEGER NOT NULL,
    vector   BLOB NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_repo_embeddings_repo ON repo_embeddings(repo_id);
