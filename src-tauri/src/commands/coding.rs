//! LeetCode-style coding practice commands: list categories with progress,
//! generate a problem, and review a submitted solution with the local LLM.

use crate::state::AppState;
use knowledge_core::learning::{
    build_problem_prompt, build_review_prompt, load_categories, parse_problem, parse_review,
    seed_problems,
};
use knowledge_core::models::{CodeReview, CodingProblem, Difficulty, DsCategoryCard};
use knowledge_core::ollama::OllamaClient;
use tauri::State;

/// Data-structure categories merged with the user's coding progress.
#[tauri::command]
pub fn list_ds_categories(state: State<AppState>) -> Result<Vec<DsCategoryCard>, String> {
    let db = state.db.lock().unwrap();
    let categories = load_categories().map_err(|e| e.to_string())?;
    let progress = db.coding_progress().map_err(|e| e.to_string())?;
    Ok(categories
        .into_iter()
        .map(|c| {
            let (attempted, solved) = progress.get(&c.slug).copied().unwrap_or((0, 0));
            DsCategoryCard {
                slug: c.slug,
                title: c.title,
                description: c.description,
                attempted,
                solved,
            }
        })
        .collect())
}

/// Generate a coding problem for a category/difficulty, falling back to the
/// bundled seed bank if the model is unavailable or returns invalid output.
#[tauri::command]
pub async fn generate_coding_problem(
    state: State<'_, AppState>,
    category_slug: String,
    difficulty: Difficulty,
) -> Result<CodingProblem, String> {
    let (settings, category) = {
        let db = state.db.lock().unwrap();
        let settings = db.get_settings().map_err(|e| e.to_string())?;
        let category = load_categories()
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|c| c.slug == category_slug)
            .ok_or_else(|| format!("unknown category: {category_slug}"))?;
        (settings, category)
    };

    let client = OllamaClient::new(&settings.ollama_url);
    let prompt = build_problem_prompt(&category, difficulty);
    if let Ok(raw) = client.generate(&settings.chat_model, &prompt, true).await {
        if let Ok(p) = parse_problem(&category.slug, &raw) {
            return Ok(p);
        }
    }

    // Fallback: a seed problem (prefer the requested difficulty).
    let seeds = seed_problems(&category.slug);
    seeds
        .iter()
        .find(|p| p.difficulty == difficulty)
        .or_else(|| seeds.first())
        .cloned()
        .ok_or_else(|| "No problem available — start Ollama for generated problems.".to_string())
}

/// Review a submitted solution: analyze correctness + complexity, persist the
/// attempt, and return the grade.
#[tauri::command]
pub async fn review_solution(
    state: State<'_, AppState>,
    problem: CodingProblem,
    code: String,
    language: String,
) -> Result<CodeReview, String> {
    if code.trim().is_empty() {
        return Err("Write a solution before submitting.".into());
    }
    let settings = {
        let db = state.db.lock().unwrap();
        db.get_settings().map_err(|e| e.to_string())?
    };

    let client = OllamaClient::new(&settings.ollama_url);
    let prompt = build_review_prompt(&problem, &code, &language);
    let raw = client
        .generate(&settings.chat_model, &prompt, true)
        .await
        .map_err(|e| e.to_string())?;
    let review = parse_review(&raw, 10).map_err(|e| e.to_string())?;

    {
        let db = state.db.lock().unwrap();
        db.insert_coding_attempt(
            &problem.category_slug,
            &problem.title,
            &language,
            &code,
            &review.verdict,
            review.score,
            review.max_score,
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(review)
}
