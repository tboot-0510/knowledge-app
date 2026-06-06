//! SQLite storage layer (rusqlite, bundled).
//!
//! Wraps a [`Connection`] and exposes typed data-access methods. Migrations are
//! embedded at compile time and applied via `PRAGMA user_version`. An in-memory
//! constructor ([`Db::open_in_memory`]) keeps the whole layer unit-testable.

use crate::error::Result;
use crate::models::*;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

/// Ordered list of (target user_version, SQL). Index + 1 is the version.
const MIGRATIONS: &[&str] = &[
    include_str!("../migrations/0001_init.sql"),
    include_str!("../migrations/0002_repos.sql"),
];

/// Thin wrapper around a rusqlite connection.
pub struct Db {
    conn: Connection,
}

impl Db {
    /// Open (or create) the database at `path` and run pending migrations.
    pub fn open(path: &std::path::Path) -> Result<Db> {
        let conn = Connection::open(path)?;
        Self::init(conn)
    }

    /// Open an in-memory database (used by tests).
    pub fn open_in_memory() -> Result<Db> {
        let conn = Connection::open_in_memory()?;
        Self::init(conn)
    }

    fn init(conn: Connection) -> Result<Db> {
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        let mut db = Db { conn };
        db.migrate()?;
        Ok(db)
    }

    /// Apply any migrations whose version exceeds the current `user_version`.
    fn migrate(&mut self) -> Result<()> {
        let current: i64 =
            self.conn
                .query_row("PRAGMA user_version", [], |r| r.get(0))?;
        for (i, sql) in MIGRATIONS.iter().enumerate() {
            let version = (i + 1) as i64;
            if version > current {
                let tx = self.conn.transaction()?;
                tx.execute_batch(sql)?;
                tx.pragma_update(None, "user_version", version)?;
                tx.commit()?;
            }
        }
        Ok(())
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    // ---- settings -------------------------------------------------------

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let v = self
            .conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key],
                |r| r.get(0),
            )
            .optional()?;
        Ok(v)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO settings(key, value) VALUES(?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    /// Load settings, falling back to defaults for any missing key.
    pub fn get_settings(&self) -> Result<Settings> {
        let d = Settings::default();
        Ok(Settings {
            level: self
                .get_setting("level")?
                .map(|s| Level::from_str_lenient(&s))
                .unwrap_or(d.level),
            chat_model: self.get_setting("chat_model")?.unwrap_or(d.chat_model),
            embed_model: self.get_setting("embed_model")?.unwrap_or(d.embed_model),
            schedule_hour: self
                .get_setting("schedule_hour")?
                .and_then(|s| s.parse().ok())
                .unwrap_or(d.schedule_hour),
            ollama_url: self.get_setting("ollama_url")?.unwrap_or(d.ollama_url),
        })
    }

    pub fn save_settings(&self, s: &Settings) -> Result<()> {
        self.set_setting("level", s.level.as_str())?;
        self.set_setting("chat_model", &s.chat_model)?;
        self.set_setting("embed_model", &s.embed_model)?;
        self.set_setting("schedule_hour", &s.schedule_hour.to_string())?;
        self.set_setting("ollama_url", &s.ollama_url)?;
        Ok(())
    }

    // ---- topics ---------------------------------------------------------

    /// Insert a topic and return its id.
    #[allow(clippy::too_many_arguments)]
    pub fn insert_topic(
        &self,
        slug: &str,
        title: &str,
        summary: &str,
        area: &str,
        level: Level,
        source: &str,
        talking_points: &[String],
    ) -> Result<i64> {
        let tp = serde_json::to_string(talking_points)?;
        self.conn.execute(
            "INSERT INTO topics(slug, title, summary, area, level, source, talking_points, created_at)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
            params![slug, title, summary, area, level.as_str(), source, tp, Utc::now().to_rfc3339()],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_topic(&self, id: i64) -> Result<Topic> {
        self.conn
            .query_row(
                "SELECT id, slug, title, summary, area, level, source, talking_points
                 FROM topics WHERE id = ?1",
                params![id],
                row_to_topic,
            )
            .map_err(Into::into)
    }

    /// Slugs of topics used in the most recent `n` daily sessions (to avoid repeats).
    pub fn recent_topic_slugs(&self, n: i64) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.slug FROM daily_sessions d
             JOIN topics t ON t.id = d.topic_id
             ORDER BY d.date DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![n], |r| r.get::<_, String>(0))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    // ---- daily sessions -------------------------------------------------

    pub fn get_session_by_date(&self, date: &str) -> Result<Option<DailySession>> {
        let row = self
            .conn
            .query_row(
                "SELECT id, date, topic_id, level, completed, score
                 FROM daily_sessions WHERE date = ?1",
                params![date],
                |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, i64>(2)?,
                        r.get::<_, i64>(4)?,
                        r.get::<_, Option<i64>>(5)?,
                    ))
                },
            )
            .optional()?;
        let Some((id, date, topic_id, completed, score)) = row else {
            return Ok(None);
        };
        let topic = self.get_topic(topic_id)?;
        let questions = self.questions_for_session(id)?;
        Ok(Some(DailySession {
            id,
            date,
            topic,
            questions,
            completed: completed != 0,
            score: score.map(|s| s as u32),
        }))
    }

    /// Create a session row for `date`/`topic_id` and return its id.
    pub fn create_session(&self, date: &str, topic_id: i64, level: Level) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO daily_sessions(date, topic_id, level, completed)
             VALUES(?1,?2,?3,0)",
            params![date, topic_id, level.as_str()],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn insert_question(&self, session_id: i64, topic_id: i64, q: &Question) -> Result<i64> {
        let choices = serde_json::to_string(&q.choices)?;
        self.conn.execute(
            "INSERT INTO questions(session_id, topic_id, prompt, choices_json, correct_index, explanation, difficulty, created_at)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                session_id, topic_id, q.prompt, choices,
                q.correct_index as i64, q.explanation, q.difficulty as i64,
                Utc::now().to_rfc3339()
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn questions_for_session(&self, session_id: i64) -> Result<Vec<Question>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, topic_id, prompt, choices_json, correct_index, explanation, difficulty
             FROM questions WHERE session_id = ?1 ORDER BY id",
        )?;
        let rows = stmt.query_map(params![session_id], row_to_question)?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn get_question(&self, id: i64) -> Result<Question> {
        self.conn
            .query_row(
                "SELECT id, topic_id, prompt, choices_json, correct_index, explanation, difficulty
                 FROM questions WHERE id = ?1",
                params![id],
                row_to_question,
            )
            .map_err(Into::into)
    }

    /// Record an answer (idempotent per question via the unique index).
    pub fn record_attempt(
        &self,
        session_id: i64,
        question_id: i64,
        chosen_index: usize,
        is_correct: bool,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO attempts(question_id, session_id, chosen_index, is_correct, answered_at)
             VALUES(?1,?2,?3,?4,?5)
             ON CONFLICT(question_id) DO UPDATE SET
                chosen_index = excluded.chosen_index,
                is_correct   = excluded.is_correct,
                answered_at  = excluded.answered_at",
            params![
                question_id, session_id, chosen_index as i64,
                is_correct as i64, Utc::now().to_rfc3339()
            ],
        )?;
        Ok(())
    }

    /// (answered, total) question counts for a session.
    pub fn session_answer_counts(&self, session_id: i64) -> Result<(u32, u32)> {
        let total: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM questions WHERE session_id = ?1",
            params![session_id],
            |r| r.get(0),
        )?;
        let answered: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM attempts WHERE session_id = ?1",
            params![session_id],
            |r| r.get(0),
        )?;
        Ok((answered as u32, total as u32))
    }

    /// Mark a session complete and store its correct-answer count.
    pub fn complete_session(&self, session_id: i64) -> Result<u32> {
        let correct: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM attempts WHERE session_id = ?1 AND is_correct = 1",
            params![session_id],
            |r| r.get(0),
        )?;
        self.conn.execute(
            "UPDATE daily_sessions SET completed = 1, score = ?2 WHERE id = ?1",
            params![session_id, correct],
        )?;
        Ok(correct as u32)
    }

    // ---- streaks & progress --------------------------------------------

    pub fn get_streak(&self) -> Result<Streak> {
        self.conn
            .query_row(
                "SELECT current_streak, longest_streak, last_active_date FROM streaks WHERE id = 1",
                [],
                |r| {
                    Ok(Streak {
                        current_streak: r.get::<_, i64>(0)? as u32,
                        longest_streak: r.get::<_, i64>(1)? as u32,
                        last_active_date: r.get(2)?,
                    })
                },
            )
            .map_err(Into::into)
    }

    pub fn save_streak(&self, s: &Streak) -> Result<()> {
        self.conn.execute(
            "UPDATE streaks SET current_streak = ?1, longest_streak = ?2, last_active_date = ?3 WHERE id = 1",
            params![s.current_streak as i64, s.longest_streak as i64, s.last_active_date],
        )?;
        Ok(())
    }

    pub fn progress_stats(&self) -> Result<ProgressStats> {
        let (total, correct): (i64, i64) = self.conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(is_correct),0) FROM attempts",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let days: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM daily_sessions WHERE completed = 1",
            [],
            |r| r.get(0),
        )?;
        let mut stmt = self.conn.prepare(
            "SELECT t.area, COALESCE(SUM(a.is_correct),0), COUNT(*)
             FROM attempts a
             JOIN questions q ON q.id = a.question_id
             JOIN topics t ON t.id = q.topic_id
             GROUP BY t.area ORDER BY t.area",
        )?;
        let by_area = stmt
            .query_map([], |r| {
                Ok(AreaStat {
                    area: r.get(0)?,
                    correct: r.get::<_, i64>(1)? as u32,
                    total: r.get::<_, i64>(2)? as u32,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(ProgressStats {
            total_questions: total as u32,
            total_correct: correct as u32,
            days_completed: days as u32,
            streak: self.get_streak()?,
            by_area,
        })
    }

    // ---- repos ----------------------------------------------------------

    pub fn insert_repo(&self, name: &str, url: &str, local_path: &str) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO repos(name, url, local_path, status, created_at)
             VALUES(?1,?2,?3,'pending',?4)",
            params![name, url, local_path, Utc::now().to_rfc3339()],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn set_repo_status(&self, repo_id: i64, status: RepoStatus, error: Option<&str>) -> Result<()> {
        self.conn.execute(
            "UPDATE repos SET status = ?2, error = ?3 WHERE id = ?1",
            params![repo_id, status.as_str(), error],
        )?;
        Ok(())
    }

    pub fn finalize_repo_index(
        &self,
        repo_id: i64,
        file_count: u32,
        chunk_count: u32,
        head_commit: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE repos SET status='ready', file_count=?2, chunk_count=?3,
                head_commit=?4, indexed_at=?5, error=NULL WHERE id=?1",
            params![
                repo_id, file_count as i64, chunk_count as i64,
                head_commit, Utc::now().to_rfc3339()
            ],
        )?;
        Ok(())
    }

    pub fn get_repo(&self, id: i64) -> Result<Repo> {
        self.conn
            .query_row(
                "SELECT id, name, url, local_path, status, file_count, chunk_count, indexed_at, error
                 FROM repos WHERE id = ?1",
                params![id],
                row_to_repo,
            )
            .map_err(Into::into)
    }

    pub fn list_repos(&self) -> Result<Vec<Repo>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, url, local_path, status, file_count, chunk_count, indexed_at, error
             FROM repos ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], row_to_repo)?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn delete_repo(&self, id: i64) -> Result<()> {
        // Cascades to chunks + embeddings via ON DELETE CASCADE.
        self.conn
            .execute("DELETE FROM repos WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn clear_repo_chunks(&self, repo_id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM repo_chunks WHERE repo_id = ?1", params![repo_id])?;
        Ok(())
    }

    /// Insert a chunk and its embedding together; returns the chunk id.
    #[allow(clippy::too_many_arguments)]
    pub fn insert_chunk_with_embedding(
        &self,
        repo_id: i64,
        file_path: &str,
        start_line: u32,
        end_line: u32,
        content: &str,
        token_estimate: u32,
        embedding: &[f32],
    ) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO repo_chunks(repo_id, file_path, start_line, end_line, content, token_estimate)
             VALUES(?1,?2,?3,?4,?5,?6)",
            params![repo_id, file_path, start_line as i64, end_line as i64, content, token_estimate as i64],
        )?;
        let chunk_id = self.conn.last_insert_rowid();
        self.conn.execute(
            "INSERT INTO repo_embeddings(chunk_id, repo_id, dim, vector) VALUES(?1,?2,?3,?4)",
            params![chunk_id, repo_id, embedding.len() as i64, f32_slice_to_blob(embedding)],
        )?;
        Ok(chunk_id)
    }

    /// Load all (chunk_id, embedding) pairs for a repo for cosine search.
    pub fn embeddings_for_repo(&self, repo_id: i64) -> Result<Vec<(i64, Vec<f32>)>> {
        let mut stmt = self.conn.prepare(
            "SELECT chunk_id, vector FROM repo_embeddings WHERE repo_id = ?1",
        )?;
        let rows = stmt.query_map(params![repo_id], |r| {
            let id: i64 = r.get(0)?;
            let blob: Vec<u8> = r.get(1)?;
            Ok((id, blob_to_f32_vec(&blob)))
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn get_chunk(&self, chunk_id: i64) -> Result<RepoChunk> {
        self.conn
            .query_row(
                "SELECT id, repo_id, file_path, start_line, end_line, content
                 FROM repo_chunks WHERE id = ?1",
                params![chunk_id],
                |r| {
                    Ok(RepoChunk {
                        id: r.get(0)?,
                        repo_id: r.get(1)?,
                        file_path: r.get(2)?,
                        start_line: r.get::<_, i64>(3)? as u32,
                        end_line: r.get::<_, i64>(4)? as u32,
                        content: r.get(5)?,
                    })
                },
            )
            .map_err(Into::into)
    }
}

// ---- row mappers --------------------------------------------------------

fn row_to_topic(r: &rusqlite::Row) -> rusqlite::Result<Topic> {
    let tp: String = r.get(7)?;
    Ok(Topic {
        id: r.get(0)?,
        slug: r.get(1)?,
        title: r.get(2)?,
        summary: r.get(3)?,
        area: r.get(4)?,
        level: Level::from_str_lenient(&r.get::<_, String>(5)?),
        source: r.get(6)?,
        talking_points: serde_json::from_str(&tp).unwrap_or_default(),
    })
}

fn row_to_question(r: &rusqlite::Row) -> rusqlite::Result<Question> {
    let choices: String = r.get(3)?;
    Ok(Question {
        id: r.get(0)?,
        topic_id: r.get(1)?,
        prompt: r.get(2)?,
        choices: serde_json::from_str(&choices).unwrap_or_default(),
        correct_index: r.get::<_, i64>(4)? as usize,
        explanation: r.get(5)?,
        difficulty: r.get::<_, i64>(6)? as u8,
    })
}

fn row_to_repo(r: &rusqlite::Row) -> rusqlite::Result<Repo> {
    Ok(Repo {
        id: r.get(0)?,
        name: r.get(1)?,
        url: r.get(2)?,
        local_path: r.get(3)?,
        status: RepoStatus::from_str_lenient(&r.get::<_, String>(4)?),
        file_count: r.get::<_, i64>(5)? as u32,
        chunk_count: r.get::<_, i64>(6)? as u32,
        indexed_at: r.get(7)?,
        error: r.get(8)?,
    })
}

// ---- vector (de)serialization ------------------------------------------

/// Pack an f32 slice into a little-endian byte BLOB.
pub fn f32_slice_to_blob(v: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(v.len() * 4);
    for f in v {
        out.extend_from_slice(&f.to_le_bytes());
    }
    out
}

/// Unpack a little-endian byte BLOB into an f32 vector.
pub fn blob_to_f32_vec(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_apply_and_are_idempotent() {
        let db = Db::open_in_memory().unwrap();
        let v: i64 = db
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, MIGRATIONS.len() as i64);
    }

    #[test]
    fn settings_round_trip_with_defaults() {
        let db = Db::open_in_memory().unwrap();
        let loaded = db.get_settings().unwrap();
        assert_eq!(loaded.level, Level::Senior); // default
        let mut s = Settings::default();
        s.level = Level::Staff;
        s.schedule_hour = 7;
        db.save_settings(&s).unwrap();
        let again = db.get_settings().unwrap();
        assert_eq!(again.level, Level::Staff);
        assert_eq!(again.schedule_hour, 7);
    }

    #[test]
    fn vector_blob_round_trip() {
        let v = vec![0.0f32, 1.5, -2.25, 1e-3];
        let blob = f32_slice_to_blob(&v);
        assert_eq!(blob.len(), v.len() * 4);
        assert_eq!(blob_to_f32_vec(&blob), v);
    }

    #[test]
    fn session_and_attempt_flow() {
        let db = Db::open_in_memory().unwrap();
        let tid = db
            .insert_topic("consensus", "Consensus", "...", "distributed-systems", Level::Staff, "catalog", &[])
            .unwrap();
        let sid = db.create_session("2026-06-06", tid, Level::Staff).unwrap();
        let q = Question {
            id: 0,
            topic_id: tid,
            prompt: "Q?".into(),
            choices: vec!["a".into(), "b".into(), "c".into(), "d".into()],
            correct_index: 2,
            explanation: "because".into(),
            difficulty: 3,
        };
        let qid = db.insert_question(sid, tid, &q).unwrap();
        db.record_attempt(sid, qid, 2, true).unwrap();
        let (answered, total) = db.session_answer_counts(sid).unwrap();
        assert_eq!((answered, total), (1, 1));
        let correct = db.complete_session(sid).unwrap();
        assert_eq!(correct, 1);
        let loaded = db.get_session_by_date("2026-06-06").unwrap().unwrap();
        assert!(loaded.completed);
        assert_eq!(loaded.score, Some(1));
        assert_eq!(loaded.questions.len(), 1);
    }
}
