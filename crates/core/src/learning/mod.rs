//! Daily learning domain: topic catalog, MCQ generation/validation, scoring.

pub mod catalog;
pub mod fallback;
pub mod followup;
pub mod generator;
pub mod scoring;

pub use catalog::{load_catalog, pick_topic, CatalogTopic};
pub use fallback::{generic_fallback, seed_questions};
pub use followup::build_followup_prompt;
pub use generator::{build_mcq_prompt, parse_and_validate_mcqs, to_questions, GeneratedMcq};
pub use scoring::{next_difficulty, update_streak};
