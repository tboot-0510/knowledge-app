//! Curated topic catalog.
//!
//! The catalog is a bundled JSON file (compiled in via `include_str!`) tuned for
//! senior/staff/principal engineers. The local LLM later generates fresh MCQs
//! from the selected topic (the "hybrid" content approach).

use crate::error::Result;
use crate::models::Level;
use serde::Deserialize;

/// One catalog entry. `min_level` is the lowest seniority this topic targets.
#[derive(Debug, Clone, Deserialize)]
pub struct CatalogTopic {
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub area: String,
    pub min_level: Level,
    #[serde(default)]
    pub talking_points: Vec<String>,
}

const CATALOG_JSON: &str = include_str!("../../../../resources/catalog.json");

/// Parse the bundled catalog.
pub fn load_catalog() -> Result<Vec<CatalogTopic>> {
    let topics: Vec<CatalogTopic> = serde_json::from_str(CATALOG_JSON)?;
    Ok(topics)
}

/// Whether a topic is appropriate for a given seniority level (>= its min level).
fn level_rank(l: Level) -> u8 {
    match l {
        Level::Senior => 0,
        Level::Staff => 1,
        Level::Principal => 2,
    }
}

/// Pick the day's topic for `level`, avoiding the `recent` slugs where possible.
///
/// Deterministic given the inputs: it walks the eligible topics in catalog order
/// and returns the first that isn't in `recent`; if all are recent (or none are
/// eligible) it falls back to the first eligible topic, then the first topic.
pub fn pick_topic<'a>(
    catalog: &'a [CatalogTopic],
    level: Level,
    recent: &[String],
) -> Option<&'a CatalogTopic> {
    let eligible: Vec<&CatalogTopic> = catalog
        .iter()
        .filter(|t| level_rank(t.min_level) <= level_rank(level))
        .collect();
    if let Some(t) = eligible.iter().find(|t| !recent.contains(&t.slug)) {
        return Some(t);
    }
    eligible.first().copied().or_else(|| catalog.first())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_parses_and_is_nonempty() {
        let c = load_catalog().expect("catalog.json should parse");
        assert!(c.len() >= 8, "expected a decent catalog, got {}", c.len());
        // every slug unique
        let mut slugs: Vec<_> = c.iter().map(|t| t.slug.clone()).collect();
        slugs.sort();
        slugs.dedup();
        assert_eq!(slugs.len(), c.len(), "duplicate slugs in catalog");
    }

    #[test]
    fn senior_only_sees_senior_topics() {
        let c = load_catalog().unwrap();
        let t = pick_topic(&c, Level::Senior, &[]).unwrap();
        assert_eq!(t.min_level, Level::Senior);
    }

    #[test]
    fn avoids_recent_topics() {
        let c = load_catalog().unwrap();
        let first = pick_topic(&c, Level::Principal, &[]).unwrap().slug.clone();
        let second = pick_topic(&c, Level::Principal, &[first.clone()]).unwrap();
        assert_ne!(second.slug, first);
    }
}
