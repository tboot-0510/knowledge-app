//! Tauri command handlers (Rust ⇄ React IPC).
//!
//! Each submodule groups related commands. They are registered in
//! `lib.rs::run` via `tauri::generate_handler!`.

pub mod chat;
pub mod daily;
pub mod repo;
pub mod settings;
pub mod window;
