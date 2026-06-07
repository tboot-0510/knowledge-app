//! In-app Ollama model manager: list installed, recommend, pull, delete.

use crate::state::AppState;
use knowledge_core::models::{ModelInfo, RecommendedModel};
use knowledge_core::ollama::{OllamaClient, PullProgress};
use tauri::ipc::Channel;
use tauri::State;

/// Locally installed models with sizes.
#[tauri::command]
pub async fn list_installed_models(state: State<'_, AppState>) -> Result<Vec<ModelInfo>, String> {
    state
        .ollama()
        .list_models_detailed()
        .await
        .map_err(|e| e.to_string())
}

/// Curated models we recommend for this app's workloads.
#[tauri::command]
pub fn recommended_models() -> Vec<RecommendedModel> {
    vec![
        RecommendedModel {
            name: "llama3.1:8b".into(),
            purpose: "Chat / explanations / grading".into(),
            note: "Strong general reasoning; good default on 16GB+ RAM.".into(),
        },
        RecommendedModel {
            name: "qwen2.5:7b".into(),
            purpose: "Chat / explanations".into(),
            note: "Excellent at technical Q&A; solid alternative to Llama.".into(),
        },
        RecommendedModel {
            name: "qwen2.5:3b".into(),
            purpose: "Fast MCQ generation".into(),
            note: "Small and quick; great for the mcq_model slot.".into(),
        },
        RecommendedModel {
            name: "gemma2:2b".into(),
            purpose: "Fast MCQ generation".into(),
            note: "Runs on ~8GB RAM; lowest latency for daily questions.".into(),
        },
        RecommendedModel {
            name: "nomic-embed-text".into(),
            purpose: "Embeddings (repo Q&A)".into(),
            note: "Required for indexing/searching linked repositories.".into(),
        },
    ]
}

/// Pull a model, streaming download progress to `on_event`.
#[tauri::command]
pub async fn pull_model(
    state: State<'_, AppState>,
    name: String,
    on_event: Channel<PullProgress>,
) -> Result<(), String> {
    let client = state.ollama();
    client
        .pull_model(&name, |p| {
            let _ = on_event.send(p);
        })
        .await
        .map_err(|e| e.to_string())
}

/// Delete a locally installed model.
#[tauri::command]
pub async fn delete_model(state: State<'_, AppState>, name: String) -> Result<(), String> {
    state
        .ollama()
        .delete_model(&name)
        .await
        .map_err(|e| e.to_string())
}
