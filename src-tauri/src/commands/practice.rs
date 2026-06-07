//! Free-response (open-ended) practice commands: generate an open question and
//! grade the user's written answer with the local LLM.

use crate::state::AppState;
use knowledge_core::learning::{
    build_freeresponse_prompt, build_grading_prompt, load_catalog, parse_freeresponse_question,
    parse_grade, pick_topic,
};
use knowledge_core::models::{FreeResponseGrade, FreeResponseQuestion};
use knowledge_core::ollama::OllamaClient;
use tauri::State;

/// Generate one open-ended interview question for a topic (or a selected topic
/// if `topic_slug` is omitted).
#[tauri::command]
pub async fn generate_free_response(
    state: State<'_, AppState>,
    topic_slug: Option<String>,
) -> Result<FreeResponseQuestion, String> {
    let (settings, topic, difficulty) = {
        let db = state.db.lock().unwrap();
        let settings = db.get_settings().map_err(|e| e.to_string())?;
        let mut catalog = load_catalog().map_err(|e| e.to_string())?;
        catalog.extend(db.list_custom_topics().map_err(|e| e.to_string())?);
        let prefs = db.topic_prefs().map_err(|e| e.to_string())?;

        let topic = match topic_slug {
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
        let difficulty = prefs
            .get(&topic.slug)
            .map(|p| p.target_difficulty)
            .unwrap_or_default();
        (settings, topic, difficulty)
    };

    let client = OllamaClient::new(&settings.ollama_url);
    let prompt = build_freeresponse_prompt(&topic, settings.level, difficulty);
    let raw = client
        .generate(&settings.chat_model, &prompt, true)
        .await
        .map_err(|e| e.to_string())?;
    parse_freeresponse_question(&topic.slug, &raw).map_err(|e| e.to_string())
}

/// Grade a written answer against the question's rubric, persisting the result.
#[tauri::command]
pub async fn grade_free_response(
    state: State<'_, AppState>,
    question: FreeResponseQuestion,
    answer: String,
) -> Result<FreeResponseGrade, String> {
    let settings = {
        let db = state.db.lock().unwrap();
        db.get_settings().map_err(|e| e.to_string())?
    };

    let client = OllamaClient::new(&settings.ollama_url);
    let prompt = build_grading_prompt(&question, &answer);
    let raw = client
        .generate(&settings.chat_model, &prompt, true)
        .await
        .map_err(|e| e.to_string())?;
    let grade = parse_grade(&raw, question.max_score).map_err(|e| e.to_string())?;

    {
        let db = state.db.lock().unwrap();
        db.insert_free_response(
            &question.topic_slug,
            &question.prompt,
            &answer,
            grade.score,
            grade.max_score,
            &grade.feedback,
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(grade)
}
