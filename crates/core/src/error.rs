//! Shared error type for knowledge-core.

use serde::Serialize;

/// Result alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// All errors surfaced by the core. `Serialize` so Tauri commands can return it
/// directly to the frontend as a string message.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("http/ollama error: {0}")]
    Http(String),

    #[error("ollama is not reachable at {0} — is `ollama serve` running?")]
    OllamaUnreachable(String),

    #[error("model '{0}' is not installed in Ollama — pull it or pick an installed model in Settings (run `ollama pull {0}`)")]
    ModelNotFound(String),

    #[error("the model response could not be parsed as valid MCQ JSON: {0}")]
    InvalidMcqJson(String),

    #[error("validation failed: {0}")]
    Validation(String),

    #[error("git error: {0}")]
    Git(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("not found: {0}")]
    NotFound(String),
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Http(e.to_string())
    }
}

// Make the error serializable so it can cross the Tauri IPC boundary as a string.
impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
