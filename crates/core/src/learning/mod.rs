//! Daily learning domain: topic catalog, MCQ generation/validation, scoring.

pub mod catalog;
pub mod elo;
pub mod fallback;
pub mod followup;
pub mod freeresponse;
pub mod generator;
pub mod leetcode;
pub mod paths;
pub mod report;
pub mod scoring;
pub mod srs;
pub mod synthesize;

pub use catalog::{load_catalog, pick_topic, CatalogTopic};
pub use elo::{
    bloom_directive, bloom_for, level_prior, target_difficulty, tier_to_rating, update_ratings,
    BloomLevel, K_FACTOR,
};
pub use fallback::{generic_fallback, seed_questions};
pub use followup::build_followup_prompt;
pub use freeresponse::{
    build_freeresponse_prompt, build_grading_prompt, parse_freeresponse_question, parse_grade,
};
pub use generator::{build_mcq_prompt, parse_and_validate_mcqs, to_questions, GeneratedMcq};
pub use leetcode::{
    build_problem_prompt, build_review_prompt, is_solved, load_categories, parse_problem,
    parse_review, seed_problems,
};
pub use paths::{build_path_card, load_paths};
pub use report::build_weakness_prompt;
pub use scoring::{next_difficulty, update_streak};
pub use srs::{quality_from_correct, sm2};
pub use synthesize::{build_synthesis_prompt, parse_synthesized_topic, slugify};
