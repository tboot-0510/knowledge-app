//! GitHub repo RAG: clone, chunk, embed, and retrieve.
//!
//! "Linking a repo in memory" = clone it locally, chunk indexable files, embed
//! each chunk with the local embedding model, and store vectors in SQLite. Q&A
//! then retrieves the most similar chunks and asks the local chat model to
//! answer using only that context, with file/line citations.

pub mod chunker;
pub mod clone;
pub mod index;
pub mod search;

pub use chunker::{chunk_text, is_indexable, Chunk};
pub use clone::{clone_repo, repo_name_from_url};
pub use search::{build_rag_prompt, cosine_similarity, top_k};
