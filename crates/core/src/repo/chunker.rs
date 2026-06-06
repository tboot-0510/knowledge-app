//! Code-aware-ish chunking: split files into overlapping line windows while
//! tracking 1-based line ranges for citations. Pure and fully unit-tested.

/// A chunk of a file with its 1-based inclusive line range.
#[derive(Debug, Clone, PartialEq)]
pub struct Chunk {
    pub start_line: u32,
    pub end_line: u32,
    pub content: String,
}

/// File extensions we index (source + common text/docs). Anything else is skipped.
const INDEXABLE_EXTS: &[&str] = &[
    "rs", "ts", "tsx", "js", "jsx", "py", "go", "java", "kt", "kts", "scala", "rb", "php", "c", "h",
    "cc", "cpp", "hpp", "cs", "swift", "m", "mm", "sh", "bash", "zsh", "sql", "proto", "graphql",
    "html", "css", "scss", "vue", "svelte", "lua", "r", "jl", "ex", "exs", "erl", "clj", "hs",
    "ml", "dart", "zig", "nim", "toml", "yaml", "yml", "json", "md", "mdx", "txt", "cfg", "ini",
    "dockerfile", "makefile", "gradle", "tf",
];

/// Directory names that are never indexed.
const SKIP_DIRS: &[&str] = &[
    ".git", "node_modules", "target", "dist", "build", "vendor", ".next", ".venv", "venv",
    "__pycache__", ".idea", ".vscode", "coverage", "out", ".turbo", ".cargo",
];

/// Max bytes for a single file to be indexed (skip huge generated/minified files).
pub const MAX_FILE_BYTES: u64 = 512 * 1024;

/// Whether a relative path should be indexed, based on its extension/name and
/// that none of its directory components are in the skip list.
pub fn is_indexable(rel_path: &str) -> bool {
    let lower = rel_path.to_lowercase();
    let parts: Vec<&str> = lower.split(['/', '\\']).collect();
    if parts.iter().any(|p| SKIP_DIRS.contains(p)) {
        return false;
    }
    let name = parts.last().copied().unwrap_or("");
    // lockfiles & minified bundles are noise
    if name.ends_with(".min.js") || name.ends_with(".min.css") {
        return false;
    }
    if matches!(name, "package-lock.json" | "yarn.lock" | "pnpm-lock.yaml" | "cargo.lock") {
        return false;
    }
    // match by special filename or extension
    if matches!(name, "dockerfile" | "makefile") {
        return true;
    }
    match name.rsplit_once('.') {
        Some((_, ext)) => INDEXABLE_EXTS.contains(&ext),
        None => false,
    }
}

/// Estimate token count (~4 chars/token heuristic).
pub fn estimate_tokens(text: &str) -> u32 {
    ((text.len() as f32) / 4.0).ceil() as u32
}

/// Split `content` into overlapping windows of `window` lines with `overlap`
/// lines of context carried between consecutive chunks.
///
/// `window` must be > 0; `overlap` is clamped to `window - 1` to guarantee
/// forward progress. Trailing all-whitespace chunks are dropped.
pub fn chunk_text(content: &str, window: usize, overlap: usize) -> Vec<Chunk> {
    let window = window.max(1);
    let overlap = overlap.min(window - 1);
    let step = window - overlap;
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return Vec::new();
    }
    let mut chunks = Vec::new();
    let mut start = 0usize;
    while start < lines.len() {
        let end = (start + window).min(lines.len());
        let text = lines[start..end].join("\n");
        if !text.trim().is_empty() {
            chunks.push(Chunk {
                start_line: (start + 1) as u32, // 1-based
                end_line: end as u32,
                content: text,
            });
        }
        if end == lines.len() {
            break;
        }
        start += step;
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexable_filters_by_extension_and_dir() {
        assert!(is_indexable("src/main.rs"));
        assert!(is_indexable("README.md"));
        assert!(is_indexable("Dockerfile"));
        assert!(!is_indexable("node_modules/react/index.js"));
        assert!(!is_indexable("target/debug/foo.rs"));
        assert!(!is_indexable("assets/logo.png"));
        assert!(!is_indexable("pnpm-lock.yaml"));
        assert!(!is_indexable("dist/app.min.js"));
    }

    #[test]
    fn chunks_cover_all_lines_with_overlap() {
        let content = (1..=10)
            .map(|i| format!("line{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let chunks = chunk_text(&content, 4, 1);
        // step = 3 → starts at 1,4,7,10
        assert_eq!(chunks[0].start_line, 1);
        assert_eq!(chunks[0].end_line, 4);
        assert_eq!(chunks[1].start_line, 4); // overlap of 1 line
        // last chunk reaches the final line
        assert_eq!(chunks.last().unwrap().end_line, 10);
    }

    #[test]
    fn small_file_is_one_chunk() {
        let chunks = chunk_text("a\nb\nc", 60, 10);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].start_line, 1);
        assert_eq!(chunks[0].end_line, 3);
    }

    #[test]
    fn empty_and_whitespace_yield_no_chunks() {
        assert!(chunk_text("", 10, 2).is_empty());
        assert!(chunk_text("   \n  \n", 10, 2).is_empty());
    }

    #[test]
    fn overlap_clamped_to_make_progress() {
        // overlap >= window would otherwise loop forever
        let content = (1..=5).map(|i| i.to_string()).collect::<Vec<_>>().join("\n");
        let chunks = chunk_text(&content, 2, 5);
        assert!(chunks.last().unwrap().end_line == 5);
    }
}
