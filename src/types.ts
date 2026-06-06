// TypeScript mirrors of the Rust DTOs in crates/core/src/models.rs and the
// StreamChunk enum in crates/core/src/ollama.rs. Keep in sync by hand.

export type Level = "senior" | "staff" | "principal";

export type Difficulty = "easy" | "medium" | "hard" | "advanced";

export type FollowupMode = "explain" | "followup" | "custom";

export interface DifficultyStat {
  difficulty: Difficulty;
  correct: number;
  total: number;
}

export interface TopicCard {
  slug: string;
  title: string;
  summary: string;
  area: string;
  min_level: Level;
  enabled: boolean;
  target_difficulty: Difficulty;
  answered: number;
  correct: number;
  by_difficulty: DifficultyStat[];
}

export interface Topic {
  id: number;
  slug: string;
  title: string;
  summary: string;
  area: string;
  level: Level;
  source: string;
  talking_points: string[];
}

export interface Question {
  id: number;
  topic_id: number;
  prompt: string;
  choices: string[];
  correct_index: number;
  explanation: string;
  difficulty: number;
}

export interface DailySession {
  id: number;
  date: string;
  topic: Topic;
  questions: Question[];
  completed: boolean;
  score: number | null;
}

export interface AttemptResult {
  question_id: number;
  chosen_index: number;
  correct_index: number;
  is_correct: boolean;
  explanation: string;
  session_completed: boolean;
}

export interface Streak {
  current_streak: number;
  longest_streak: number;
  last_active_date: string | null;
}

export interface AreaStat {
  area: string;
  correct: number;
  total: number;
}

export interface ProgressStats {
  total_questions: number;
  total_correct: number;
  days_completed: number;
  streak: Streak;
  by_area: AreaStat[];
}

export type RepoStatus = "pending" | "cloning" | "indexing" | "ready" | "error";

export interface Repo {
  id: number;
  name: string;
  url: string;
  local_path: string;
  status: RepoStatus;
  file_count: number;
  chunk_count: number;
  indexed_at: string | null;
  error: string | null;
}

export interface Settings {
  level: Level;
  chat_model: string;
  embed_model: string;
  schedule_hour: number;
  ollama_url: string;
}

// Streaming chunk from a repo Q&A answer.
export type StreamChunk =
  | { kind: "token"; text: string }
  | { kind: "done" }
  | { kind: "error"; message: string };

export interface IndexProgress {
  repo_id: number;
  done: number;
  total: number;
}
