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
    include_str!("../migrations/0003_topic_prefs.sql"),
    include_str!("../migrations/0004_practice.sql"),
    include_str!("../migrations/0005_coding.sql"),
    include_str!("../migrations/0006_ratings.sql"),
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
            mcq_model: self.get_setting("mcq_model")?.unwrap_or(d.mcq_model),
            embed_model: self.get_setting("embed_model")?.unwrap_or(d.embed_model),
            schedule_hour: self
                .get_setting("schedule_hour")?
                .and_then(|s| s.parse().ok())
                .unwrap_or(d.schedule_hour),
            ollama_url: self.get_setting("ollama_url")?.unwrap_or(d.ollama_url),
            reminders_enabled: self
                .get_setting("reminders_enabled")?
                .map(|s| s != "false")
                .unwrap_or(d.reminders_enabled),
            global_shortcut: self
                .get_setting("global_shortcut")?
                .unwrap_or(d.global_shortcut),
            onboarded: self
                .get_setting("onboarded")?
                .map(|s| s == "true")
                .unwrap_or(d.onboarded),
            cloud_enabled: self
                .get_setting("cloud_enabled")?
                .map(|s| s == "true")
                .unwrap_or(d.cloud_enabled),
            cloud_api_key: self.get_setting("cloud_api_key")?.unwrap_or(d.cloud_api_key),
            cloud_url: self.get_setting("cloud_url")?.unwrap_or(d.cloud_url),
        })
    }

    pub fn save_settings(&self, s: &Settings) -> Result<()> {
        self.set_setting("level", s.level.as_str())?;
        self.set_setting("chat_model", &s.chat_model)?;
        self.set_setting("mcq_model", &s.mcq_model)?;
        self.set_setting("embed_model", &s.embed_model)?;
        self.set_setting("schedule_hour", &s.schedule_hour.to_string())?;
        self.set_setting("ollama_url", &s.ollama_url)?;
        self.set_setting("reminders_enabled", if s.reminders_enabled { "true" } else { "false" })?;
        self.set_setting("global_shortcut", &s.global_shortcut)?;
        self.set_setting("onboarded", if s.onboarded { "true" } else { "false" })?;
        self.set_setting("cloud_enabled", if s.cloud_enabled { "true" } else { "false" })?;
        self.set_setting("cloud_api_key", &s.cloud_api_key)?;
        self.set_setting("cloud_url", &s.cloud_url)?;
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

    // ---- topic preferences ----------------------------------------------

    /// All saved topic preferences, keyed by slug.
    pub fn topic_prefs(&self) -> Result<std::collections::HashMap<String, TopicPref>> {
        let mut stmt = self
            .conn
            .prepare("SELECT slug, enabled, target_difficulty FROM topic_prefs")?;
        let rows = stmt.query_map([], |r| {
            let slug: String = r.get(0)?;
            Ok((
                slug.clone(),
                TopicPref {
                    slug,
                    enabled: r.get::<_, i64>(1)? != 0,
                    target_difficulty: Difficulty::from_str_lenient(&r.get::<_, String>(2)?),
                },
            ))
        })?;
        let mut map = std::collections::HashMap::new();
        for row in rows {
            let (k, v) = row?;
            map.insert(k, v);
        }
        Ok(map)
    }

    /// Upsert a single topic preference.
    pub fn set_topic_pref(&self, pref: &TopicPref) -> Result<()> {
        self.conn.execute(
            "INSERT INTO topic_prefs(slug, enabled, target_difficulty) VALUES(?1,?2,?3)
             ON CONFLICT(slug) DO UPDATE SET
                enabled = excluded.enabled,
                target_difficulty = excluded.target_difficulty",
            params![
                pref.slug,
                pref.enabled as i64,
                pref.target_difficulty.as_str()
            ],
        )?;
        Ok(())
    }

    /// Per-topic progress, keyed by slug: (answered, correct, per-tier stats).
    #[allow(clippy::type_complexity)]
    pub fn progress_by_topic(
        &self,
    ) -> Result<std::collections::HashMap<String, (u32, u32, Vec<DifficultyStat>)>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.slug, q.difficulty, COALESCE(SUM(a.is_correct),0), COUNT(*)
             FROM attempts a
             JOIN questions q ON q.id = a.question_id
             JOIN topics t ON t.id = q.topic_id
             GROUP BY t.slug, q.difficulty",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)? as u8,
                r.get::<_, i64>(2)? as u32,
                r.get::<_, i64>(3)? as u32,
            ))
        })?;

        // Accumulate into per-slug totals and per-tier buckets.
        let mut acc: std::collections::HashMap<String, (u32, u32, std::collections::HashMap<&'static str, (u32, u32)>)> =
            std::collections::HashMap::new();
        for row in rows {
            let (slug, diff_num, correct, total) = row?;
            let tier = Difficulty::from_numeric(diff_num);
            let entry = acc.entry(slug).or_default();
            entry.0 += total;
            entry.1 += correct;
            let bucket = entry.2.entry(tier.as_str()).or_default();
            bucket.0 += correct;
            bucket.1 += total;
        }

        let mut out = std::collections::HashMap::new();
        for (slug, (answered, correct, tiers)) in acc {
            let by_difficulty = [
                Difficulty::Easy,
                Difficulty::Medium,
                Difficulty::Hard,
                Difficulty::Advanced,
            ]
            .into_iter()
            .filter_map(|d| {
                tiers.get(d.as_str()).map(|(c, t)| DifficultyStat {
                    difficulty: d,
                    correct: *c,
                    total: *t,
                })
            })
            .collect();
            out.insert(slug, (answered, correct, by_difficulty));
        }
        Ok(out)
    }

    // ---- elo ratings ----------------------------------------------------

    /// Slug used for the learner's overall (cross-topic) skill rating.
    pub const GLOBAL_RATING: &'static str = "_global";

    pub fn get_rating(&self, slug: &str) -> Result<Option<TopicRating>> {
        let row = self
            .conn
            .query_row(
                "SELECT slug, skill, difficulty, attempts, streak FROM topic_ratings WHERE slug = ?1",
                params![slug],
                |r| {
                    Ok(TopicRating {
                        slug: r.get(0)?,
                        skill: r.get(1)?,
                        difficulty: r.get(2)?,
                        attempts: r.get::<_, i64>(3)? as u32,
                        streak: r.get::<_, i64>(4)? as u32,
                    })
                },
            )
            .optional()?;
        Ok(row)
    }

    pub fn upsert_rating(&self, r: &TopicRating) -> Result<()> {
        self.conn.execute(
            "INSERT INTO topic_ratings(slug, skill, difficulty, attempts, streak, updated_at)
             VALUES(?1,?2,?3,?4,?5,?6)
             ON CONFLICT(slug) DO UPDATE SET
                skill = excluded.skill, difficulty = excluded.difficulty,
                attempts = excluded.attempts, streak = excluded.streak,
                updated_at = excluded.updated_at",
            params![r.slug, r.skill, r.difficulty, r.attempts as i64, r.streak as i64, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    /// All per-topic ratings (excludes the `_global` row), highest skill first.
    pub fn list_ratings(&self) -> Result<Vec<TopicRating>> {
        let mut stmt = self.conn.prepare(
            "SELECT slug, skill, difficulty, attempts, streak FROM topic_ratings
             WHERE slug != ?1 ORDER BY skill DESC",
        )?;
        let rows = stmt.query_map(params![Self::GLOBAL_RATING], |r| {
            Ok(TopicRating {
                slug: r.get(0)?,
                skill: r.get(1)?,
                difficulty: r.get(2)?,
                attempts: r.get::<_, i64>(3)? as u32,
                streak: r.get::<_, i64>(4)? as u32,
            })
        })?;
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

    /// Look up a session id by its date key (lightweight; doesn't hydrate
    /// questions). Used for the persistent "focus" session.
    pub fn session_id_by_date(&self, date: &str) -> Result<Option<i64>> {
        let id = self
            .conn
            .query_row(
                "SELECT id FROM daily_sessions WHERE date = ?1",
                params![date],
                |r| r.get(0),
            )
            .optional()?;
        Ok(id)
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

    // ---- spaced repetition ----------------------------------------------

    pub fn get_review_state(&self, question_id: i64) -> Result<Option<ReviewState>> {
        let row = self
            .conn
            .query_row(
                "SELECT ease_factor, interval_days, repetitions FROM reviews WHERE question_id = ?1",
                params![question_id],
                |r| {
                    Ok(ReviewState {
                        ease_factor: r.get::<_, f64>(0)? as f32,
                        interval_days: r.get::<_, i64>(1)? as u32,
                        repetitions: r.get::<_, i64>(2)? as u32,
                    })
                },
            )
            .optional()?;
        Ok(row)
    }

    /// Upsert a question's review schedule.
    pub fn upsert_review(&self, question_id: i64, state: &ReviewState, due_date: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO reviews(question_id, ease_factor, interval_days, repetitions, due_date, last_reviewed)
             VALUES(?1,?2,?3,?4,?5,?6)
             ON CONFLICT(question_id) DO UPDATE SET
                ease_factor = excluded.ease_factor,
                interval_days = excluded.interval_days,
                repetitions = excluded.repetitions,
                due_date = excluded.due_date,
                last_reviewed = excluded.last_reviewed",
            params![
                question_id,
                state.ease_factor as f64,
                state.interval_days as i64,
                state.repetitions as i64,
                due_date,
                Utc::now().to_rfc3339()
            ],
        )?;
        Ok(())
    }

    /// Questions due for review on or before `today`, newest schedule first.
    pub fn due_reviews(&self, today: &str, limit: i64) -> Result<Vec<ReviewItem>> {
        let mut stmt = self.conn.prepare(
            "SELECT q.id, q.topic_id, q.prompt, q.choices_json, q.correct_index, q.explanation, q.difficulty,
                    t.title, r.due_date
             FROM reviews r
             JOIN questions q ON q.id = r.question_id
             JOIN topics t ON t.id = q.topic_id
             WHERE r.due_date <= ?1
             ORDER BY r.due_date ASC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![today, limit], |r| {
            let choices: String = r.get(3)?;
            Ok(ReviewItem {
                question: Question {
                    id: r.get(0)?,
                    topic_id: r.get(1)?,
                    prompt: r.get(2)?,
                    choices: serde_json::from_str(&choices).unwrap_or_default(),
                    correct_index: r.get::<_, i64>(4)? as usize,
                    explanation: r.get(5)?,
                    difficulty: r.get::<_, i64>(6)? as u8,
                },
                topic_title: r.get(7)?,
                due_date: r.get(8)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn count_due_reviews(&self, today: &str) -> Result<u32> {
        let n: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM reviews WHERE due_date <= ?1",
            params![today],
            |r| r.get(0),
        )?;
        Ok(n as u32)
    }

    // ---- custom topics --------------------------------------------------

    pub fn insert_custom_topic(&self, t: &crate::learning::catalog::CatalogTopic) -> Result<()> {
        let tp = serde_json::to_string(&t.talking_points)?;
        self.conn.execute(
            "INSERT INTO custom_topics(slug, title, summary, area, min_level, talking_points, created_at)
             VALUES(?1,?2,?3,?4,?5,?6,?7)
             ON CONFLICT(slug) DO UPDATE SET
                title = excluded.title, summary = excluded.summary, area = excluded.area,
                min_level = excluded.min_level, talking_points = excluded.talking_points",
            params![t.slug, t.title, t.summary, t.area, t.min_level.as_str(), tp, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn list_custom_topics(&self) -> Result<Vec<crate::learning::catalog::CatalogTopic>> {
        let mut stmt = self.conn.prepare(
            "SELECT slug, title, summary, area, min_level, talking_points FROM custom_topics ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |r| {
            let tp: String = r.get(5)?;
            Ok(crate::learning::catalog::CatalogTopic {
                slug: r.get(0)?,
                title: r.get(1)?,
                summary: r.get(2)?,
                area: r.get(3)?,
                min_level: Level::from_str_lenient(&r.get::<_, String>(4)?),
                talking_points: serde_json::from_str(&tp).unwrap_or_default(),
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn delete_custom_topic(&self, slug: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM custom_topics WHERE slug = ?1", params![slug])?;
        Ok(())
    }

    // ---- free-response history ------------------------------------------

    #[allow(clippy::too_many_arguments)]
    pub fn insert_free_response(
        &self,
        topic_slug: &str,
        prompt: &str,
        answer: &str,
        score: u32,
        max_score: u32,
        feedback: &str,
    ) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO free_responses(topic_slug, prompt, answer, score, max_score, feedback, created_at)
             VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![topic_slug, prompt, answer, score as i64, max_score as i64, feedback, Utc::now().to_rfc3339()],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    // ---- coding practice ------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    pub fn insert_coding_attempt(
        &self,
        category_slug: &str,
        title: &str,
        language: &str,
        code: &str,
        verdict: &str,
        score: u32,
        max_score: u32,
    ) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO coding_attempts(category_slug, title, language, code, verdict, score, max_score, created_at)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
            params![category_slug, title, language, code, verdict, score as i64, max_score as i64, Utc::now().to_rfc3339()],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Per-category coding progress: slug -> (attempted, solved).
    pub fn coding_progress(&self) -> Result<std::collections::HashMap<String, (u32, u32)>> {
        let mut stmt = self.conn.prepare(
            "SELECT category_slug, COUNT(*),
                    SUM(CASE WHEN verdict = 'correct' OR (max_score > 0 AND score * 100 >= max_score * 80) THEN 1 ELSE 0 END)
             FROM coding_attempts GROUP BY category_slug",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                (r.get::<_, i64>(1)? as u32, r.get::<_, i64>(2)? as u32),
            ))
        })?;
        let mut map = std::collections::HashMap::new();
        for row in rows {
            let (k, v) = row?;
            map.insert(k, v);
        }
        Ok(map)
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
    fn review_schedule_and_due_queue() {
        let db = Db::open_in_memory().unwrap();
        let tid = db
            .insert_topic("t", "Topic", "...", "area", Level::Senior, "catalog", &[])
            .unwrap();
        let sid = db.create_session("2026-06-06", tid, Level::Senior).unwrap();
        let q = Question {
            id: 0,
            topic_id: tid,
            prompt: "Q?".into(),
            choices: vec!["a".into(), "b".into(), "c".into(), "d".into()],
            correct_index: 0,
            explanation: "e".into(),
            difficulty: 2,
        };
        let qid = db.insert_question(sid, tid, &q).unwrap();
        assert!(db.get_review_state(qid).unwrap().is_none());
        db.upsert_review(qid, &ReviewState::default(), "2026-06-07").unwrap();
        // not due yet on the 6th, due on the 7th
        assert_eq!(db.count_due_reviews("2026-06-06").unwrap(), 0);
        assert_eq!(db.count_due_reviews("2026-06-07").unwrap(), 1);
        let due = db.due_reviews("2026-06-08", 10).unwrap();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].topic_title, "Topic");
    }

    #[test]
    fn coding_progress_counts_solved() {
        let db = Db::open_in_memory().unwrap();
        db.insert_coding_attempt("graphs", "Islands", "python", "code", "correct", 9, 10)
            .unwrap();
        db.insert_coding_attempt("graphs", "Islands", "python", "code", "incorrect", 3, 10)
            .unwrap();
        // 50% score, below the 80% solved threshold
        db.insert_coding_attempt("arrays-hashing", "Two Sum", "python", "c", "partial", 5, 10)
            .unwrap();
        let prog = db.coding_progress().unwrap();
        assert_eq!(prog["graphs"], (2, 1)); // two attempts, one solved
        assert_eq!(prog["arrays-hashing"], (1, 0));
    }

    #[test]
    fn custom_topic_round_trip() {
        let db = Db::open_in_memory().unwrap();
        let topic = crate::learning::catalog::CatalogTopic {
            slug: "webassembly".into(),
            title: "WebAssembly".into(),
            summary: "Portable bytecode".into(),
            area: "systems".into(),
            min_level: Level::Staff,
            talking_points: vec!["wasi".into()],
        };
        db.insert_custom_topic(&topic).unwrap();
        let list = db.list_custom_topics().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].slug, "webassembly");
        assert_eq!(list[0].min_level, Level::Staff);
        db.delete_custom_topic("webassembly").unwrap();
        assert!(db.list_custom_topics().unwrap().is_empty());
    }

    #[test]
    fn ratings_round_trip_and_exclude_global() {
        let db = Db::open_in_memory().unwrap();
        assert!(db.get_rating("consensus").unwrap().is_none());
        db.upsert_rating(&TopicRating {
            slug: "consensus".into(),
            skill: 1420.0,
            difficulty: 1380.0,
            attempts: 3,
            streak: 2,
        })
        .unwrap();
        db.upsert_rating(&TopicRating {
            slug: Db::GLOBAL_RATING.into(),
            skill: 1400.0,
            difficulty: 1400.0,
            attempts: 3,
            streak: 2,
        })
        .unwrap();
        let r = db.get_rating("consensus").unwrap().unwrap();
        assert_eq!(r.streak, 2);
        assert!((r.skill - 1420.0).abs() < 1e-9);
        // list excludes the _global row
        let list = db.list_ratings().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].slug, "consensus");
    }

    #[test]
    fn topic_prefs_round_trip() {
        let db = Db::open_in_memory().unwrap();
        assert!(db.topic_prefs().unwrap().is_empty());
        db.set_topic_pref(&TopicPref {
            slug: "evm-internals".into(),
            enabled: false,
            target_difficulty: Difficulty::Advanced,
        })
        .unwrap();
        let prefs = db.topic_prefs().unwrap();
        let p = prefs.get("evm-internals").unwrap();
        assert!(!p.enabled);
        assert_eq!(p.target_difficulty, Difficulty::Advanced);
        // upsert
        db.set_topic_pref(&TopicPref {
            slug: "evm-internals".into(),
            enabled: true,
            target_difficulty: Difficulty::Hard,
        })
        .unwrap();
        assert!(db.topic_prefs().unwrap()["evm-internals"].enabled);
    }

    #[test]
    fn progress_by_topic_buckets_by_tier() {
        let db = Db::open_in_memory().unwrap();
        let tid = db
            .insert_topic("transport-protocols", "TP", "...", "networking", Level::Senior, "catalog", &[])
            .unwrap();
        let sid = db.create_session("2026-06-06", tid, Level::Senior).unwrap();
        let mut q = Question {
            id: 0,
            topic_id: tid,
            prompt: "Q?".into(),
            choices: vec!["a".into(), "b".into(), "c".into(), "d".into()],
            correct_index: 0,
            explanation: "e".into(),
            difficulty: 3, // Hard tier
        };
        let q1 = db.insert_question(sid, tid, &q).unwrap();
        q.difficulty = 1; // Easy tier
        let q2 = db.insert_question(sid, tid, &q).unwrap();
        db.record_attempt(sid, q1, 0, true).unwrap();
        db.record_attempt(sid, q2, 1, false).unwrap();

        let prog = db.progress_by_topic().unwrap();
        let (answered, correct, tiers) = prog.get("transport-protocols").unwrap();
        assert_eq!(*answered, 2);
        assert_eq!(*correct, 1);
        // one Hard and one Easy bucket present
        assert!(tiers.iter().any(|t| t.difficulty == Difficulty::Hard && t.total == 1));
        assert!(tiers.iter().any(|t| t.difficulty == Difficulty::Easy && t.total == 1));
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
