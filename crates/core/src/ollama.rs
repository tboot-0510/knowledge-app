//! Local-LLM client for a running Ollama server (default `127.0.0.1:11434`).
//!
//! Only local inference — no cloud endpoints. Provides blocking-style async
//! helpers for generation and embeddings, plus a streaming generator used by the
//! repo Q&A view. Pure request/response shaping; networking is via `reqwest`.

use crate::error::{Error, Result};
use crate::models::ModelInfo;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Thin async client around the Ollama HTTP API.
#[derive(Clone)]
pub struct OllamaClient {
    base_url: String,
    http: reqwest::Client,
}

#[derive(Deserialize)]
struct GenerateResponse {
    response: String,
}

#[derive(Deserialize)]
struct EmbeddingsResponse {
    embedding: Vec<f32>,
}

#[derive(Deserialize)]
struct TagsResponse {
    models: Vec<TagModel>,
}

#[derive(Deserialize)]
struct TagModel {
    name: String,
    #[serde(default)]
    size: u64,
}

/// One streamed token (or terminal signal) from a generation call.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum StreamChunk {
    Token { text: String },
    Done,
    Error { message: String },
}

/// Progress of an `ollama pull`, streamed to the model-manager UI.
#[derive(Debug, Clone, Serialize)]
pub struct PullProgress {
    pub status: String,
    pub total: u64,
    pub completed: u64,
    pub done: bool,
}

impl OllamaClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        OllamaClient {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            http: reqwest::Client::new(),
        }
    }

    /// Verify the server is reachable; maps connection errors to a friendly message.
    pub async fn health(&self) -> Result<()> {
        self.http
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await
            .map_err(|_| Error::OllamaUnreachable(self.base_url.clone()))?
            .error_for_status()?;
        Ok(())
    }

    /// List locally available model names (`/api/tags`).
    pub async fn list_models(&self) -> Result<Vec<String>> {
        let resp = self
            .http
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await
            .map_err(|_| Error::OllamaUnreachable(self.base_url.clone()))?
            .error_for_status()?
            .json::<TagsResponse>()
            .await?;
        Ok(resp.models.into_iter().map(|m| m.name).collect())
    }

    /// List locally available models with their on-disk sizes.
    pub async fn list_models_detailed(&self) -> Result<Vec<ModelInfo>> {
        let resp = self
            .http
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await
            .map_err(|_| Error::OllamaUnreachable(self.base_url.clone()))?
            .error_for_status()?
            .json::<TagsResponse>()
            .await?;
        Ok(resp
            .models
            .into_iter()
            .map(|m| ModelInfo {
                name: m.name,
                size_bytes: m.size,
            })
            .collect())
    }

    /// Pull a model from the Ollama registry, streaming progress to `on_progress`.
    pub async fn pull_model<F>(&self, name: &str, mut on_progress: F) -> Result<()>
    where
        F: FnMut(PullProgress),
    {
        let resp = self
            .http
            .post(format!("{}/api/pull", self.base_url))
            .json(&json!({ "model": name, "stream": true }))
            .send()
            .await
            .map_err(|_| Error::OllamaUnreachable(self.base_url.clone()))?
            .error_for_status()?;

        let mut stream = resp.bytes_stream();
        let mut buf = String::new();
        while let Some(item) = stream.next().await {
            let bytes = item?;
            buf.push_str(&String::from_utf8_lossy(&bytes));
            while let Some(nl) = buf.find('\n') {
                let line = buf[..nl].trim().to_string();
                buf.drain(..=nl);
                if line.is_empty() {
                    continue;
                }
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
                    let status = v
                        .get("status")
                        .and_then(|s| s.as_str())
                        .unwrap_or("")
                        .to_string();
                    let total = v.get("total").and_then(|t| t.as_u64()).unwrap_or(0);
                    let completed = v.get("completed").and_then(|c| c.as_u64()).unwrap_or(0);
                    let done = status == "success";
                    on_progress(PullProgress {
                        status,
                        total,
                        completed,
                        done,
                    });
                }
            }
        }
        on_progress(PullProgress {
            status: "success".into(),
            total: 0,
            completed: 0,
            done: true,
        });
        Ok(())
    }

    /// Non-streaming generation. `format_json` requests strict JSON output
    /// (used for MCQ generation).
    pub async fn generate(&self, model: &str, prompt: &str, format_json: bool) -> Result<String> {
        let mut body = json!({
            "model": model,
            "prompt": prompt,
            "stream": false,
        });
        if format_json {
            body["format"] = json!("json");
        }
        let resp = self
            .http
            .post(format!("{}/api/generate", self.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|_| Error::OllamaUnreachable(self.base_url.clone()))?
            .error_for_status()?
            .json::<GenerateResponse>()
            .await?;
        Ok(resp.response)
    }

    /// Delete a locally installed model.
    pub async fn delete_model(&self, name: &str) -> Result<()> {
        self.http
            .delete(format!("{}/api/delete", self.base_url))
            .json(&json!({ "model": name }))
            .send()
            .await
            .map_err(|_| Error::OllamaUnreachable(self.base_url.clone()))?
            .error_for_status()?;
        Ok(())
    }

    /// Embed a single text via `/api/embeddings`.
    pub async fn embed(&self, model: &str, text: &str) -> Result<Vec<f32>> {
        let resp = self
            .http
            .post(format!("{}/api/embeddings", self.base_url))
            .json(&json!({ "model": model, "prompt": text }))
            .send()
            .await
            .map_err(|_| Error::OllamaUnreachable(self.base_url.clone()))?
            .error_for_status()?
            .json::<EmbeddingsResponse>()
            .await?;
        Ok(resp.embedding)
    }

    /// Stream a generation, invoking `on_chunk` for each token and at completion.
    /// Used by the repo Q&A view to render tokens as they arrive.
    pub async fn generate_stream<F>(&self, model: &str, prompt: &str, mut on_chunk: F) -> Result<()>
    where
        F: FnMut(StreamChunk),
    {
        let resp = self
            .http
            .post(format!("{}/api/generate", self.base_url))
            .json(&json!({ "model": model, "prompt": prompt, "stream": true }))
            .send()
            .await
            .map_err(|_| Error::OllamaUnreachable(self.base_url.clone()))?
            .error_for_status()?;

        let mut stream = resp.bytes_stream();
        let mut buf = String::new();
        while let Some(item) = stream.next().await {
            let bytes = item?;
            buf.push_str(&String::from_utf8_lossy(&bytes));
            // Ollama streams newline-delimited JSON objects.
            while let Some(nl) = buf.find('\n') {
                let line = buf[..nl].trim().to_string();
                buf.drain(..=nl);
                if line.is_empty() {
                    continue;
                }
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
                    if let Some(tok) = v.get("response").and_then(|r| r.as_str()) {
                        if !tok.is_empty() {
                            on_chunk(StreamChunk::Token { text: tok.to_string() });
                        }
                    }
                    if v.get("done").and_then(|d| d.as_bool()).unwrap_or(false) {
                        on_chunk(StreamChunk::Done);
                        return Ok(());
                    }
                }
            }
        }
        on_chunk(StreamChunk::Done);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_url_trailing_slash_trimmed() {
        let c = OllamaClient::new("http://127.0.0.1:11434/");
        assert_eq!(c.base_url, "http://127.0.0.1:11434");
    }

    #[test]
    fn stream_chunk_serializes_with_tag() {
        let j = serde_json::to_string(&StreamChunk::Token { text: "hi".into() }).unwrap();
        assert!(j.contains("\"kind\":\"token\""));
        assert!(j.contains("\"text\":\"hi\""));
        let d = serde_json::to_string(&StreamChunk::Done).unwrap();
        assert!(d.contains("\"kind\":\"done\""));
    }
}
