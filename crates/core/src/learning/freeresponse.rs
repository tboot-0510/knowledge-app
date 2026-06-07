//! Free-response (open-ended) practice: prompt construction for generating an
//! open question, and for grading the user's answer against a rubric. The model
//! calls live in `commands`; parsing/validation here is pure and unit-tested.

use crate::error::{Error, Result};
use crate::learning::catalog::CatalogTopic;
use crate::learning::generator::extract_json_object;
use crate::models::{Difficulty, FreeResponseGrade, FreeResponseQuestion, Level};
use serde::Deserialize;

#[derive(Deserialize)]
struct RawQuestion {
    prompt: String,
    #[serde(default)]
    rubric: Vec<String>,
    #[serde(default = "default_max")]
    max_score: u32,
}

fn default_max() -> u32 {
    10
}

/// Build the prompt asking the model for one open-ended, interview-style question.
pub fn build_freeresponse_prompt(topic: &CatalogTopic, level: Level, difficulty: Difficulty) -> String {
    format!(
        r#"You are a senior interviewer at a top technology company conducting a {difficulty}-tier interview of a {level} engineer.

Topic: {title}
Area: {area}
Summary: {summary}

Write ONE open-ended interview question on this topic that requires a written explanation or design (not multiple choice). Provide a grading rubric of 3-5 concrete criteria a strong answer must satisfy.

Respond with ONLY valid JSON, no prose, no fences:
{{
  "prompt": "string",
  "rubric": ["criterion 1", "criterion 2", "criterion 3"],
  "max_score": 10
}}"#,
        difficulty = difficulty.as_str(),
        level = level.as_str(),
        title = topic.title,
        area = topic.area,
        summary = topic.summary,
    )
}

/// Parse + validate a generated free-response question.
pub fn parse_freeresponse_question(topic_slug: &str, raw: &str) -> Result<FreeResponseQuestion> {
    let json = extract_json_object(raw)
        .ok_or_else(|| Error::InvalidMcqJson("no JSON object in response".into()))?;
    let q: RawQuestion =
        serde_json::from_str(json).map_err(|e| Error::InvalidMcqJson(e.to_string()))?;
    if q.prompt.trim().is_empty() {
        return Err(Error::Validation("empty question prompt".into()));
    }
    Ok(FreeResponseQuestion {
        topic_slug: topic_slug.to_string(),
        prompt: q.prompt,
        rubric: q.rubric,
        max_score: q.max_score.max(1),
    })
}

/// Build the prompt that grades a user's free-response answer against the rubric.
pub fn build_grading_prompt(question: &FreeResponseQuestion, answer: &str) -> String {
    let rubric = if question.rubric.is_empty() {
        "(no explicit rubric — judge correctness, depth, and clarity)".to_string()
    } else {
        question
            .rubric
            .iter()
            .map(|c| format!("- {c}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    format!(
        r#"You are a fair but rigorous senior interviewer grading a candidate's written answer.

Question: {prompt}

Rubric:
{rubric}

Candidate's answer:
{answer}

Grade the answer out of {max}. Be specific and honest. Respond with ONLY valid JSON, no prose, no fences:
{{
  "score": 0,
  "max_score": {max},
  "feedback": "2-4 sentences of actionable feedback",
  "strengths": ["..."],
  "gaps": ["..."]
}}"#,
        prompt = question.prompt,
        rubric = rubric,
        answer = answer,
        max = question.max_score,
    )
}

/// Parse + validate the model's grade, clamping the score to `[0, max_score]`.
pub fn parse_grade(raw: &str, max_score: u32) -> Result<FreeResponseGrade> {
    let json = extract_json_object(raw)
        .ok_or_else(|| Error::InvalidMcqJson("no JSON object in response".into()))?;
    let mut g: FreeResponseGrade =
        serde_json::from_str(json).map_err(|e| Error::InvalidMcqJson(e.to_string()))?;
    g.max_score = max_score;
    g.score = g.score.min(max_score);
    if g.feedback.trim().is_empty() {
        g.feedback = "No feedback provided.".to_string();
    }
    Ok(g)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn topic() -> CatalogTopic {
        CatalogTopic {
            slug: "caching".into(),
            title: "Caching".into(),
            summary: "...".into(),
            area: "system-design".into(),
            min_level: Level::Senior,
            talking_points: vec![],
        }
    }

    #[test]
    fn question_prompt_requests_json_and_rubric() {
        let p = build_freeresponse_prompt(&topic(), Level::Staff, Difficulty::Hard);
        assert!(p.contains("open-ended"));
        assert!(p.contains("rubric"));
        assert!(p.to_lowercase().contains("hard"));
    }

    #[test]
    fn parses_question_with_fences() {
        let raw = "```json\n{\"prompt\":\"Design a cache\",\"rubric\":[\"a\",\"b\"],\"max_score\":10}\n```";
        let q = parse_freeresponse_question("caching", raw).unwrap();
        assert_eq!(q.prompt, "Design a cache");
        assert_eq!(q.rubric.len(), 2);
        assert_eq!(q.topic_slug, "caching");
    }

    #[test]
    fn grade_is_clamped_to_max() {
        let raw = r#"{"score": 99, "max_score": 7, "feedback": "good", "strengths": [], "gaps": []}"#;
        let g = parse_grade(raw, 10).unwrap();
        assert_eq!(g.max_score, 10);
        assert_eq!(g.score, 10); // clamped to caller max
    }

    #[test]
    fn rejects_non_json_grade() {
        assert!(parse_grade("nope", 10).is_err());
    }
}
