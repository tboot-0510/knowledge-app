//! Curated learning paths (ordered topic tracks) and their progress rollup.

use crate::error::Result;
use crate::models::{PathCard, PathStep};
use serde::Deserialize;
use std::collections::HashMap;

/// A learning path definition as stored in `paths.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct LearningPathDef {
    pub slug: String,
    pub title: String,
    pub description: String,
    pub topic_slugs: Vec<String>,
}

const PATHS_JSON: &str = include_str!("../../../../resources/paths.json");

/// Parse the bundled learning paths.
pub fn load_paths() -> Result<Vec<LearningPathDef>> {
    Ok(serde_json::from_str(PATHS_JSON)?)
}

/// Build a [`PathCard`] for a path: resolve each step's title from `titles` and
/// its progress from `progress` (slug → (answered, correct)).
pub fn build_path_card(
    def: &LearningPathDef,
    titles: &HashMap<String, String>,
    progress: &HashMap<String, (u32, u32)>,
) -> PathCard {
    let steps: Vec<PathStep> = def
        .topic_slugs
        .iter()
        .map(|slug| {
            let (answered, correct) = progress.get(slug).copied().unwrap_or((0, 0));
            PathStep {
                slug: slug.clone(),
                title: titles.get(slug).cloned().unwrap_or_else(|| slug.clone()),
                answered,
                correct,
            }
        })
        .collect();
    let started = steps.iter().filter(|s| s.answered > 0).count() as u32;
    PathCard {
        slug: def.slug.clone(),
        title: def.title.clone(),
        description: def.description.clone(),
        total: steps.len() as u32,
        started,
        steps,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_parse_and_reference_known_slugs() {
        let paths = load_paths().unwrap();
        assert!(paths.len() >= 3);
        let catalog = crate::learning::catalog::load_catalog().unwrap();
        let known: std::collections::HashSet<_> = catalog.iter().map(|t| t.slug.as_str()).collect();
        for p in &paths {
            assert!(!p.topic_slugs.is_empty());
            for s in &p.topic_slugs {
                assert!(known.contains(s.as_str()), "path references unknown slug {s}");
            }
        }
    }

    #[test]
    fn path_card_counts_started_steps() {
        let def = LearningPathDef {
            slug: "t".into(),
            title: "T".into(),
            description: "d".into(),
            topic_slugs: vec!["a".into(), "b".into(), "c".into()],
        };
        let titles = HashMap::from([("a".to_string(), "Alpha".to_string())]);
        let progress = HashMap::from([("a".to_string(), (4u32, 3u32)), ("b".to_string(), (1, 0))]);
        let card = build_path_card(&def, &titles, &progress);
        assert_eq!(card.total, 3);
        assert_eq!(card.started, 2); // a and b have answers, c doesn't
        assert_eq!(card.steps[0].title, "Alpha");
        assert_eq!(card.steps[2].title, "c"); // fallback to slug
    }
}
