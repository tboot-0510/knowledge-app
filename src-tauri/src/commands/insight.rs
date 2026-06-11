//! Insight commands: the local-LLM weakness report + study plan.

use crate::state::AppState;
use knowledge_core::learning::{build_weakness_prompt, load_catalog};
use knowledge_core::ollama::{OllamaClient, StreamChunk};
use std::collections::HashMap;
use tauri::ipc::Channel;
use tauri::State;

/// Stream a personalized weakness report and one-week study plan derived from
/// the user's progress statistics.
#[tauri::command]
pub async fn generate_weakness_report(
    state: State<'_, AppState>,
    on_event: Channel<StreamChunk>,
) -> Result<(), String> {
    let (settings, prompt) = {
        let db = state.db.lock().unwrap();
        let settings = db.get_settings().map_err(|e| e.to_string())?;
        let stats = db.progress_stats().map_err(|e| e.to_string())?;

        // Map topic slugs to display titles (catalog + custom).
        let mut titles: HashMap<String, String> = load_catalog()
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|t| (t.slug, t.title))
            .collect();
        for t in db.list_custom_topics().map_err(|e| e.to_string())? {
            titles.insert(t.slug, t.title);
        }

        let topics: Vec<(String, u32, u32)> = db
            .progress_by_topic()
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|(slug, (answered, correct, _))| {
                (titles.get(&slug).cloned().unwrap_or(slug), answered, correct)
            })
            .collect();

        (settings, build_weakness_prompt(&stats, &topics))
    };

    let client = OllamaClient::from_settings(&settings);
    let result = client
        .generate_stream(&settings.chat_model, &prompt, |chunk| {
            let _ = on_event.send(chunk);
        })
        .await;
    if let Err(e) = result {
        let _ = on_event.send(StreamChunk::Error { message: e.to_string() });
        return Err(e.to_string());
    }
    Ok(())
}
