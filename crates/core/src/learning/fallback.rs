//! Bundled seed question bank — the reliability half of the hybrid content model.
//!
//! When the local LLM is unavailable or returns invalid output, the daily
//! challenge falls back to these curated, pre-validated MCQs so the app always
//! works offline.

use crate::learning::generator::GeneratedMcq;
use std::collections::HashMap;

const SEED_JSON: &str = include_str!("../../../../resources/seed_questions.json");

/// Parse the seed bank into a slug → questions map.
pub fn load_seed_bank() -> HashMap<String, Vec<GeneratedMcq>> {
    serde_json::from_str(SEED_JSON).unwrap_or_default()
}

/// Seed questions for a topic slug. Returns an empty vec if the slug is unknown.
pub fn seed_questions(slug: &str) -> Vec<GeneratedMcq> {
    load_seed_bank().remove(slug).unwrap_or_default()
}

/// A last-resort generic question, used only if a slug has no seed entry and the
/// LLM is unreachable, so the UI never shows an empty challenge.
pub fn generic_fallback(topic_title: &str) -> GeneratedMcq {
    GeneratedMcq {
        prompt: format!(
            "Your local model is unavailable, so here is a reflective prompt on \"{topic_title}\": which habit best reflects senior engineering judgement?"
        ),
        choices: vec![
            "Make the smallest reversible change and validate with data".to_string(),
            "Rewrite the whole system before measuring anything".to_string(),
            "Optimize the part that is easiest, not the bottleneck".to_string(),
            "Avoid writing down the decision or its trade-offs".to_string(),
        ],
        correct_index: 0,
        explanation: "Senior engineers bias toward small, reversible, measurable changes and document trade-offs. Start Ollama to get fresh questions generated for this topic.".to_string(),
        difficulty: 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::learning::catalog::load_catalog;

    #[test]
    fn seed_bank_parses() {
        let bank = load_seed_bank();
        assert!(!bank.is_empty());
    }

    #[test]
    fn every_catalog_slug_has_a_seed_question() {
        let bank = load_seed_bank();
        for t in load_catalog().unwrap() {
            let qs = bank.get(&t.slug);
            assert!(qs.map(|q| !q.is_empty()).unwrap_or(false), "missing seed for {}", t.slug);
        }
    }

    #[test]
    fn seed_questions_are_valid_mcqs() {
        for (slug, qs) in load_seed_bank() {
            for q in qs {
                assert_eq!(q.choices.len(), 4, "{slug}: wrong choice count");
                assert!(q.correct_index < 4, "{slug}: bad correct_index");
                assert!(!q.explanation.trim().is_empty(), "{slug}: empty explanation");
            }
        }
    }
}
