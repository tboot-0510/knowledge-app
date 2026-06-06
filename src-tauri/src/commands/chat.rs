//! Repo Q&A (RAG) command — streams the answer token-by-token via a Channel.

use crate::state::AppState;
use knowledge_core::models::RetrievedChunk;
use knowledge_core::ollama::{OllamaClient, StreamChunk};
use knowledge_core::repo::search::{build_rag_prompt, top_k};
use tauri::ipc::Channel;
use tauri::State;

/// Number of chunks retrieved as context for an answer.
const TOP_K: usize = 6;

/// Ask a question about a linked repo. Retrieves the most similar chunks and
/// streams a locally-generated, citation-grounded answer to `on_event`.
#[tauri::command]
pub async fn ask_repo(
    state: State<'_, AppState>,
    repo_id: i64,
    question: String,
    on_event: Channel<StreamChunk>,
) -> Result<(), String> {
    // Load settings + this repo's embeddings under a scoped lock.
    let (settings, candidates) = {
        let db = state.db.lock().unwrap();
        let settings = db.get_settings().map_err(|e| e.to_string())?;
        let candidates = db
            .embeddings_for_repo(repo_id)
            .map_err(|e| e.to_string())?;
        (settings, candidates)
    };
    if candidates.is_empty() {
        let _ = on_event.send(StreamChunk::Error {
            message: "This repository has no indexed content yet.".into(),
        });
        return Ok(());
    }

    let client = OllamaClient::new(&settings.ollama_url);

    // Embed the question and rank chunks.
    let qvec = match client.embed(&settings.embed_model, &question).await {
        Ok(v) => v,
        Err(e) => {
            let _ = on_event.send(StreamChunk::Error { message: e.to_string() });
            return Err(e.to_string());
        }
    };
    let ranked = top_k(&qvec, &candidates, TOP_K);

    // Hydrate the retrieved chunks (scoped lock, no await inside).
    let retrieved: Vec<RetrievedChunk> = {
        let db = state.db.lock().unwrap();
        ranked
            .iter()
            .filter_map(|(id, score)| {
                db.get_chunk(*id).ok().map(|c| RetrievedChunk {
                    file_path: c.file_path,
                    start_line: c.start_line,
                    end_line: c.end_line,
                    content: c.content,
                    score: *score,
                })
            })
            .collect()
    };

    let prompt = build_rag_prompt(&question, &retrieved);
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
