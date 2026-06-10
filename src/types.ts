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
  mcq_model: string;
  embed_model: string;
  schedule_hour: number;
  ollama_url: string;
  reminders_enabled: boolean;
  global_shortcut: string;
  onboarded: boolean;
}

export interface ReviewItem {
  question: Question;
  topic_title: string;
  due_date: string;
}

export interface FreeResponseQuestion {
  topic_slug: string;
  prompt: string;
  rubric: string[];
  max_score: number;
}

export interface FreeResponseGrade {
  score: number;
  max_score: number;
  feedback: string;
  strengths: string[];
  gaps: string[];
}

export interface PathStep {
  slug: string;
  title: string;
  answered: number;
  correct: number;
}

export interface PathCard {
  slug: string;
  title: string;
  description: string;
  steps: PathStep[];
  started: number;
  total: number;
}

export interface ModelInfo {
  name: string;
  size_bytes: number;
}

export interface RecommendedModel {
  name: string;
  purpose: string;
  note: string;
}

export interface PullProgress {
  status: string;
  total: number;
  completed: number;
  done: boolean;
}

export interface SearchResult {
  title: string;
  url: string;
  snippet: string;
}

export interface FetchedPage {
  url: string;
  title: string;
  text: string;
}

export interface DsCategoryCard {
  slug: string;
  title: string;
  description: string;
  attempted: number;
  solved: number;
}

export interface CodingProblem {
  category_slug: string;
  title: string;
  prompt: string;
  examples: string[];
  constraints: string[];
  difficulty: Difficulty;
  starter_signature: string | null;
  optimal_time: string;
  optimal_space: string;
}

export interface CodeReview {
  verdict: string;
  score: number;
  max_score: number;
  time_complexity: string;
  space_complexity: string;
  correctness: string;
  edge_cases_missed: string[];
  feedback: string;
  optimal_approach: string;
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
