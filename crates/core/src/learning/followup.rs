//! Follow-up prompt construction for the post-answer "explain / follow-up / ask"
//! interaction. Pure and unit-tested; the streaming call lives in `commands`.

use crate::models::{FollowupMode, Question};

/// Render the answered question (with the correct choice marked) as context.
fn question_context(q: &Question) -> String {
    let mut choices = String::new();
    for (i, c) in q.choices.iter().enumerate() {
        let marker = if i == q.correct_index { " (correct)" } else { "" };
        choices.push_str(&format!("  {}. {}{}\n", (b'A' + i as u8) as char, c, marker));
    }
    format!(
        "Question: {prompt}\nChoices:\n{choices}Reference explanation: {explanation}\n",
        prompt = q.prompt,
        choices = choices,
        explanation = q.explanation,
    )
}

/// Build the prompt for a follow-up interaction about an answered question.
pub fn build_followup_prompt(
    q: &Question,
    mode: FollowupMode,
    user_query: Option<&str>,
) -> String {
    let context = question_context(q);
    let instruction = match mode {
        FollowupMode::Explain => {
            "Act as a staff-level engineering mentor. Explain the underlying concept in depth: \
             why the correct answer is right, why each other option is tempting but wrong, and a \
             concrete real-world example. Be precise and concise."
                .to_string()
        }
        FollowupMode::Followup => {
            "Act as a senior big-tech interviewer. Pose ONE progressively harder follow-up \
             question on the same concept (the kind asked after a candidate answers correctly), \
             then give a model answer that demonstrates the depth a strong candidate would show."
                .to_string()
        }
        FollowupMode::Custom => {
            let user = user_query.unwrap_or("").trim();
            format!(
                "Act as a staff-level engineering mentor. Answer the learner's question about this \
                 topic clearly and accurately, grounded in the concept above.\n\nLearner's question: {user}"
            )
        }
    };
    format!("{instruction}\n\nContext:\n{context}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q() -> Question {
        Question {
            id: 1,
            topic_id: 1,
            prompt: "What does CAP describe?".into(),
            choices: vec!["a".into(), "b".into(), "c".into(), "d".into()],
            correct_index: 2,
            explanation: "because".into(),
            difficulty: 3,
        }
    }

    #[test]
    fn explain_includes_context_and_marks_correct() {
        let p = build_followup_prompt(&q(), FollowupMode::Explain, None);
        assert!(p.contains("mentor"));
        assert!(p.contains("C. c (correct)"));
        assert!(p.contains("What does CAP describe?"));
    }

    #[test]
    fn followup_is_interviewer_framed() {
        let p = build_followup_prompt(&q(), FollowupMode::Followup, None);
        assert!(p.to_lowercase().contains("interviewer"));
        assert!(p.contains("follow-up"));
    }

    #[test]
    fn custom_embeds_user_query() {
        let p = build_followup_prompt(&q(), FollowupMode::Custom, Some("How does Raft differ?"));
        assert!(p.contains("How does Raft differ?"));
    }
}
