//! Shared application state held by Tauri's managed-state container.

use knowledge_core::db::Db;
use knowledge_core::ollama::OllamaClient;
use std::path::PathBuf;
use std::sync::Mutex;

/// Global state. The DB is behind a `Mutex` because `rusqlite::Connection` is not
/// `Sync`; command handlers must scope their lock so it is never held across an
/// `.await` point.
pub struct AppState {
    pub db: Mutex<Db>,
    /// Per-user app data directory (DB file + cloned repos live here).
    pub data_dir: PathBuf,
}

impl AppState {
    pub fn new(db: Db, data_dir: PathBuf) -> Self {
        AppState {
            db: Mutex::new(db),
            data_dir,
        }
    }

    /// Build an Ollama client for the current settings (local or Ollama Cloud).
    pub fn ollama(&self) -> OllamaClient {
        let settings = self
            .db
            .lock()
            .unwrap()
            .get_settings()
            .unwrap_or_default();
        OllamaClient::from_settings(&settings)
    }

    /// Directory under which linked repos are cloned.
    pub fn repos_dir(&self) -> PathBuf {
        self.data_dir.join("repos")
    }
}
