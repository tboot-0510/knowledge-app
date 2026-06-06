//! Indexing orchestration: walk a cloned repo, chunk indexable files, and embed
//! each chunk with the local embedding model.
//!
//! The async embedding step is deliberately separated from the (synchronous)
//! SQLite writes so the returned future stays `Send` — the DB connection is not
//! `Sync`, so persistence is done by the caller after awaiting this.

use crate::error::Result;
use crate::ollama::OllamaClient;
use crate::repo::chunker::{self, MAX_FILE_BYTES};
use ignore::WalkBuilder;
use std::path::Path;

/// A chunk plus its computed embedding, ready to persist.
pub struct EmbeddedChunk {
    pub file_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub content: String,
    pub token_estimate: u32,
    pub embedding: Vec<f32>,
}

/// Collect indexable files under `repo_path`, honoring `.gitignore` and our
/// extension/size filters. Returns (relative_path, absolute_path) pairs.
pub fn collect_indexable_files(repo_path: &Path) -> Vec<(String, std::path::PathBuf)> {
    let mut out = Vec::new();
    for result in WalkBuilder::new(repo_path).hidden(true).git_ignore(true).build() {
        let Ok(entry) = result else { continue };
        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        let abs = entry.path();
        let Ok(rel) = abs.strip_prefix(repo_path) else { continue };
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        if !chunker::is_indexable(&rel_str) {
            continue;
        }
        if std::fs::metadata(abs).map(|m| m.len() > MAX_FILE_BYTES).unwrap_or(true) {
            continue;
        }
        out.push((rel_str, abs.to_path_buf()));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Chunk + embed every indexable file in the repo. `progress(done, total)` is
/// called after each file's chunks are embedded so the UI can show progress.
///
/// Returns the embedded chunks (persisted by the caller) and the file count.
pub async fn embed_repo<F>(
    ollama: &OllamaClient,
    embed_model: &str,
    repo_path: &Path,
    mut progress: F,
) -> Result<(Vec<EmbeddedChunk>, u32)>
where
    F: FnMut(u32, u32),
{
    let files = collect_indexable_files(repo_path);
    let total = files.len() as u32;
    let mut embedded = Vec::new();
    for (done, (rel_path, abs)) in files.iter().enumerate() {
        let Ok(content) = std::fs::read_to_string(abs) else {
            progress(done as u32 + 1, total);
            continue; // skip non-UTF8/binary
        };
        for chunk in chunker::chunk_text(&content, 60, 10) {
            let vector = ollama.embed(embed_model, &chunk.content).await?;
            embedded.push(EmbeddedChunk {
                file_path: rel_path.clone(),
                start_line: chunk.start_line,
                end_line: chunk.end_line,
                token_estimate: chunker::estimate_tokens(&chunk.content),
                content: chunk.content,
                embedding: vector,
            });
        }
        progress(done as u32 + 1, total);
    }
    Ok((embedded, total))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn collects_only_indexable_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("main.rs"), "fn main() {}").unwrap();
        fs::write(root.join("README.md"), "# hi").unwrap();
        fs::create_dir_all(root.join("node_modules/x")).unwrap();
        fs::write(root.join("node_modules/x/index.js"), "x").unwrap();
        fs::write(root.join("logo.png"), [0u8, 1, 2, 3]).unwrap();

        let files = collect_indexable_files(root);
        let names: Vec<_> = files.iter().map(|(r, _)| r.clone()).collect();
        assert!(names.contains(&"main.rs".to_string()));
        assert!(names.contains(&"README.md".to_string()));
        assert!(!names.iter().any(|n| n.contains("node_modules")));
        assert!(!names.iter().any(|n| n.ends_with(".png")));
    }
}
