// Typed wrappers around Tauri's invoke()/Channel/event APIs.

import { invoke, Channel } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AttemptResult,
  DailySession,
  Difficulty,
  FollowupMode,
  IndexProgress,
  ProgressStats,
  Repo,
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

// ---- topics ------------------------------------------------------------

export const listTopics = () => invoke<TopicCard[]>("list_topics");
export const setTopicPref = (
  slug: string,
  enabled: boolean,
  targetDifficulty: Difficulty,
) => invoke<void>("set_topic_pref", { slug, enabled, targetDifficulty });

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
