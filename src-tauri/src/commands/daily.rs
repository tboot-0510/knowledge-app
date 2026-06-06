//! Daily learning commands: fetch/generate today's session, grade answers,
//! and report streak/progress.

use crate::state::AppState;
use knowledge_core::learning::{
    build_mcq_prompt, generic_fallback, load_catalog, next_difficulty, parse_and_validate_mcqs,
    pick_topic, seed_questions, to_questions, update_streak, CatalogTopic, GeneratedMcq,
};
use knowledge_core::models::{
    AttemptResult, DailySession, Difficulty, ProgressStats, Settings, Streak,
};
use knowledge_core::ollama::OllamaClient;
use knowledge_core::scheduler;
use tauri::State;

/// Get today's session, generating it (topic + MCQs) on first access of the day.
#[tauri::command]
pub async fn get_today_session(state: State<'_, AppState>) -> Result<DailySession, String> {
    let today = scheduler::today_local();

    // Return an already-created session unchanged.
    {
        let db = state.db.lock().unwrap();
        if let Some(s) = db.get_session_by_date(&today).map_err(|e| e.to_string())? {
            return Ok(s);
        }
    }

    // Gather generation inputs under a scoped lock (dropped before any await).
    let (settings, topic, target_difficulty) = {
        let db = state.db.lock().unwrap();
        let settings = db.get_settings().map_err(|e| e.to_string())?;
        let recent = db.recent_topic_slugs(5).map_err(|e| e.to_string())?;
        let prefs = db.topic_prefs().map_err(|e| e.to_string())?;
        let catalog = load_catalog().map_err(|e| e.to_string())?;

        // Restrict to topics the user has opted into (a topic with no pref, or
        // an enabled pref, counts as selected). If none are selected, use all.
        let enabled: Vec<CatalogTopic> = catalog
            .iter()
            .filter(|t| prefs.get(&t.slug).map(|p| p.enabled).unwrap_or(true))
            .cloned()
            .collect();
        let pool = if enabled.is_empty() { &catalog } else { &enabled };

        let topic = pick_topic(pool, settings.level, &recent)
            .cloned()
            .ok_or_else(|| "topic catalog is empty".to_string())?;

        // Difficulty: honour the user's per-topic target if set, else adapt to
        // recent accuracy and seniority.
        let difficulty = match prefs.get(&topic.slug) {
            Some(p) => p.target_difficulty,
            None => {
                let stats = db.progress_stats().map_err(|e| e.to_string())?;
                let accuracy = if stats.total_questions > 0 {
                    stats.total_correct as f32 / stats.total_questions as f32
                } else {
                    0.5
                };
                Difficulty::from_numeric(next_difficulty(settings.level, accuracy))
            }
        };
        (settings, topic, difficulty)
    };

    // Generate via local LLM, falling back to the bundled seed bank.
    let (mcqs, source) = generate_or_fallback(&settings, &topic, target_difficulty).await;

    // Persist topic, session, and questions, then return the hydrated session.
    let session = {
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
        let session_id = db
            .create_session(&today, topic_id, settings.level)
            .map_err(|e| e.to_string())?;
        for q in to_questions(topic_id, mcqs) {
            db.insert_question(session_id, topic_id, &q)
                .map_err(|e| e.to_string())?;
        }
        db.get_session_by_date(&today)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "session disappeared after creation".to_string())?
    };
    Ok(session)
}

/// Try the local model; on any failure use the seed bank, then a generic question.
async fn generate_or_fallback(
    settings: &Settings,
    topic: &CatalogTopic,
    difficulty: Difficulty,
) -> (Vec<GeneratedMcq>, &'static str) {
    let client = OllamaClient::new(&settings.ollama_url);
    let prompt = build_mcq_prompt(topic, settings.level, 4, difficulty);
    if let Ok(raw) = client.generate(&settings.chat_model, &prompt, true).await {
        if let Ok(mcqs) = parse_and_validate_mcqs(&raw) {
            return (mcqs, "generated");
        }
    }
    let seed = seed_questions(&topic.slug);
    if !seed.is_empty() {
        return (seed, "catalog");
    }
    (vec![generic_fallback(&topic.title)], "catalog")
}

/// Grade a submitted answer; completes the session + updates the streak when the
/// last question is answered.
#[tauri::command]
pub fn submit_answer(
    state: State<AppState>,
    session_id: i64,
    question_id: i64,
    chosen_index: usize,
) -> Result<AttemptResult, String> {
    let db = state.db.lock().unwrap();
    let q = db.get_question(question_id).map_err(|e| e.to_string())?;
    let is_correct = chosen_index == q.correct_index;
    db.record_attempt(session_id, question_id, chosen_index, is_correct)
        .map_err(|e| e.to_string())?;

    let (answered, total) = db
        .session_answer_counts(session_id)
        .map_err(|e| e.to_string())?;
    let session_completed = total > 0 && answered >= total;
    if session_completed {
        db.complete_session(session_id).map_err(|e| e.to_string())?;
        let streak = db.get_streak().map_err(|e| e.to_string())?;
        let updated = update_streak(streak, &scheduler::today_local());
        db.save_streak(&updated).map_err(|e| e.to_string())?;
    }

    Ok(AttemptResult {
        question_id,
        chosen_index,
        correct_index: q.correct_index,
        is_correct,
        explanation: q.explanation,
        session_completed,
    })
}

#[tauri::command]
pub fn get_streak(state: State<AppState>) -> Result<Streak, String> {
    state.db.lock().unwrap().get_streak().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_progress(state: State<AppState>) -> Result<ProgressStats, String> {
    state
        .db
        .lock()
        .unwrap()
        .progress_stats()
        .map_err(|e| e.to_string())
}

/// Whether the daily popup should be shown now (once per local day).
#[tauri::command]
pub fn should_show_today(state: State<AppState>) -> Result<bool, String> {
    let db = state.db.lock().unwrap();
    let last = db.get_setting("last_shown_date").map_err(|e| e.to_string())?;
    Ok(scheduler::should_show(last.as_deref(), &scheduler::today_local()))
}

/// Record that the popup was shown today (updates the once-per-day gate).
#[tauri::command]
pub fn mark_shown_today(state: State<AppState>) -> Result<(), String> {
    state
        .db
        .lock()
        .unwrap()
        .set_setting("last_shown_date", &scheduler::today_local())
        .map_err(|e| e.to_string())
}
