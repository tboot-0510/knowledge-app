//! Topic-selection + per-topic progress commands for the Topics panel.

use crate::state::AppState;
use knowledge_core::learning::load_catalog;
use knowledge_core::models::{Difficulty, TopicCard, TopicPref};
use tauri::State;

/// List every catalog topic merged with the user's selection and progress, for
/// the Topics panel (select topics, set difficulty, track grades).
#[tauri::command]
pub fn list_topics(state: State<AppState>) -> Result<Vec<TopicCard>, String> {
    let db = state.db.lock().unwrap();
    let catalog = load_catalog().map_err(|e| e.to_string())?;
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
