//! LeetCode-style coding practice: data-structure categories, a seed problem
//! bank, and prompts for generating problems and reviewing submitted code.
//!
//! Since the app is local-only with no code-execution sandbox, solutions are
//! reviewed by the local LLM (correctness reasoning + complexity + edge cases +
//! optimal approach), analogous to the free-response feature. Prompt building
//! and parsing here are pure and unit-tested.

use crate::error::{Error, Result};
use crate::learning::generator::extract_json_object;
use crate::models::{CodeReview, CodingProblem, Difficulty, DsCategory};
use serde::Deserialize;
use std::collections::HashMap;

const CATEGORIES_JSON: &str = include_str!("../../../../resources/ds_categories.json");
const SEEDS_JSON: &str = include_str!("../../../../resources/leetcode_seeds.json");

/// Parse the data-structure category catalog.
pub fn load_categories() -> Result<Vec<DsCategory>> {
    Ok(serde_json::from_str(CATEGORIES_JSON)?)
}

/// A seed problem as stored in the bank (category injected at load time).
#[derive(Deserialize)]
struct RawProblem {
    title: String,
    prompt: String,
    #[serde(default)]
    examples: Vec<String>,
    #[serde(default)]
    constraints: Vec<String>,
    difficulty: Difficulty,
    #[serde(default)]
    starter_signature: Option<String>,
    #[serde(default)]
    optimal_time: String,
    #[serde(default)]
    optimal_space: String,
}

impl RawProblem {
    fn into_problem(self, category_slug: &str) -> CodingProblem {
        CodingProblem {
            category_slug: category_slug.to_string(),
            title: self.title,
            prompt: self.prompt,
            examples: self.examples,
            constraints: self.constraints,
            difficulty: self.difficulty,
            starter_signature: self.starter_signature,
            optimal_time: self.optimal_time,
            optimal_space: self.optimal_space,
        }
    }
}

/// Load the seed problem bank, keyed by category slug.
pub fn load_seed_bank() -> HashMap<String, Vec<CodingProblem>> {
    let raw: HashMap<String, Vec<RawProblem>> = serde_json::from_str(SEEDS_JSON).unwrap_or_default();
    raw.into_iter()
        .map(|(slug, problems)| {
            let mapped = problems.into_iter().map(|p| p.into_problem(&slug)).collect();
            (slug, mapped)
        })
        .collect()
}

/// Seed problems for a category (empty if unknown).
pub fn seed_problems(category_slug: &str) -> Vec<CodingProblem> {
    load_seed_bank().remove(category_slug).unwrap_or_default()
}

/// Prompt asking the model to produce a LeetCode-style problem in `category`.
pub fn build_problem_prompt(category: &DsCategory, difficulty: Difficulty) -> String {
    format!(
        r#"You are a problem setter for a big-tech coding interview. Create ONE original {difficulty}-difficulty coding problem in the category "{title}" ({description}).

The problem must be self-contained, unambiguous, and solvable in code. Include 1-2 worked examples, the constraints, a function signature, and the optimal time/space complexity. Do NOT include the solution.

Respond with ONLY valid JSON, no prose, no fences:
{{
  "title": "string",
  "prompt": "full problem statement",
  "examples": ["input -> output", "..."],
  "constraints": ["...", "..."],
  "difficulty": "{difficulty}",
  "starter_signature": "def solve(...):",
  "optimal_time": "O(n)",
  "optimal_space": "O(1)"
}}"#,
        difficulty = difficulty.as_str(),
        title = category.title,
        description = category.description,
    )
}

/// Parse + validate a generated problem, injecting its category.
pub fn parse_problem(category_slug: &str, raw: &str) -> Result<CodingProblem> {
    let json = extract_json_object(raw)
        .ok_or_else(|| Error::InvalidMcqJson("no JSON object in response".into()))?;
    let p: RawProblem =
        serde_json::from_str(json).map_err(|e| Error::InvalidMcqJson(e.to_string()))?;
    if p.title.trim().is_empty() || p.prompt.trim().is_empty() {
        return Err(Error::Validation("problem is missing a title or prompt".into()));
    }
    Ok(p.into_problem(category_slug))
}

/// Prompt asking the model to review a submitted solution.
pub fn build_review_prompt(problem: &CodingProblem, code: &str, language: &str) -> String {
    let examples = if problem.examples.is_empty() {
        String::new()
    } else {
        format!("\nExamples:\n- {}\n", problem.examples.join("\n- "))
    };
    format!(
        r#"You are a rigorous coding interviewer reviewing a candidate's solution. Reason carefully about correctness by mentally tracing the examples and edge cases. Do NOT execute code; analyze it.

Problem ({difficulty}): {title}
{prompt}{examples}
Optimal complexity: time {opt_time}, space {opt_space}.

Candidate's solution ({language}):
```
{code}
```

Grade out of 10. Respond with ONLY valid JSON, no prose, no fences:
{{
  "verdict": "correct | partial | incorrect",
  "score": 0,
  "max_score": 10,
  "time_complexity": "O(...) of the submission",
  "space_complexity": "O(...) of the submission",
  "correctness": "1-2 sentences on whether it actually solves the problem",
  "edge_cases_missed": ["..."],
  "feedback": "2-4 sentences of specific, actionable feedback",
  "optimal_approach": "a concise description of the optimal approach"
}}"#,
        difficulty = problem.difficulty.as_str(),
        title = problem.title,
        prompt = problem.prompt,
        examples = examples,
        opt_time = if problem.optimal_time.is_empty() { "?" } else { &problem.optimal_time },
        opt_space = if problem.optimal_space.is_empty() { "?" } else { &problem.optimal_space },
        language = language,
        code = code,
    )
}

/// Parse + validate a code review, clamping the score.
pub fn parse_review(raw: &str, max_score: u32) -> Result<CodeReview> {
    let json = extract_json_object(raw)
        .ok_or_else(|| Error::InvalidMcqJson("no JSON object in response".into()))?;
    let mut r: CodeReview =
        serde_json::from_str(json).map_err(|e| Error::InvalidMcqJson(e.to_string()))?;
    r.max_score = max_score;
    r.score = r.score.min(max_score);
    if r.verdict.trim().is_empty() {
        r.verdict = "partial".to_string();
    }
    if r.feedback.trim().is_empty() {
        r.feedback = "No feedback provided.".to_string();
    }
    Ok(r)
}

/// Whether a review counts as "solved" (for progress tracking).
pub fn is_solved(review: &CodeReview) -> bool {
    review.verdict.eq_ignore_ascii_case("correct")
        || (review.max_score > 0 && review.score * 100 >= review.max_score * 80)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categories_parse_and_are_unique() {
        let c = load_categories().unwrap();
        assert!(c.len() >= 15, "expected a broad category set, got {}", c.len());
        let mut slugs: Vec<_> = c.iter().map(|x| x.slug.clone()).collect();
        slugs.sort();
        slugs.dedup();
        assert_eq!(slugs.len(), c.len(), "duplicate category slugs");
    }

    #[test]
    fn every_category_has_a_seed_problem() {
        let bank = load_seed_bank();
        for c in load_categories().unwrap() {
            let probs = bank.get(&c.slug);
            assert!(
                probs.map(|p| !p.is_empty()).unwrap_or(false),
                "category {} has no seed problem",
                c.slug
            );
            // category_slug is correctly injected
            assert_eq!(probs.unwrap()[0].category_slug, c.slug);
        }
    }

    #[test]
    fn problem_prompt_requests_json_no_solution() {
        let cat = DsCategory {
            slug: "graphs".into(),
            title: "Graphs".into(),
            description: "BFS/DFS".into(),
        };
        let p = build_problem_prompt(&cat, Difficulty::Hard);
        assert!(p.contains("Graphs"));
        assert!(p.to_lowercase().contains("hard"));
        assert!(p.contains("starter_signature"));
        assert!(p.to_lowercase().contains("do not include the solution"));
    }

    #[test]
    fn parses_problem_with_fences() {
        let raw = "```json\n{\"title\":\"T\",\"prompt\":\"do x\",\"difficulty\":\"medium\",\"examples\":[],\"constraints\":[],\"optimal_time\":\"O(n)\",\"optimal_space\":\"O(1)\"}\n```";
        let p = parse_problem("arrays-hashing", raw).unwrap();
        assert_eq!(p.category_slug, "arrays-hashing");
        assert_eq!(p.title, "T");
        assert_eq!(p.difficulty, Difficulty::Medium);
    }

    #[test]
    fn review_clamps_and_flags_solved() {
        let raw = r#"{"verdict":"correct","score":15,"max_score":3,"time_complexity":"O(n)","space_complexity":"O(1)","correctness":"yes","edge_cases_missed":[],"feedback":"good","optimal_approach":"hash map"}"#;
        let r = parse_review(raw, 10).unwrap();
        assert_eq!(r.max_score, 10);
        assert_eq!(r.score, 10);
        assert!(is_solved(&r));
    }

    #[test]
    fn partial_below_threshold_is_not_solved() {
        let r = CodeReview {
            verdict: "partial".into(),
            score: 5,
            max_score: 10,
            time_complexity: "O(n^2)".into(),
            space_complexity: "O(1)".into(),
            correctness: "misses cases".into(),
            edge_cases_missed: vec!["empty input".into()],
            feedback: "...".into(),
            optimal_approach: "...".into(),
        };
        assert!(!is_solved(&r));
    }
}
