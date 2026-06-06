//! Pure scoring logic: streak transitions and adaptive difficulty.

use crate::models::{Level, Streak};
use chrono::{Duration, NaiveDate};

/// Update the streak given that the user completed a session on `today`
/// (local date "YYYY-MM-DD").
///
/// Rules:
/// - same day as last completion → unchanged (idempotent).
/// - exactly the next day → `current_streak += 1`.
/// - any larger gap (or first ever) → `current_streak = 1`.
///
/// `longest_streak` is bumped to track the max; `last_active_date` becomes today.
pub fn update_streak(mut streak: Streak, today: &str) -> Streak {
    let today_date = NaiveDate::parse_from_str(today, "%Y-%m-%d").ok();
    match (&streak.last_active_date, today_date) {
        (Some(last), Some(_td)) if last == today => {
            // already counted today; no change
        }
        (Some(last), Some(td)) => {
            let consecutive = NaiveDate::parse_from_str(last, "%Y-%m-%d")
                .map(|ld| td - ld == Duration::days(1))
                .unwrap_or(false);
            streak.current_streak = if consecutive { streak.current_streak + 1 } else { 1 };
            streak.last_active_date = Some(today.to_string());
        }
        _ => {
            streak.current_streak = 1;
            streak.last_active_date = Some(today.to_string());
        }
    }
    if streak.current_streak > streak.longest_streak {
        streak.longest_streak = streak.current_streak;
    }
    streak
}

/// Choose the target difficulty (1..=5) for the next session given the user's
/// seniority baseline and their recent accuracy (0.0..=1.0).
///
/// High accuracy nudges difficulty up; low accuracy nudges it down, always
/// clamped to 1..=5 and never below the seniority baseline minus one.
pub fn next_difficulty(level: Level, recent_accuracy: f32) -> u8 {
    let baseline = level.baseline_difficulty() as i32;
    let delta = if recent_accuracy >= 0.8 {
        1
    } else if recent_accuracy < 0.5 {
        -1
    } else {
        0
    };
    let floor = (baseline - 1).max(1);
    (baseline + delta).clamp(floor, 5) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn streak(current: u32, longest: u32, last: Option<&str>) -> Streak {
        Streak {
            current_streak: current,
            longest_streak: longest,
            last_active_date: last.map(String::from),
        }
    }

    #[test]
    fn first_completion_starts_streak() {
        let s = update_streak(Streak::default(), "2026-06-06");
        assert_eq!(s.current_streak, 1);
        assert_eq!(s.longest_streak, 1);
        assert_eq!(s.last_active_date.as_deref(), Some("2026-06-06"));
    }

    #[test]
    fn consecutive_day_increments() {
        let s = update_streak(streak(3, 5, Some("2026-06-05")), "2026-06-06");
        assert_eq!(s.current_streak, 4);
        assert_eq!(s.longest_streak, 5);
    }

    #[test]
    fn new_longest_is_tracked() {
        let s = update_streak(streak(5, 5, Some("2026-06-05")), "2026-06-06");
        assert_eq!(s.current_streak, 6);
        assert_eq!(s.longest_streak, 6);
    }

    #[test]
    fn gap_resets_streak() {
        let s = update_streak(streak(7, 9, Some("2026-06-03")), "2026-06-06");
        assert_eq!(s.current_streak, 1);
        assert_eq!(s.longest_streak, 9);
    }

    #[test]
    fn same_day_is_idempotent() {
        let s = update_streak(streak(4, 4, Some("2026-06-06")), "2026-06-06");
        assert_eq!(s.current_streak, 4);
    }

    #[test]
    fn difficulty_adapts_within_bounds() {
        assert_eq!(next_difficulty(Level::Senior, 0.9), 3); // baseline 2 + 1
        assert_eq!(next_difficulty(Level::Senior, 0.6), 2); // baseline
        assert_eq!(next_difficulty(Level::Senior, 0.2), 1); // floor
        assert_eq!(next_difficulty(Level::Principal, 1.0), 5); // clamp top
    }
}
