//! Weakness report: turn the user's progress stats into a prompt that asks the
//! local LLM for a diagnosis of weak areas and a concrete study plan.

use crate::models::ProgressStats;

/// Per-topic accuracy line for the report (title, answered, correct).
pub type TopicScore = (String, u32, u32);

/// Build the weakness-report prompt from aggregate + per-topic stats.
pub fn build_weakness_prompt(stats: &ProgressStats, topics: &[TopicScore]) -> String {
    let mut by_area = String::new();
    for a in &stats.by_area {
        let pct = if a.total > 0 { a.correct * 100 / a.total } else { 0 };
        by_area.push_str(&format!(
            "- {}: {}/{} ({}%)\n",
            a.area, a.correct, a.total, pct
        ));
    }
    if by_area.is_empty() {
        by_area.push_str("- (no answers yet)\n");
    }

    let mut by_topic = String::new();
    // Show the weakest topics first (lowest accuracy among those attempted).
    let mut scored: Vec<&TopicScore> = topics.iter().filter(|(_, a, _)| *a > 0).collect();
    scored.sort_by_key(|(_, answered, correct)| {
        // ascending accuracy * 100
        if *answered > 0 { correct * 100 / answered } else { 0 }
    });
    for (title, answered, correct) in scored.iter().take(12) {
        let pct = if *answered > 0 { correct * 100 / answered } else { 0 };
        by_topic.push_str(&format!("- {title}: {correct}/{answered} ({pct}%)\n"));
    }
    if by_topic.is_empty() {
        by_topic.push_str("- (no topics attempted yet)\n");
    }

    let overall = if stats.total_questions > 0 {
        stats.total_correct * 100 / stats.total_questions
    } else {
        0
    };

    format!(
        r#"You are a staff-level engineering mentor reviewing a learner's quiz performance. Be direct and specific.

Overall: {correct}/{total} correct ({overall}%), {days} days completed, current streak {streak}.

Accuracy by area:
{by_area}
Weakest attempted topics:
{by_topic}
Write a concise report in markdown with exactly these sections:
1. **Strengths** — what they seem solid on.
2. **Focus areas** — the 2-3 weakest areas/topics and *why* they likely matter for a senior/staff engineer.
3. **One-week study plan** — a concrete day-by-day plan (Mon-Sun) referencing the weak topics above.
Keep it practical and motivating."#,
        correct = stats.total_correct,
        total = stats.total_questions,
        overall = overall,
        days = stats.days_completed,
        streak = stats.streak.current_streak,
        by_area = by_area,
        by_topic = by_topic,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AreaStat, Streak};

    #[test]
    fn prompt_includes_stats_and_sections() {
        let stats = ProgressStats {
            total_questions: 20,
            total_correct: 12,
            days_completed: 5,
            streak: Streak {
                current_streak: 3,
                longest_streak: 5,
                last_active_date: None,
            },
            by_area: vec![AreaStat {
                area: "concurrency".into(),
                correct: 2,
                total: 8,
            }],
        };
        let topics = vec![
            ("Async Runtimes".to_string(), 8u32, 2u32),
            ("Caching".to_string(), 4, 4),
        ];
        let p = build_weakness_prompt(&stats, &topics);
        assert!(p.contains("12/20"));
        assert!(p.contains("concurrency"));
        assert!(p.contains("study plan") || p.contains("study plan".to_uppercase().as_str()));
        // weakest topic (Async Runtimes 25%) should appear before Caching (100%)
        let async_idx = p.find("Async Runtimes").unwrap();
        let cache_idx = p.find("Caching").unwrap();
        assert!(async_idx < cache_idx);
    }
}
