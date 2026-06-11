//! Topic-selection, custom topics, learning paths, and per-topic progress.

use crate::state::AppState;
use knowledge_core::learning::{
    build_path_card, build_synthesis_prompt, load_catalog, load_paths, parse_synthesized_topic,
};
use knowledge_core::models::{Difficulty, PathCard, TopicCard, TopicPref, TopicRating};
use knowledge_core::ollama::OllamaClient;
use std::collections::HashMap;
use tauri::State;

/// List every catalog + custom topic merged with the user's selection and
/// progress, for the Topics panel.
#[tauri::command]
pub fn list_topics(state: State<AppState>) -> Result<Vec<TopicCard>, String> {
    let db = state.db.lock().unwrap();
    let mut catalog = load_catalog().map_err(|e| e.to_string())?;
    catalog.extend(db.list_custom_topics().map_err(|e| e.to_string())?);
    let prefs = db.topic_prefs().map_err(|e| e.to_string())?;
    let progress = db.progress_by_topic().map_err(|e| e.to_string())?;

    let cards = catalog
        .into_iter()
        .map(|t| {
            let pref = prefs.get(&t.slug);
            let (answered, correct, by_difficulty) =
                progress.get(&t.slug).cloned().unwrap_or((0, 0, Vec::new()));
            TopicCard {
                slug: t.slug,
                title: t.title,
                summary: t.summary,
                area: t.area,
                min_level: t.min_level,
                enabled: pref.map(|p| p.enabled).unwrap_or(true),
                target_difficulty: pref.map(|p| p.target_difficulty).unwrap_or_default(),
                answered,
                correct,
                by_difficulty,
            }
        })
        .collect();
    Ok(cards)
}

/// Select/deselect a topic and set the difficulty tier its questions target.
#[tauri::command]
pub fn set_topic_pref(
    state: State<AppState>,
    slug: String,
    enabled: bool,
    target_difficulty: Difficulty,
) -> Result<(), String> {
    state
        .db
        .lock()
        .unwrap()
        .set_topic_pref(&TopicPref {
            slug,
            enabled,
            target_difficulty,
        })
        .map_err(|e| e.to_string())
}

/// Synthesize a custom topic from free text via the local LLM and persist it.
/// Returns the new topic's slug.
#[tauri::command]
pub async fn add_custom_topic(state: State<'_, AppState>, name: String) -> Result<String, String> {
    if name.trim().is_empty() {
        return Err("topic name is empty".into());
    }
    let settings = {
        let db = state.db.lock().unwrap();
        db.get_settings().map_err(|e| e.to_string())?
    };
    let client = OllamaClient::from_settings(&settings);
    let prompt = build_synthesis_prompt(&name);
    let raw = client
        .generate(&settings.chat_model, &prompt, true)
        .await
        .map_err(|e| e.to_string())?;
    let topic = parse_synthesized_topic(&name, &raw).map_err(|e| e.to_string())?;
    let slug = topic.slug.clone();
    {
        let db = state.db.lock().unwrap();
        db.insert_custom_topic(&topic).map_err(|e| e.to_string())?;
    }
    Ok(slug)
}

#[tauri::command]
pub fn delete_custom_topic(state: State<AppState>, slug: String) -> Result<(), String> {
    state
        .db
        .lock()
        .unwrap()
        .delete_custom_topic(&slug)
        .map_err(|e| e.to_string())
}

/// Per-topic Elo skill ratings (for the Stats "skill by topic" view).
#[tauri::command]
pub fn list_topic_ratings(state: State<AppState>) -> Result<Vec<TopicRating>, String> {
    state.db.lock().unwrap().list_ratings().map_err(|e| e.to_string())
}

/// List curated learning paths with the user's progress through each.
#[tauri::command]
pub fn list_paths(state: State<AppState>) -> Result<Vec<PathCard>, String> {
    let db = state.db.lock().unwrap();
    let defs = load_paths().map_err(|e| e.to_string())?;

    // Resolve step titles from catalog + custom topics.
    let mut titles: HashMap<String, String> = load_catalog()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|t| (t.slug, t.title))
        .collect();
    for t in db.list_custom_topics().map_err(|e| e.to_string())? {
        titles.insert(t.slug, t.title);
    }

    // Reduce per-topic progress to (answered, correct).
    let progress: HashMap<String, (u32, u32)> = db
        .progress_by_topic()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|(slug, (answered, correct, _))| (slug, (answered, correct)))
        .collect();

    Ok(defs
        .iter()
        .map(|d| build_path_card(d, &titles, &progress))
        .collect())
}
