//! Elo-based adaptive recommendation.
//!
//! Each learner has a skill rating (per topic, plus a global rating) and items
//! have a difficulty rating. After every answer both are nudged by the standard
//! Elo update, so questions automatically harden after success and ease after a
//! miss — without the large-sample pre-calibration that full IRT needs.
//!
//! Targeting aims for "desirable difficulty": items slightly below the learner's
//! skill (~70% success), climbing as a win streak grows. A separate Bloom level
//! pushes for *deeper* questions on streaks. All functions here are pure and
//! unit-tested; persistence lives in `db` and orchestration in `commands`.

use crate::models::{Difficulty, Level};

/// Standard Elo K-factor (step size of each update).
pub const K_FACTOR: f64 = 32.0;

/// Probability the learner answers correctly given `skill` vs item `difficulty`.
pub fn expected_score(skill: f64, difficulty: f64) -> f64 {
    1.0 / (1.0 + 10f64.powf((difficulty - skill) / 400.0))
}

/// Apply one Elo update. Returns `(new_skill, new_difficulty)`: a correct answer
/// raises the learner's skill and lowers the item's difficulty (and vice versa).
pub fn update_ratings(skill: f64, difficulty: f64, correct: bool, k: f64) -> (f64, f64) {
    let expected = expected_score(skill, difficulty);
    let actual = if correct { 1.0 } else { 0.0 };
    let delta = k * (actual - expected);
    (skill + delta, difficulty - delta)
}

/// Cold-start skill prior from the chosen seniority level.
pub fn level_prior(level: Level) -> f64 {
    match level {
        Level::Senior => 1300.0,
        Level::Staff => 1450.0,
        Level::Principal => 1600.0,
    }
}

/// Map a rating to a difficulty tier (band thresholds).
pub fn rating_to_tier(rating: f64) -> Difficulty {
    if rating < 1250.0 {
        Difficulty::Easy
    } else if rating < 1450.0 {
        Difficulty::Medium
    } else if rating < 1650.0 {
        Difficulty::Hard
    } else {
        Difficulty::Advanced
    }
}

/// The center rating of a tier's band (used to seed cold-start difficulty from a
/// user's chosen starting tier).
pub fn tier_to_rating(d: Difficulty) -> f64 {
    match d {
        Difficulty::Easy => 1150.0,
        Difficulty::Medium => 1350.0,
        Difficulty::Hard => 1550.0,
        Difficulty::Advanced => 1750.0,
    }
}

/// Target difficulty tier for the next item: slightly below current skill (so
/// success ~70%), climbing as the win `streak` grows.
pub fn target_difficulty(skill: f64, streak: u32) -> Difficulty {
    let target = skill - 120.0 + (streak as f64) * 70.0;
    rating_to_tier(target)
}

/// Bloom's-taxonomy cognitive depth, ordered shallow → deep.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BloomLevel {
    Remember,
    Understand,
    Apply,
    Analyze,
    Evaluate,
}

const BLOOM_ORDER: [BloomLevel; 5] = [
    BloomLevel::Remember,
    BloomLevel::Understand,
    BloomLevel::Apply,
    BloomLevel::Analyze,
    BloomLevel::Evaluate,
];

/// Choose a Bloom level from seniority (base depth) and the win streak (climb),
/// capped at Evaluate.
pub fn bloom_for(level: Level, streak: u32) -> BloomLevel {
    let base = match level {
        Level::Senior => 2,    // Apply
        Level::Staff => 3,     // Analyze
        Level::Principal => 3, // Analyze
    };
    let idx = (base + streak.min(2) as usize).min(BLOOM_ORDER.len() - 1);
    BLOOM_ORDER[idx]
}

/// A directive for the generator describing the desired cognitive depth.
pub fn bloom_directive(b: BloomLevel) -> &'static str {
    match b {
        BloomLevel::Remember => "test recall of key facts and definitions",
        BloomLevel::Understand => "test conceptual understanding and explanation",
        BloomLevel::Apply => "require applying the concept to a concrete scenario",
        BloomLevel::Analyze => "require analyzing trade-offs, failure modes, or comparing approaches",
        BloomLevel::Evaluate => {
            "require evaluating/critiquing a design decision and justifying the best choice"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expected_score_is_half_when_equal() {
        assert!((expected_score(1400.0, 1400.0) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn higher_skill_means_higher_expected() {
        assert!(expected_score(1600.0, 1400.0) > 0.7);
        assert!(expected_score(1200.0, 1400.0) < 0.3);
    }

    #[test]
    fn correct_answer_raises_skill_lowers_difficulty() {
        let (s, d) = update_ratings(1400.0, 1400.0, true, K_FACTOR);
        assert!(s > 1400.0);
        assert!(d < 1400.0);
        // symmetric at expected 0.5: +/- K/2
        assert!((s - 1416.0).abs() < 1e-6);
        assert!((d - 1384.0).abs() < 1e-6);
    }

    #[test]
    fn wrong_answer_lowers_skill() {
        let (s, _) = update_ratings(1400.0, 1400.0, false, K_FACTOR);
        assert!(s < 1400.0);
    }

    #[test]
    fn target_difficulty_climbs_with_streak() {
        let t0 = target_difficulty(1400.0, 0);
        let t3 = target_difficulty(1400.0, 3);
        // ordering: a longer streak yields a not-easier tier
        assert!(tier_to_rating(t3) >= tier_to_rating(t0));
        assert_eq!(target_difficulty(1700.0, 3), Difficulty::Advanced);
    }

    #[test]
    fn rating_tier_round_trips_into_band() {
        assert_eq!(rating_to_tier(tier_to_rating(Difficulty::Easy)), Difficulty::Easy);
        assert_eq!(rating_to_tier(tier_to_rating(Difficulty::Medium)), Difficulty::Medium);
        assert_eq!(rating_to_tier(tier_to_rating(Difficulty::Hard)), Difficulty::Hard);
        assert_eq!(rating_to_tier(tier_to_rating(Difficulty::Advanced)), Difficulty::Advanced);
    }

    #[test]
    fn bloom_climbs_and_caps() {
        assert_eq!(bloom_for(Level::Senior, 0), BloomLevel::Apply);
        assert_eq!(bloom_for(Level::Senior, 1), BloomLevel::Analyze);
        assert_eq!(bloom_for(Level::Principal, 5), BloomLevel::Evaluate); // capped
        assert!(!bloom_directive(BloomLevel::Evaluate).is_empty());
    }
}
