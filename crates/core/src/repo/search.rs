//! Retrieval: cosine similarity, top-k selection, and RAG prompt assembly.
//! Pure functions, fully unit-tested (the embedding/DB I/O lives elsewhere).

use crate::models::RetrievedChunk;

/// Cosine similarity of two equal-length vectors. Returns 0.0 for mismatched
/// lengths or zero-magnitude vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

/// Rank `candidates` (id, vector) by cosine similarity to `query` and return the
/// top `k` as (id, score), highest first.
pub fn top_k(query: &[f32], candidates: &[(i64, Vec<f32>)], k: usize) -> Vec<(i64, f32)> {
    let mut scored: Vec<(i64, f32)> = candidates
        .iter()
        .map(|(id, v)| (*id, cosine_similarity(query, v)))
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(k);
    scored
}

/// Assemble the RAG prompt from retrieved chunks and the user's question.
/// Instructs the model to answer only from context and cite `file:line` ranges.
pub fn build_rag_prompt(question: &str, chunks: &[RetrievedChunk]) -> String {
    let mut ctx = String::new();
    for (i, c) in chunks.iter().enumerate() {
        ctx.push_str(&format!(
            "[{idx}] {path}:{start}-{end}\n```\n{content}\n```\n\n",
            idx = i + 1,
            path = c.file_path,
            start = c.start_line,
            end = c.end_line,
            content = c.content,
        ));
    }
    format!(
        r#"You are a senior engineer answering questions about a specific codebase. Use ONLY the context below. If the answer is not in the context, say "I couldn't find that in the indexed repository." Always cite the relevant files as `path:line-line`.

Context:
{ctx}
Question: {question}

Answer:"#,
        ctx = ctx,
        question = question,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_vectors_have_similarity_one() {
        let v = vec![0.1, 0.2, 0.3];
        assert!((cosine_similarity(&v, &v) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn orthogonal_vectors_have_zero_similarity() {
        assert_eq!(cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]), 0.0);
    }

    #[test]
    fn mismatched_lengths_are_safe() {
        assert_eq!(cosine_similarity(&[1.0, 0.0], &[1.0]), 0.0);
    }

    #[test]
    fn top_k_orders_by_similarity_and_truncates() {
        let query = vec![1.0, 0.0];
        let candidates = vec![
            (10, vec![0.0, 1.0]),  // orthogonal → 0
            (20, vec![1.0, 0.0]),  // identical → 1
            (30, vec![1.0, 1.0]),  // 45° → ~0.707
        ];
        let res = top_k(&query, &candidates, 2);
        assert_eq!(res.len(), 2);
        assert_eq!(res[0].0, 20);
        assert_eq!(res[1].0, 30);
    }

    #[test]
    fn prompt_includes_citations_and_question() {
        let chunks = vec![RetrievedChunk {
            file_path: "src/main.rs".into(),
            start_line: 1,
            end_line: 5,
            content: "fn main() {}".into(),
            score: 0.9,
        }];
        let p = build_rag_prompt("what does main do?", &chunks);
        assert!(p.contains("src/main.rs:1-5"));
        assert!(p.contains("what does main do?"));
        assert!(p.contains("ONLY the context"));
    }
}
