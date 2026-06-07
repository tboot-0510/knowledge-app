//! Tauri command handlers (Rust ⇄ React IPC).
//!
//! Each submodule groups related commands. They are registered in
//! `lib.rs::run` via `tauri::generate_handler!`.

pub mod chat;
pub mod coding;
pub mod daily;
pub mod insight;
pub mod models;
pub mod practice;
pub mod repo;
pub mod review;
pub mod settings;
pub mod system;
pub mod topics;
pub mod window;
