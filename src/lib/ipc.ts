// Typed wrappers around Tauri's invoke()/Channel/event APIs.

import { invoke, Channel } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AttemptResult,
  CodeReview,
  CodingProblem,
  DailySession,
  Difficulty,
  DsCategoryCard,
  FetchedPage,
  FollowupMode,
  FreeResponseGrade,
  FreeResponseQuestion,
  IndexProgress,
  ModelInfo,
  PathCard,
  ProgressStats,
  PullProgress,
  RecommendedModel,
  Repo,
  ReviewItem,
  SearchResult,
  Settings,
  Streak,
  StreamChunk,
  TopicCard,
} from "../types";

// ---- settings ----------------------------------------------------------

export const getSettings = () => invoke<Settings>("get_settings");
export const updateSettings = (settings: Settings) =>
  invoke<void>("update_settings", { settings });
export const listOllamaModels = () => invoke<string[]>("list_ollama_models");
export const checkOllamaHealth = () => invoke<boolean>("check_ollama_health");

// ---- daily -------------------------------------------------------------

export const getTodaySession = () => invoke<DailySession>("get_today_session");
export const submitAnswer = (
  sessionId: number,
  questionId: number,
  chosenIndex: number,
) =>
  invoke<AttemptResult>("submit_answer", {
    sessionId,
    questionId,
    chosenIndex,
  });
export const getStreak = () => invoke<Streak>("get_streak");
export const getProgress = () => invoke<ProgressStats>("get_progress");
export const shouldShowToday = () => invoke<boolean>("should_show_today");
export const markShownToday = () => invoke<void>("mark_shown_today");

// ---- topics, custom topics, paths --------------------------------------

export const listTopics = () => invoke<TopicCard[]>("list_topics");
export const setTopicPref = (
  slug: string,
  enabled: boolean,
  targetDifficulty: Difficulty,
) => invoke<void>("set_topic_pref", { slug, enabled, targetDifficulty });
export const addCustomTopic = (name: string) =>
  invoke<string>("add_custom_topic", { name });
export const deleteCustomTopic = (slug: string) =>
  invoke<void>("delete_custom_topic", { slug });
export const listPaths = () => invoke<PathCard[]>("list_paths");

// ---- spaced repetition -------------------------------------------------

export const getDueReviews = () => invoke<ReviewItem[]>("get_due_reviews");
export const countDueReviews = () => invoke<number>("count_due_reviews");
export const submitReview = (questionId: number, chosenIndex: number) =>
  invoke<AttemptResult>("submit_review", { questionId, chosenIndex });

// ---- free-response practice --------------------------------------------

export const generateFreeResponse = (topicSlug?: string) =>
  invoke<FreeResponseQuestion>("generate_free_response", {
    topicSlug: topicSlug ?? null,
  });
export const gradeFreeResponse = (
  question: FreeResponseQuestion,
  answer: string,
) => invoke<FreeResponseGrade>("grade_free_response", { question, answer });

// ---- insight: weakness report ------------------------------------------

export async function generateWeaknessReport(
  onChunk: (chunk: StreamChunk) => void,
): Promise<void> {
  const channel = new Channel<StreamChunk>();
  channel.onmessage = onChunk;
  await invoke<void>("generate_weakness_report", { onEvent: channel });
}

// ---- model manager -----------------------------------------------------

export const listInstalledModels = () =>
  invoke<ModelInfo[]>("list_installed_models");
export const recommendedModels = () =>
  invoke<RecommendedModel[]>("recommended_models");
export const deleteModel = (name: string) =>
  invoke<void>("delete_model", { name });

export async function pullModel(
  name: string,
  onProgress: (p: PullProgress) => void,
): Promise<void> {
  const channel = new Channel<PullProgress>();
  channel.onmessage = onProgress;
  await invoke<void>("pull_model", { name, onEvent: channel });
}

// ---- coding (LeetCode-style) -------------------------------------------

export const listDsCategories = () =>
  invoke<DsCategoryCard[]>("list_ds_categories");
export const generateCodingProblem = (
  categorySlug: string,
  difficulty: Difficulty,
) =>
  invoke<CodingProblem>("generate_coding_problem", { categorySlug, difficulty });
export const reviewSolution = (
  problem: CodingProblem,
  code: string,
  language: string,
) => invoke<CodeReview>("review_solution", { problem, code, language });

// ---- web tools (/url, /search) -----------------------------------------

export const fetchUrl = (url: string) => invoke<FetchedPage>("fetch_url", { url });
export const webSearch = (query: string) =>
  invoke<SearchResult[]>("web_search", { query });

export async function askWeb(
  question: string,
  context: string,
  sources: string[],
  onChunk: (chunk: StreamChunk) => void,
): Promise<void> {
  const channel = new Channel<StreamChunk>();
  channel.onmessage = onChunk;
  await invoke<void>("ask_web", { question, context, sources, onEvent: channel });
}

// ---- system: hotkey + reminders ----------------------------------------

export const setGlobalShortcut = (accelerator: string) =>
  invoke<void>("set_global_shortcut", { accelerator });
export const sendReminderIfDue = () => invoke<boolean>("send_reminder_if_due");

/** Stream a follow-up explanation / question for an answered MCQ. */
export async function askFollowup(
  questionId: number,
  mode: FollowupMode,
  onChunk: (chunk: StreamChunk) => void,
  userQuery?: string,
): Promise<void> {
  const channel = new Channel<StreamChunk>();
  channel.onmessage = onChunk;
  await invoke<void>("ask_followup", {
    questionId,
    mode,
    userQuery: userQuery ?? null,
    onEvent: channel,
  });
}

// ---- repos -------------------------------------------------------------

export const linkRepo = (url: string) => invoke<number>("link_repo", { url });
export const listRepos = () => invoke<Repo[]>("list_repos");
export const getRepo = (repoId: number) => invoke<Repo>("get_repo", { repoId });
export const deleteRepo = (repoId: number) =>
  invoke<void>("delete_repo", { repoId });

/** Ask a question about a repo; `onChunk` receives streamed tokens. */
export async function askRepo(
  repoId: number,
  question: string,
  onChunk: (chunk: StreamChunk) => void,
): Promise<void> {
  const channel = new Channel<StreamChunk>();
  channel.onmessage = onChunk;
  await invoke<void>("ask_repo", { repoId, question, onEvent: channel });
}

/** Subscribe to indexing-progress events. Returns an unlisten function. */
export function onIndexProgress(
  handler: (p: IndexProgress) => void,
): Promise<UnlistenFn> {
  return listen<IndexProgress>("repo-index-progress", (e) => handler(e.payload));
}

/** Subscribe to tray-driven navigation events. */
export function onNavigate(handler: (route: string) => void): Promise<UnlistenFn> {
  return listen<string>("navigate", (e) => handler(e.payload));
}

// ---- window / schedule -------------------------------------------------

export const setDailySchedule = (hour: number, enabled: boolean) =>
  invoke<void>("set_daily_schedule", { hour, enabled });
