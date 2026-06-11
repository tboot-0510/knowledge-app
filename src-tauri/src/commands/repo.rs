//! Repository linking + indexing commands.

use crate::state::AppState;
use knowledge_core::models::{Repo, RepoStatus};
use knowledge_core::ollama::OllamaClient;
use knowledge_core::repo::clone::{clone_repo, head_commit, repo_name_from_url};
use knowledge_core::repo::index::embed_repo;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

#[derive(Clone, Serialize)]
struct IndexProgress {
    repo_id: i64,
    done: u32,
    total: u32,
}

/// Link a GitHub repo by URL: clone it, chunk + embed every indexable file, and
/// store the vectors locally. Emits `repo-index-progress` events while indexing.
/// Returns the new repo id.
#[tauri::command]
pub async fn link_repo(
    app: AppHandle,
    state: State<'_, AppState>,
    url: String,
) -> Result<i64, String> {
    let repos_root = state.repos_dir();
    let name = repo_name_from_url(&url);
    let local_path = repos_root.join(&name);
    let local_path_str = local_path.to_string_lossy().into_owned();

    // Create the repo row up front so the UI can show it as pending.
    let (settings, repo_id) = {
        let db = state.db.lock().unwrap();
        let s = db.get_settings().map_err(|e| e.to_string())?;
        let repo_id = db
            .insert_repo(&name, &url, &local_path_str)
            .map_err(|e| e.to_string())?;
        db.set_repo_status(repo_id, RepoStatus::Cloning, None)
            .map_err(|e| e.to_string())?;
        (s, repo_id)
    };

    // Clone on a blocking thread (it shells out to `git`).
    let url_for_clone = url.clone();
    let root_for_clone = repos_root.clone();
    let cloned = tokio::task::spawn_blocking(move || clone_repo(&url_for_clone, &root_for_clone))
        .await
        .map_err(|e| e.to_string())?;
    let path = match cloned {
        Ok(p) => p,
        Err(e) => {
            let _ = state
                .db
                .lock()
                .unwrap()
                .set_repo_status(repo_id, RepoStatus::Error, Some(&e.to_string()));
            return Err(e.to_string());
        }
    };

    // Move to indexing.
    state
        .db
        .lock()
        .unwrap()
        .set_repo_status(repo_id, RepoStatus::Indexing, None)
        .map_err(|e| e.to_string())?;

    // Embed all chunks (await — no DB lock held here).
    let client = OllamaClient::from_settings(&settings);
    let app_for_progress = app.clone();
    let embed_result = embed_repo(&client, &settings.embed_model, &path, move |done, total| {
        let _ = app_for_progress.emit(
            "repo-index-progress",
            IndexProgress { repo_id, done, total },
        );
    })
    .await;

    let (embedded, file_count) = match embed_result {
        Ok(r) => r,
        Err(e) => {
            let _ = state
                .db
                .lock()
                .unwrap()
                .set_repo_status(repo_id, RepoStatus::Error, Some(&e.to_string()));
            return Err(e.to_string());
        }
    };

    // Persist chunks + vectors and finalize.
    {
        let db = state.db.lock().unwrap();
        db.clear_repo_chunks(repo_id).map_err(|e| e.to_string())?;
        for ec in &embedded {
            db.insert_chunk_with_embedding(
                repo_id,
                &ec.file_path,
                ec.start_line,
                ec.end_line,
                &ec.content,
                ec.token_estimate,
                &ec.embedding,
            )
            .map_err(|e| e.to_string())?;
        }
        let head = head_commit(&path);
        db.finalize_repo_index(repo_id, file_count, embedded.len() as u32, head.as_deref())
            .map_err(|e| e.to_string())?;
    }

    Ok(repo_id)
}

#[tauri::command]
pub fn list_repos(state: State<AppState>) -> Result<Vec<Repo>, String> {
    state.db.lock().unwrap().list_repos().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_repo(state: State<AppState>, repo_id: i64) -> Result<Repo, String> {
    state
        .db
        .lock()
        .unwrap()
        .get_repo(repo_id)
        .map_err(|e| e.to_string())
}

/// Delete a repo: remove its DB rows (cascades to chunks/embeddings) and its
/// cloned files on disk.
#[tauri::command]
pub fn delete_repo(state: State<AppState>, repo_id: i64) -> Result<(), String> {
    let local_path = {
        let db = state.db.lock().unwrap();
        let path = db.get_repo(repo_id).ok().map(|r| r.local_path);
        db.delete_repo(repo_id).map_err(|e| e.to_string())?;
        path
    };
    if let Some(p) = local_path {
        if !p.is_empty() {
            let _ = std::fs::remove_dir_all(&p);
        }
    }
    Ok(())
}
