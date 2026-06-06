//! MCQ generation: prompt construction + strict parsing/validation of the
//! local LLM's JSON response.
//!
//! The network call itself lives in [`crate::ollama`]; everything here is pure
//! and unit-tested so we can guarantee malformed model output is rejected.

use crate::error::{Error, Result};
use crate::learning::catalog::CatalogTopic;
use crate::models::{Level, Question};
use serde::Deserialize;

/// Shape the model is asked to emit (per question).
#[derive(Debug, Clone, Deserialize)]
pub struct GeneratedMcq {
    pub prompt: String,
    pub choices: Vec<String>,
    pub correct_index: usize,
    pub explanation: String,
    #[serde(default = "default_difficulty")]
    pub difficulty: u8,
}

fn default_difficulty() -> u8 {
    3
}

#[derive(Debug, Deserialize)]
struct McqEnvelope {
    questions: Vec<GeneratedMcq>,
}

/// Build the prompt that asks the local model for `n` MCQs about `topic`,
/// calibrated to `level`, returning JSON only.
pub fn build_mcq_prompt(topic: &CatalogTopic, level: Level, n: usize, target_difficulty: u8) -> String {
    let points = if topic.talking_points.is_empty() {
        String::new()
    } else {
        format!(
            "\nKey sub-topics to potentially cover:\n- {}\n",
            topic.talking_points.join("\n- ")
        )
    };
    format!(
        r#"You are an expert technical interviewer creating a daily learning challenge for a {level} software engineer.

Topic: {title}
Area: {area}
Summary: {summary}{points}
Write exactly {n} multiple-choice questions that deepen practical, senior-level understanding of this topic. Each question must:
- be non-trivial and reflect real engineering trade-offs (avoid trivia and "all of the above")
- have exactly 4 answer choices, with exactly ONE correct
- include a concise explanation of why the correct answer is right
- have an integer "difficulty" from 1 (easier) to 5 (hardest); aim around {target_difficulty}

Respond with ONLY valid JSON, no prose, no markdown fences, matching exactly:
{{
  "questions": [
    {{
      "prompt": "string",
      "choices": ["string", "string", "string", "string"],
      "correct_index": 0,
      "explanation": "string",
      "difficulty": 3
    }}
  ]
}}"#,
        level = level.as_str(),
        title = topic.title,
        area = topic.area,
        summary = topic.summary,
        points = points,
        n = n,
        target_difficulty = target_difficulty,
    )
}

/// Extract the first balanced top-level JSON object from `raw`.
///
/// Local models often wrap JSON in ```json fences or add a sentence of preamble;
/// this finds the outermost `{ ... }` so parsing is robust to that.
fn extract_json_object(raw: &str) -> Option<&str> {
    let start = raw.find('{')?;
    let bytes = raw.as_bytes();
    let mut depth = 0usize;
    let mut in_str = false;
    let mut escaped = false;
    for i in start..bytes.len() {
        let c = bytes[i] as char;
        if in_str {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_str = false;
            }
            continue;
        }
        match c {
            '"' => in_str = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&raw[start..=i]);
                }
            }
            _ => {}
        }
    }
    None
}

/// Validate a single MCQ's invariants.
fn validate_one(idx: usize, q: &GeneratedMcq) -> Result<()> {
    if q.prompt.trim().is_empty() {
        return Err(Error::Validation(format!("question {idx}: empty prompt")));
    }
    if q.choices.len() != 4 {
        return Err(Error::Validation(format!(
            "question {idx}: expected 4 choices, got {}",
            q.choices.len()
        )));
    }
    if q.choices.iter().any(|c| c.trim().is_empty()) {
        return Err(Error::Validation(format!("question {idx}: a choice is empty")));
    }
    if q.correct_index >= q.choices.len() {
        return Err(Error::Validation(format!(
            "question {idx}: correct_index {} out of range",
            q.correct_index
        )));
    }
    if q.explanation.trim().is_empty() {
        return Err(Error::Validation(format!("question {idx}: empty explanation")));
    }
    Ok(())
}

/// Parse the model output and validate every question. Returns the validated
/// MCQs or an error describing the first problem found.
pub fn parse_and_validate_mcqs(raw: &str) -> Result<Vec<GeneratedMcq>> {
    let json = extract_json_object(raw)
        .ok_or_else(|| Error::InvalidMcqJson("no JSON object found in response".into()))?;
    let env: McqEnvelope =
        serde_json::from_str(json).map_err(|e| Error::InvalidMcqJson(e.to_string()))?;
    if env.questions.is_empty() {
        return Err(Error::Validation("model returned zero questions".into()));
    }
    for (i, q) in env.questions.iter().enumerate() {
        validate_one(i, q)?;
    }
    Ok(env.questions)
}

/// Convert validated MCQs into persistable [`Question`]s (clamping difficulty).
pub fn to_questions(topic_id: i64, mcqs: Vec<GeneratedMcq>) -> Vec<Question> {
    mcqs.into_iter()
        .map(|m| Question {
            id: 0,
            topic_id,
            prompt: m.prompt,
            choices: m.choices,
            correct_index: m.correct_index,
            explanation: m.explanation,
            difficulty: m.difficulty.clamp(1, 5),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"{"questions":[
        {"prompt":"What does CAP describe?","choices":["a","b","c","d"],
         "correct_index":1,"explanation":"because","difficulty":3}
    ]}"#;

    #[test]
    fn parses_clean_json() {
        let q = parse_and_validate_mcqs(GOOD).unwrap();
        assert_eq!(q.len(), 1);
        assert_eq!(q[0].correct_index, 1);
    }

    #[test]
    fn parses_json_wrapped_in_fences_and_prose() {
        let raw = format!("Sure! Here you go:\n```json\n{GOOD}\n```\nHope that helps.");
        let q = parse_and_validate_mcqs(&raw).unwrap();
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn rejects_wrong_choice_count() {
        let raw = r#"{"questions":[{"prompt":"q","choices":["a","b","c"],
            "correct_index":0,"explanation":"e","difficulty":2}]}"#;
        assert!(matches!(parse_and_validate_mcqs(raw), Err(Error::Validation(_))));
    }

    #[test]
    fn rejects_out_of_range_index() {
        let raw = r#"{"questions":[{"prompt":"q","choices":["a","b","c","d"],
            "correct_index":9,"explanation":"e","difficulty":2}]}"#;
        assert!(matches!(parse_and_validate_mcqs(raw), Err(Error::Validation(_))));
    }

    #[test]
    fn rejects_non_json() {
        assert!(matches!(
            parse_and_validate_mcqs("I cannot help with that."),
            Err(Error::InvalidMcqJson(_))
        ));
    }

    #[test]
    fn rejects_empty_question_set() {
        assert!(matches!(
            parse_and_validate_mcqs(r#"{"questions":[]}"#),
            Err(Error::Validation(_))
        ));
    }

    #[test]
    fn difficulty_is_clamped() {
        let raw = r#"{"questions":[{"prompt":"q","choices":["a","b","c","d"],
            "correct_index":0,"explanation":"e","difficulty":99}]}"#;
        let mcqs = parse_and_validate_mcqs(raw).unwrap();
        let qs = to_questions(1, mcqs);
        assert_eq!(qs[0].difficulty, 5);
    }
}
