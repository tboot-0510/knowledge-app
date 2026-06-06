//! knowledge-core
//!
//! Platform-agnostic core for knowledge-app. Contains everything that does NOT
//! depend on Tauri or the macOS UI layer, so it can be compiled and unit-tested
//! on any platform (including the Linux CI used to develop this project).
//!
//! Modules:
//! - [`error`]    — shared error type.
//! - [`models`]   — serde data-transfer objects shared with the frontend.
//! - [`db`]       — SQLite (rusqlite) connection + embedded migrations.
//! - [`ollama`]   — local-LLM client (generate / embeddings / model list).
//! - [`learning`] — daily topics, MCQ generation/validation, scoring & streaks.
//! - [`repo`]     — GitHub repo clone, chunking, embedding index, RAG retrieval.
//! - [`scheduler`]— once-per-day gate + launchd plist text generation (pure).

pub mod db;
pub mod error;
pub mod learning;
pub mod models;
pub mod ollama;
pub mod repo;
pub mod scheduler;

pub use error::{Error, Result};
