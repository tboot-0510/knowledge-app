//! Custom topics: turn a user's free-text topic into a structured catalog entry
//! via the local LLM. Prompt + parsing are pure and unit-tested.

use crate::error::{Error, Result};
use crate::learning::catalog::CatalogTopic;
use crate::learning::generator::extract_json_object;
use crate::models::Level;
use serde::Deserialize;

#[derive(Deserialize)]
struct RawTopic {
    title: String,
    summary: String,
    #[serde(default = "default_area")]
    area: String,
    #[serde(default)]
    talking_points: Vec<String>,
    #[serde(default)]
    min_level: Option<String>,
}

fn default_area() -> String {
    "custom".to_string()
}

/// Build a URL/file-safe slug from arbitrary text.
pub fn slugify(input: &str) -> String {
    let mut slug = String::new();
    let mut prev_dash = false;
    for c in input.trim().to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
            prev_dash = false;
        } else if !prev_dash && !slug.is_empty() {
            slug.push('-');
            prev_dash = true;
        }
    }
    let trimmed = slug.trim_end_matches('-').to_string();
    if trimmed.is_empty() {
        "custom-topic".to_string()
    } else {
        trimmed
    }
}

/// Prompt asking the model to expand `user_input` into a structured topic.
pub fn build_synthesis_prompt(user_input: &str) -> String {
    format!(
        r#"A senior software engineer wants to study this topic: "{user_input}"

Produce a concise, structured learning topic for it. Pick the most fitting "area" (a short kebab-case label like "distributed-systems", "blockchain", "networking", "security", "system-design", "concurrency"). Provide 3-5 key sub-topics.

Respond with ONLY valid JSON, no prose, no fences:
{{
  "title": "string",
  "summary": "one-sentence description",
  "area": "kebab-case-area",
  "min_level": "senior",
  "talking_points": ["...", "...", "..."]
}}"#,
        user_input = user_input,
    )
}

/// Parse the synthesized topic into a [`CatalogTopic`], deriving a slug.
pub fn parse_synthesized_topic(user_input: &str, raw: &str) -> Result<CatalogTopic> {
    let json = extract_json_object(raw)
        .ok_or_else(|| Error::InvalidMcqJson("no JSON object in response".into()))?;
    let t: RawTopic =
        serde_json::from_str(json).map_err(|e| Error::InvalidMcqJson(e.to_string()))?;
    if t.title.trim().is_empty() {
        return Err(Error::Validation("synthesized topic has no title".into()));
    }
    let slug = slugify(if t.title.trim().is_empty() { user_input } else { &t.title });
    Ok(CatalogTopic {
        slug,
        title: t.title,
        summary: t.summary,
        area: t.area,
        min_level: t.min_level.map(|s| Level::from_str_lenient(&s)).unwrap_or(Level::Senior),
        talking_points: t.talking_points,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_handles_punctuation_and_spaces() {
        assert_eq!(slugify("Rust Async/Await!"), "rust-async-await");
        assert_eq!(slugify("  EVM  Internals "), "evm-internals");
        assert_eq!(slugify("***"), "custom-topic");
    }

    #[test]
    fn parses_synthesized_topic() {
        let raw = r#"Sure:
        {"title":"WebAssembly","summary":"Portable bytecode.","area":"systems","min_level":"staff","talking_points":["sandboxing","wasi"]}"#;
        let t = parse_synthesized_topic("wasm", raw).unwrap();
        assert_eq!(t.slug, "webassembly");
        assert_eq!(t.area, "systems");
        assert_eq!(t.min_level, Level::Staff);
        assert_eq!(t.talking_points.len(), 2);
    }

    #[test]
    fn rejects_empty_title() {
        let raw = r#"{"title":"","summary":"x"}"#;
        assert!(parse_synthesized_topic("x", raw).is_err());
    }
}
