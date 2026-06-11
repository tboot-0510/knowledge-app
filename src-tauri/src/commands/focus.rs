//! Focus (Pomodoro) mode: a timed study session that serves questions on demand
//! and adapts difficulty as the learner succeeds. Questions are persisted (so
//! follow-ups, progress, and spaced-repetition all apply) under a single
//! sentinel "focus" session.

use crate::commands::daily::{generate_mcqs, schedule_review};
use crate::state::AppState;
use knowledge_core::learning::{load_catalog, pick_topic, to_questions};
use knowledge_core::models::{AttemptResult, Difficulty, Question};
use tauri::State;

const FOCUS_DATE: &str = "focus";

/// Generate (and persist) one question for a focus session, at `difficulty`.
/// If `topic_slug` is omitted, a selected topic is chosen automatically.
#[tauri::command]
pub async fn generate_focus_question(
    state: State<'_, AppState>,
    topic_slug: Option<String>,
    difficulty: Difficulty,
) -> Result<Question, String> {
    let (settings, topic) = {
        let db = state.db.lock().unwrap();
        let settings = db.get_settings().map_err(|e| e.to_string())?;
        let mut catalog = load_catalog().map_err(|e| e.to_string())?;
        catalog.extend(db.list_custom_topics().map_err(|e| e.to_string())?);
        let prefs = db.topic_prefs().map_err(|e| e.to_string())?;

        let topic = match topic_slug {
            Some(slug) if !slug.is_empty() => catalog
                .iter()
                .find(|t| t.slug == slug)
                .cloned()
                .ok_or_else(|| format!("unknown topic: {slug}"))?,
            _ => {
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
        (settings, topic)
    };

    let (mcqs, source) = generate_mcqs(&settings, &topic, difficulty, 1).await;

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

/// Grade a focus answer: record it (counts toward progress) and schedule it for
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
    Ok(AttemptResult {
        question_id,
        chosen_index,
        correct_index: q.correct_index,
        is_correct,
        explanation: q.explanation,
        session_completed: false,
    })
}
