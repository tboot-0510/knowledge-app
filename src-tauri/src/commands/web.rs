//! Web-tool commands backing `/url` (fetch a page) and `/search` (web search),
//! plus a streamed answer grounded in fetched content — all answered locally.

use crate::state::AppState;
use knowledge_core::models::{FetchedPage, SearchResult};
use knowledge_core::ollama::{OllamaClient, StreamChunk};
use knowledge_core::web;
use tauri::ipc::Channel;
use tauri::State;

/// Fetch a URL and return its readable text (`/url`).
#[tauri::command]
pub async fn fetch_url(url: String) -> Result<FetchedPage, String> {
    web::fetch_url(&url).await.map_err(|e| e.to_string())
}

/// Search the web and return results (`/search`).
#[tauri::command]
pub async fn web_search(query: String) -> Result<Vec<SearchResult>, String> {
    web::search_web(&query).await.map_err(|e| e.to_string())
}

/// Stream an answer to `question`, grounded in `context` (may be empty for a
/// plain local-LLM answer). `sources` are cited in the prompt.
#[tauri::command]
pub async fn ask_web(
    state: State<'_, AppState>,
    question: String,
    context: String,
    sources: Vec<String>,
    on_event: Channel<StreamChunk>,
) -> Result<(), String> {
    let settings = {
        let db = state.db.lock().unwrap();
        db.get_settings().map_err(|e| e.to_string())?
    };
    let prompt = web::build_web_prompt(&question, &context, &sources);
    let client = OllamaClient::new(&settings.ollama_url);
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
