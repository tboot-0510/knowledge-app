//! Focus (Pomodoro) mode: a timed study session that serves questions on demand
//! and adapts difficulty via the Elo recommender. Questions are persisted (so
//! follow-ups, progress, and spaced repetition all apply) under a single
//! sentinel "focus" session.

use crate::commands::daily::{generate_mcqs, schedule_review, topic_skill_streak, update_topic_rating};
use crate::state::AppState;
use knowledge_core::db::Db;
use knowledge_core::learning::{bloom_for, load_catalog, pick_topic, target_difficulty, to_questions};
use knowledge_core::models::{AttemptResult, Difficulty, Question};
use tauri::State;

const FOCUS_DATE: &str = "focus";

/// Generate (and persist) one question for a focus session. Difficulty and Bloom
/// depth are chosen by the Elo recommender: for a fixed topic from that topic's
/// skill rating, for "mixed" from the learner's global rating. If `topic_slug`
/// is omitted, a topic is chosen automatically.
#[tauri::command]
pub async fn generate_focus_question(
    state: State<'_, AppState>,
    topic_slug: Option<String>,
) -> Result<Question, String> {
    let (settings, topic, difficulty, bloom) = {
        let db = state.db.lock().unwrap();
        let settings = db.get_settings().map_err(|e| e.to_string())?;
        let mut catalog = load_catalog().map_err(|e| e.to_string())?;
        catalog.extend(db.list_custom_topics().map_err(|e| e.to_string())?);
        let prefs = db.topic_prefs().map_err(|e| e.to_string())?;

        let fixed = topic_slug.as_deref().filter(|s| !s.is_empty());
        let topic = match fixed {
            Some(slug) => catalog
                .iter()
                .find(|t| t.slug == slug)
                .cloned()
                .ok_or_else(|| format!("unknown topic: {slug}"))?,
            None => {
                let recent = db.recent_topic_slugs(3).map_err(|e| e.to_string())?;
                let enabled: Vec<_> = catalog
                    .iter()
                    .filter(|t| prefs.get(&t.slug).map(|p| p.enabled).unwrap_or(true))
                    .cloned()
                    .collect();
                let pool = if enabled.is_empty() { catalog.clone() } else { enabled };
                pick_topic(&pool, settings.level, &recent)
                    .cloned()
                    .ok_or_else(|| "no topics available".to_string())?
            }
        };

        // Fixed topic → that topic's rating; mixed → the global rating so the
        // session's momentum (streak) drives the hardening across topics.
        let rating_slug = if fixed.is_some() {
            topic.slug.clone()
        } else {
            Db::GLOBAL_RATING.to_string()
        };
        let cold_start = prefs
            .get(&topic.slug)
            .map(|p| p.target_difficulty)
            .unwrap_or(Difficulty::Medium);
        let level = settings.level;
        let (skill, streak) = topic_skill_streak(&db, &rating_slug, level, cold_start);
        let difficulty = target_difficulty(skill, streak);
        let bloom = bloom_for(level, streak);
        (settings, topic, difficulty, bloom)
    };

    let (mcqs, source) = generate_mcqs(&settings, &topic, difficulty, 1, bloom).await;

    let question = {
        let db = state.db.lock().unwrap();
        let topic_id = db
            .insert_topic(
                &topic.slug,
                &topic.title,
                &topic.summary,
                &topic.area,
                settings.level,
                source,
                &topic.talking_points,
            )
            .map_err(|e| e.to_string())?;
        let session_id = match db.session_id_by_date(FOCUS_DATE).map_err(|e| e.to_string())? {
            Some(id) => id,
            None => db
                .create_session(FOCUS_DATE, topic_id, settings.level)
                .map_err(|e| e.to_string())?,
        };
        let mut questions = to_questions(topic_id, mcqs);
        let mut q = questions.remove(0);
        let qid = db
            .insert_question(session_id, topic_id, &q)
            .map_err(|e| e.to_string())?;
        q.id = qid;
        q
    };
    Ok(question)
}

/// Grade a focus answer: record it, update Elo ratings, and schedule it for
/// spaced repetition. Does not affect the daily streak.
#[tauri::command]
pub fn submit_focus_answer(
    state: State<AppState>,
    question_id: i64,
    chosen_index: usize,
) -> Result<AttemptResult, String> {
    let db = state.db.lock().unwrap();
    let q = db.get_question(question_id).map_err(|e| e.to_string())?;
    let is_correct = chosen_index == q.correct_index;
    let session_id = db
        .session_id_by_date(FOCUS_DATE)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no active focus session".to_string())?;
    db.record_attempt(session_id, question_id, chosen_index, is_correct)
        .map_err(|e| e.to_string())?;
    schedule_review(&db, question_id, is_correct)?;
    update_topic_rating(&db, q.topic_id, is_correct)?;
    Ok(AttemptResult {
        question_id,
        chosen_index,
        correct_index: q.correct_index,
        is_correct,
        explanation: q.explanation,
        session_completed: false,
    })
}
