import { useEffect, useState } from "react";
import type { AttemptResult, DailySession, Streak } from "../types";
import {
  getStreak,
  getTodaySession,
  markShownToday,
  submitAnswer,
} from "../lib/ipc";
import McqCard from "../components/McqCard";
import StreakBadge from "../components/StreakBadge";

export default function DailyChallenge() {
  const [session, setSession] = useState<DailySession | null>(null);
  const [results, setResults] = useState<Record<number, AttemptResult>>({});
  const [streak, setStreak] = useState<Streak | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    (async () => {
      try {
        const [s, st] = await Promise.all([getTodaySession(), getStreak()]);
        if (!active) return;
        setSession(s);
        setStreak(st);
        // Seed results for an already-completed session so we don't re-answer.
        await markShownToday();
      } catch (e) {
        if (active) setError(String(e));
      } finally {
        if (active) setLoading(false);
      }
    })();
    return () => {
      active = false;
    };
  }, []);

  async function answer(questionId: number, chosenIndex: number) {
    if (!session || results[questionId]) return;
    try {
      const res = await submitAnswer(session.id, questionId, chosenIndex);
      setResults((prev) => ({ ...prev, [questionId]: res }));
      if (res.session_completed) {
        setStreak(await getStreak());
        setSession((prev) => (prev ? { ...prev, completed: true } : prev));
      }
    } catch (e) {
      setError(String(e));
    }
  }

  if (loading) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-3 text-slate-400">
        <div className="h-6 w-6 animate-spin rounded-full border-2 border-accent/40 border-t-accent" />
        <p className="text-sm">Preparing today's challenge…</p>
        <p className="text-xs text-slate-500">
          Generating questions with your local model.
        </p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="rounded-lg border border-rose-500/30 bg-rose-500/10 p-4 text-sm text-rose-200">
        <p className="font-semibold">Couldn't load today's challenge</p>
        <p className="mt-1 text-rose-200/80">{error}</p>
        <p className="mt-2 text-xs text-rose-200/60">
          Make sure Ollama is running (<code>ollama serve</code>) or check Settings.
        </p>
      </div>
    );
  }

  if (!session) return null;

  const answeredCount = Object.keys(results).length;
  const correctCount = Object.values(results).filter((r) => r.is_correct).length;
  const allAnswered = answeredCount >= session.questions.length;

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="text-[11px] uppercase tracking-wider text-accent">
            {session.topic.area.replace(/-/g, " ")} · {session.topic.level}
          </p>
          <h2 className="mt-0.5 text-lg font-semibold">{session.topic.title}</h2>
        </div>
        {streak && <StreakBadge streak={streak} />}
      </div>

      <p className="rounded-lg bg-white/5 p-3 text-sm leading-relaxed text-slate-300">
        {session.topic.summary}
      </p>

      <div className="flex flex-col gap-3">
        {session.questions.map((q, i) => (
          <McqCard
            key={q.id}
            question={q}
            index={i}
            result={results[q.id]}
            onAnswer={(idx) => answer(q.id, idx)}
          />
        ))}
      </div>

      {allAnswered && (
        <div className="rounded-xl border border-emerald-500/30 bg-emerald-500/10 p-4 text-center">
          <p className="text-sm font-semibold text-emerald-200">
            Done! You scored {correctCount} / {session.questions.length}
          </p>
          <p className="mt-1 text-xs text-emerald-200/70">
            Come back tomorrow to keep your streak alive.
          </p>
        </div>
      )}
    </div>
  );
}
