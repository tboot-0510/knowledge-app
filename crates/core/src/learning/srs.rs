//! Spaced repetition scheduling (SM-2 algorithm). Pure and unit-tested; the
//! `reviews` table persistence lives in `db`.

use crate::models::ReviewState;

/// Map a multiple-choice outcome to an SM-2 quality score (0..=5).
///
/// We don't capture response latency, so we use a simple mapping: a correct
/// answer is a confident recall (5), a wrong answer is a lapse (2).
pub fn quality_from_correct(is_correct: bool) -> u8 {
    if is_correct {
        5
    } else {
        2
    }
}

/// Apply one SM-2 review with the given quality (0..=5) to `prev`, returning the
/// updated scheduling state. The caller turns `interval_days` into a due date.
pub fn sm2(prev: ReviewState, quality: u8) -> ReviewState {
    let q = quality.min(5) as f32;

    // Update the ease factor (clamped to a 1.3 floor).
    let mut ease = prev.ease_factor + (0.1 - (5.0 - q) * (0.08 + (5.0 - q) * 0.02));
    if ease < 1.3 {
        ease = 1.3;
    }

    // Quality < 3 is a lapse: restart the repetition count and review tomorrow.
    if quality < 3 {
        return ReviewState {
            ease_factor: ease,
            interval_days: 1,
            repetitions: 0,
        };
    }

    let repetitions = prev.repetitions + 1;
    let interval_days = match prev.repetitions {
        0 => 1,
        1 => 6,
        _ => ((prev.interval_days as f32) * ease).round() as u32,
    };

    ReviewState {
        ease_factor: ease,
        interval_days: interval_days.max(1),
        repetitions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_correct_review_is_due_in_one_day() {
        let s = sm2(ReviewState::default(), 5);
        assert_eq!(s.interval_days, 1);
        assert_eq!(s.repetitions, 1);
        assert!(s.ease_factor >= 2.5);
    }

    #[test]
    fn second_correct_review_jumps_to_six_days() {
        let mut s = sm2(ReviewState::default(), 5); // reps 1, interval 1
        s = sm2(s, 5); // reps 2, interval 6
        assert_eq!(s.interval_days, 6);
        assert_eq!(s.repetitions, 2);
    }

    #[test]
    fn third_review_scales_by_ease() {
        let mut s = sm2(ReviewState::default(), 5);
        s = sm2(s, 5); // interval 6
        let before = s.interval_days as f32;
        s = sm2(s, 5); // interval = round(6 * updated_ease)
        // The interval is computed with the ease updated in this same call.
        assert_eq!(s.interval_days, (before * s.ease_factor).round() as u32);
        assert!(s.interval_days > 6);
    }

    #[test]
    fn lapse_resets_repetitions_and_interval() {
        let mut s = sm2(ReviewState::default(), 5);
        s = sm2(s, 5); // interval 6, reps 2
        s = sm2(s, 2); // wrong → lapse
        assert_eq!(s.repetitions, 0);
        assert_eq!(s.interval_days, 1);
    }

    #[test]
    fn ease_never_drops_below_floor() {
        let mut s = ReviewState::default();
        for _ in 0..10 {
            s = sm2(s, 0);
        }
        assert!(s.ease_factor >= 1.3);
    }

    #[test]
    fn quality_mapping() {
        assert_eq!(quality_from_correct(true), 5);
        assert_eq!(quality_from_correct(false), 2);
    }
}
