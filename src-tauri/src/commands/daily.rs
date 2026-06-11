//! Daily learning commands: fetch/generate today's session, grade answers,
//! and report streak/progress.

use crate::state::AppState;
use chrono::{Duration, NaiveDate};
use knowledge_core::db::Db;
use knowledge_core::learning::{
    bloom_for, build_mcq_prompt, generic_fallback, level_prior, load_catalog,
    parse_and_validate_mcqs, pick_topic, quality_from_correct, seed_questions, sm2,
    target_difficulty, tier_to_rating, to_questions, update_ratings, update_streak, BloomLevel,
    CatalogTopic, GeneratedMcq, K_FACTOR,
};
use knowledge_core::models::{
    AttemptResult, DailySession, Difficulty, Level, ProgressStats, Settings, Streak, TopicRating,
};
use knowledge_core::ollama::OllamaClient;
use knowledge_core::scheduler;
use tauri::State;

/// Current Elo skill + win-streak for a topic, cold-starting from the seniority
/// prior and the user's chosen starting difficulty tier.
pub(crate) fn topic_skill_streak(
    db: &Db,
    slug: &str,
    level: Level,
    cold_start_tier: Difficulty,
) -> (f64, u32) {
    match db.get_rating(slug).ok().flatten() {
        Some(r) => (r.skill, r.streak),
        None => (level_prior(level).max(tier_to_rating(cold_start_tier) - 100.0), 0),
    }
}

/// Update the Elo ratings (the topic's and the global rating) after an answer.
pub(crate) fn update_topic_rating(db: &Db, topic_id: i64, is_correct: bool) -> Result<(), String> {
    let level = db.get_settings().map(|s| s.level).unwrap_or(Level::Senior);
    let slug = db.get_topic(topic_id).map_err(|e| e.to_string())?.slug;
    for key in [slug.as_str(), Db::GLOBAL_RATING] {
        let prev = db.get_rating(key).map_err(|e| e.to_string())?.unwrap_or(TopicRating {
            slug: key.to_string(),
            skill: level_prior(level),
            difficulty: level_prior(level),
            attempts: 0,
            streak: 0,
        });
        let (skill, difficulty) = update_ratings(prev.skill, prev.difficulty, is_correct, K_FACTOR);
        db.upsert_rating(&TopicRating {
            slug: key.to_string(),
            skill,
            difficulty,
            attempts: prev.attempts + 1,
            streak: if is_correct { prev.streak + 1 } else { 0 },
        })
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Add `days` to a "YYYY-MM-DD" date string.
pub(crate) fn add_days(date: &str, days: u32) -> String {
    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map(|d| (d + Duration::days(days as i64)).format("%Y-%m-%d").to_string())
        .unwrap_or_else(|_| date.to_string())
}

/// Schedule (or reschedule) a question for spaced-repetition review.
pub(crate) fn schedule_review(db: &Db, question_id: i64, is_correct: bool) -> Result<(), String> {
    let prev = db
        .get_review_state(question_id)
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    let next = sm2(prev, quality_from_correct(is_correct));
    let due = add_days(&scheduler::today_local(), next.interval_days);
    db.upsert_review(question_id, &next, &due)
        .map_err(|e| e.to_string())
}

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
    let (settings, topic, difficulty, bloom) = {
        let db = state.db.lock().unwrap();
        let settings = db.get_settings().map_err(|e| e.to_string())?;
        let recent = db.recent_topic_slugs(5).map_err(|e| e.to_string())?;
        let prefs = db.topic_prefs().map_err(|e| e.to_string())?;
        let mut catalog = load_catalog().map_err(|e| e.to_string())?;
        // Include user-synthesized custom topics in the daily pool.
        catalog.extend(db.list_custom_topics().map_err(|e| e.to_string())?);

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

        // Elo-driven difficulty + Bloom depth. The user's per-topic tier seeds
        // the cold start; the rating then self-calibrates after each answer.
        let cold_start = prefs
            .get(&topic.slug)
            .map(|p| p.target_difficulty)
            .unwrap_or(Difficulty::Medium);
        let (skill, streak) = topic_skill_streak(&db, &topic.slug, settings.level, cold_start);
        let difficulty = target_difficulty(skill, streak);
        let bloom = bloom_for(settings.level, streak);
        (settings, topic, difficulty, bloom)
    };

    // Generate via local LLM, falling back to the bundled seed bank.
    let (mcqs, source) = generate_mcqs(&settings, &topic, difficulty, 4, bloom).await;

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

/// Generate up to `n` MCQs via the local model, falling back to the seed bank,
/// then a generic question. Shared by the daily challenge and Focus mode.
pub(crate) async fn generate_mcqs(
    settings: &Settings,
    topic: &CatalogTopic,
    difficulty: Difficulty,
    n: usize,
    bloom: BloomLevel,
) -> (Vec<GeneratedMcq>, &'static str) {
    let n = n.max(1);
    let client = OllamaClient::new(&settings.ollama_url);
    let prompt = build_mcq_prompt(topic, settings.level, n, difficulty, bloom);
    if let Ok(raw) = client.generate(&settings.mcq_model, &prompt, true).await {
        if let Ok(mut mcqs) = parse_and_validate_mcqs(&raw) {
            mcqs.truncate(n);
            if !mcqs.is_empty() {
                return (mcqs, "generated");
            }
        }
    }
    let mut seed = seed_questions(&topic.slug);
    if !seed.is_empty() {
        seed.truncate(n);
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

    // Schedule for spaced repetition and update the Elo ratings.
    schedule_review(&db, question_id, is_correct)?;
    update_topic_rating(&db, q.topic_id, is_correct)?;

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
