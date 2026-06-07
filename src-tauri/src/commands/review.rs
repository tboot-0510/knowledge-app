//! Spaced-repetition review commands.

use crate::commands::daily::schedule_review;
use crate::state::AppState;
use knowledge_core::models::{AttemptResult, ReviewItem};
use tauri::State;

/// Questions due for review today (most overdue first).
#[tauri::command]
pub fn get_due_reviews(state: State<AppState>) -> Result<Vec<ReviewItem>, String> {
    let db = state.db.lock().unwrap();
    let today = knowledge_core::scheduler::today_local();
    db.due_reviews(&today, 20).map_err(|e| e.to_string())
}

/// Count of questions currently due (for a badge).
#[tauri::command]
pub fn count_due_reviews(state: State<AppState>) -> Result<u32, String> {
    let db = state.db.lock().unwrap();
    let today = knowledge_core::scheduler::today_local();
    db.count_due_reviews(&today).map_err(|e| e.to_string())
}

/// Answer a review question: grade it and reschedule via SM-2.
#[tauri::command]
pub fn submit_review(
    state: State<AppState>,
    question_id: i64,
    chosen_index: usize,
) -> Result<AttemptResult, String> {
    let db = state.db.lock().unwrap();
    let q = db.get_question(question_id).map_err(|e| e.to_string())?;
    let is_correct = chosen_index == q.correct_index;
    schedule_review(&db, question_id, is_correct)?;
    Ok(AttemptResult {
        question_id,
        chosen_index,
        correct_index: q.correct_index,
        is_correct,
        explanation: q.explanation,
        session_completed: false,
    })
}
