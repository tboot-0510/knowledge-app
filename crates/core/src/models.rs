//! Data-transfer objects shared between the Rust core and the React frontend.
//!
//! These derive `Serialize`/`Deserialize` so they can cross the Tauri IPC
//! boundary, and are mirrored by hand in `src/lib/types.ts`.

use serde::{Deserialize, Serialize};

/// Seniority level used to scale topic selection and challenge difficulty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    #[default]
    Senior,
    Staff,
    Principal,
}

impl Level {
    /// Baseline difficulty (1..=5) for this seniority level.
    pub fn baseline_difficulty(self) -> u8 {
        match self {
            Level::Senior => 2,
            Level::Staff => 3,
            Level::Principal => 4,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Level::Senior => "senior",
            Level::Staff => "staff",
            Level::Principal => "principal",
        }
    }

    pub fn from_str_lenient(s: &str) -> Level {
        match s.trim().to_lowercase().as_str() {
            "principal" => Level::Principal,
            "staff" => Level::Staff,
            _ => Level::Senior,
        }
    }
}

/// A difficulty tier for questions and per-topic targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Difficulty {
    Easy,
    #[default]
    Medium,
    Hard,
    Advanced,
}

impl Difficulty {
    pub fn as_str(self) -> &'static str {
        match self {
            Difficulty::Easy => "easy",
            Difficulty::Medium => "medium",
            Difficulty::Hard => "hard",
            Difficulty::Advanced => "advanced",
        }
    }

    pub fn from_str_lenient(s: &str) -> Difficulty {
        match s.trim().to_lowercase().as_str() {
            "easy" => Difficulty::Easy,
            "hard" => Difficulty::Hard,
            "advanced" => Difficulty::Advanced,
            _ => Difficulty::Medium,
        }
    }

    /// Map to the numeric 1..=5 scale stored on questions.
    pub fn to_numeric(self) -> u8 {
        match self {
            Difficulty::Easy => 1,
            Difficulty::Medium => 2,
            Difficulty::Hard => 3,
            Difficulty::Advanced => 5,
        }
    }

    /// Bucket a numeric 1..=5 difficulty into a tier.
    pub fn from_numeric(n: u8) -> Difficulty {
        match n {
            0 | 1 => Difficulty::Easy,
            2 => Difficulty::Medium,
            3 | 4 => Difficulty::Hard,
            _ => Difficulty::Advanced,
        }
    }
}

/// A learning topic (from the curated catalog or LLM-generated).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Topic {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub summary: String,
    /// e.g. "distributed-systems", "concurrency", "system-design".
    pub area: String,
    pub level: Level,
    /// "catalog" | "generated"
    pub source: String,
    #[serde(default)]
    pub talking_points: Vec<String>,
}

/// A single multiple-choice question.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    pub id: i64,
    pub topic_id: i64,
    pub prompt: String,
    pub choices: Vec<String>,
    /// 0-based index into `choices`.
    pub correct_index: usize,
    pub explanation: String,
    /// 1..=5
    pub difficulty: u8,
}

/// The day's session: a topic plus its generated questions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailySession {
    pub id: i64,
    /// Local date, "YYYY-MM-DD".
    pub date: String,
    pub topic: Topic,
    pub questions: Vec<Question>,
    pub completed: bool,
    /// Number of questions answered correctly (None until completed).
    pub score: Option<u32>,
}

/// Result of grading a single submitted answer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttemptResult {
    pub question_id: i64,
    pub chosen_index: usize,
    pub correct_index: usize,
    pub is_correct: bool,
    pub explanation: String,
    /// True once every question in the session has been answered.
    pub session_completed: bool,
}

/// Streak / progress summary.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Streak {
    pub current_streak: u32,
    pub longest_streak: u32,
    pub last_active_date: Option<String>,
}

/// Aggregate progress statistics for the dashboard.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProgressStats {
    pub total_questions: u32,
    pub total_correct: u32,
    pub days_completed: u32,
    pub streak: Streak,
    /// Per-area accuracy, as (area, correct, total).
    pub by_area: Vec<AreaStat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AreaStat {
    pub area: String,
    pub correct: u32,
    pub total: u32,
}

/// Index status of a linked repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RepoStatus {
    Pending,
    Cloning,
    Indexing,
    Ready,
    Error,
}

impl RepoStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            RepoStatus::Pending => "pending",
            RepoStatus::Cloning => "cloning",
            RepoStatus::Indexing => "indexing",
            RepoStatus::Ready => "ready",
            RepoStatus::Error => "error",
        }
    }

    pub fn from_str_lenient(s: &str) -> RepoStatus {
        match s {
            "cloning" => RepoStatus::Cloning,
            "indexing" => RepoStatus::Indexing,
            "ready" => RepoStatus::Ready,
            "error" => RepoStatus::Error,
            _ => RepoStatus::Pending,
        }
    }
}

/// A linked repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repo {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub local_path: String,
    pub status: RepoStatus,
    pub file_count: u32,
    pub chunk_count: u32,
    pub indexed_at: Option<String>,
    /// Set when status == Error.
    pub error: Option<String>,
}

/// A chunk of repo content with its source location, used for RAG citations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoChunk {
    pub id: i64,
    pub repo_id: i64,
    pub file_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub content: String,
}

/// A retrieved chunk plus its similarity score, returned alongside an answer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievedChunk {
    pub file_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub content: String,
    pub score: f32,
}

/// User-facing settings persisted in the `settings` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub level: Level,
    pub chat_model: String,
    pub embed_model: String,
    /// Hour of day (0..=23) at which the daily popup should fire.
    pub schedule_hour: u8,
    pub ollama_url: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            level: Level::Senior,
            chat_model: "llama3.1:8b".to_string(),
            embed_model: "nomic-embed-text".to_string(),
            schedule_hour: 9,
            ollama_url: "http://127.0.0.1:11434".to_string(),
        }
    }
}

/// A catalog topic the user can opt into, with their chosen target difficulty
/// and rolled-up progress. Returned to the Topics panel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicCard {
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub area: String,
    pub min_level: Level,
    /// Whether the user has selected this topic to study.
    pub enabled: bool,
    /// The difficulty tier the user wants questions generated at.
    pub target_difficulty: Difficulty,
    /// Lifetime questions answered for this topic.
    pub answered: u32,
    /// Lifetime correct answers for this topic.
    pub correct: u32,
    /// Accuracy per difficulty tier, for the easy/medium/hard/advanced breakdown.
    pub by_difficulty: Vec<DifficultyStat>,
}

/// Correct/total counts for one difficulty tier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifficultyStat {
    pub difficulty: Difficulty,
    pub correct: u32,
    pub total: u32,
}

/// The user's selection + difficulty target for a single topic (persisted).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicPref {
    pub slug: String,
    pub enabled: bool,
    pub target_difficulty: Difficulty,
}

/// How a follow-up request relates to an answered question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FollowupMode {
    /// Explain the concept and why each option is right/wrong, in depth.
    Explain,
    /// Pose and answer a harder interview-style follow-up on the same concept.
    Followup,
    /// Answer the user's own free-form question about the topic.
    Custom,
}

impl FollowupMode {
    pub fn from_str_lenient(s: &str) -> FollowupMode {
        match s.trim().to_lowercase().as_str() {
            "explain" => FollowupMode::Explain,
            "custom" => FollowupMode::Custom,
            _ => FollowupMode::Followup,
        }
    }
}
