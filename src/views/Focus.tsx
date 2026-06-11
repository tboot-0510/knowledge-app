import { useEffect, useRef, useState } from "react";
import type { AttemptResult, Difficulty, Question, TopicCard } from "../types";
import {
  generateFocusQuestion,
  listTopics,
  submitFocusAnswer,
} from "../lib/ipc";
import McqCard from "../components/McqCard";

const DURATIONS = [5, 10, 15, 25];
const TIERS: Difficulty[] = ["easy", "medium", "hard", "advanced"];

function harder(d: Difficulty): Difficulty {
  return TIERS[Math.min(TIERS.indexOf(d) + 1, TIERS.length - 1)];
}
function easier(d: Difficulty): Difficulty {
  return TIERS[Math.max(TIERS.indexOf(d) - 1, 0)];
}
function mmss(total: number): string {
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${m}:${s.toString().padStart(2, "0")}`;
}

type Phase = "setup" | "running" | "done";

export default function Focus() {
  const [phase, setPhase] = useState<Phase>("setup");
  const [topics, setTopics] = useState<TopicCard[]>([]);
  const [slug, setSlug] = useState("");
  const [minutes, setMinutes] = useState(10);

  const [secondsLeft, setSecondsLeft] = useState(0);
  const [difficulty, setDifficulty] = useState<Difficulty>("medium");
  const [question, setQuestion] = useState<Question | null>(null);
  const [result, setResult] = useState<AttemptResult | undefined>(undefined);
  const [answered, setAnswered] = useState(0);
  const [correct, setCorrect] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const endRef = useRef<number>(0);

  useEffect(() => {
    listTopics()
      .then((t) => setTopics(t.filter((x) => x.enabled)))
      .catch(() => setTopics([]));
  }, []);

  // Countdown.
  useEffect(() => {
    if (phase !== "running") return;
    const id = setInterval(() => {
      const left = Math.max(0, Math.round((endRef.current - Date.now()) / 1000));
      setSecondsLeft(left);
      if (left <= 0) {
        clearInterval(id);
        setPhase("done");
      }
    }, 250);
    return () => clearInterval(id);
  }, [phase]);

  async function loadNext(diff: Difficulty) {
    setLoading(true);
    setError(null);
    setResult(undefined);
    setQuestion(null);
    try {
      setQuestion(await generateFocusQuestion(slug || undefined, diff));
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  async function start() {
    setAnswered(0);
    setCorrect(0);
    setDifficulty("medium");
    endRef.current = Date.now() + minutes * 60 * 1000;
    setSecondsLeft(minutes * 60);
    setPhase("running");
    await loadNext("medium");
  }

  async function answer(idx: number) {
    if (!question || result) return;
    try {
      const res = await submitFocusAnswer(question.id, idx);
      setResult(res);
      setAnswered((a) => a + 1);
      if (res.is_correct) setCorrect((c) => c + 1);
      // Harden on success, ease on a miss — the adaptive loop.
      setDifficulty((d) => (res.is_correct ? harder(d) : easier(d)));
    } catch (e) {
      setError(String(e));
    }
  }

  function next() {
    if (phase !== "running") return;
    loadNext(difficulty);
  }

  // ---- setup ----
  if (phase === "setup") {
    return (
      <div className="flex flex-col gap-5">
        <div>
          <h2 className="font-display text-lg font-semibold">Focus session</h2>
          <p className="mt-0.5 text-xs text-neutral-500">
            Pick how long you want to study. You'll get a continuous stream of
            questions that get harder as you get them right, and ease off when you
            miss.
          </p>
        </div>

        <div>
          <p className="mb-1.5 text-xs font-semibold uppercase tracking-wider text-neutral-400">
            Duration
          </p>
          <div className="flex gap-2">
            {DURATIONS.map((m) => (
              <button
                key={m}
                onClick={() => setMinutes(m)}
                className={`flex-1 rounded-xl border py-3 text-center transition ${
                  minutes === m
                    ? "border-accent bg-accent/10 text-accent"
                    : "border-black/[0.08] bg-white text-neutral-700 hover:border-black/20"
                }`}
              >
                <div className="text-lg font-semibold">{m}</div>
                <div className="text-[11px] text-neutral-500">min</div>
              </button>
            ))}
          </div>
        </div>

        <div>
          <p className="mb-1.5 text-xs font-semibold uppercase tracking-wider text-neutral-400">
            Topic
          </p>
          <select
            value={slug}
            onChange={(e) => setSlug(e.target.value)}
            className="w-full rounded-lg border border-black/[0.08] bg-white px-3 py-2 text-sm outline-none focus:border-accent/60"
          >
            <option value="">Mixed (all selected topics)</option>
            {topics.map((t) => (
              <option key={t.slug} value={t.slug}>
                {t.title}
              </option>
            ))}
          </select>
        </div>

        <button
          onClick={start}
          className="rounded-full bg-accent px-5 py-2.5 text-sm font-medium text-white transition hover:opacity-90"
        >
          Start {minutes}-minute session
        </button>
      </div>
    );
  }

  // ---- done ----
  if (phase === "done") {
    const pct = answered > 0 ? Math.round((correct / answered) * 100) : 0;
    return (
      <div className="flex flex-col items-center gap-4 py-6 text-center">
        <span className="text-4xl">🎯</span>
        <h2 className="font-display text-2xl font-semibold">Session complete</h2>
        <div className="flex gap-6">
          <Stat label="Answered" value={`${answered}`} />
          <Stat label="Correct" value={`${correct}`} />
          <Stat label="Accuracy" value={`${pct}%`} />
        </div>
        <p className="max-w-xs text-xs text-neutral-500">
          Every question counts toward your progress and is scheduled for review.
        </p>
        <button
          onClick={() => setPhase("setup")}
          className="rounded-full bg-accent px-5 py-2 text-sm font-medium text-white transition hover:opacity-90"
        >
          New session
        </button>
      </div>
    );
  }

  // ---- running ----
  const low = secondsLeft <= 30;
  return (
    <div className="flex flex-col gap-4">
      <div className="sticky top-0 z-10 -mx-5 flex items-center justify-between border-b border-black/[0.06] bg-paper/95 px-5 pb-2">
        <div
          className={`font-display text-2xl font-semibold tabular-nums ${
            low ? "text-rose-600" : "text-ink"
          }`}
        >
          {mmss(secondsLeft)}
        </div>
        <div className="flex items-center gap-3 text-xs text-neutral-500">
          <span>
            {correct}/{answered} correct
          </span>
          <span className="rounded-full bg-accent/10 px-2 py-0.5 capitalize text-accent">
            {difficulty}
          </span>
          <button
            onClick={() => setPhase("done")}
            className="rounded-full border border-black/[0.08] px-2 py-0.5 text-neutral-600 hover:border-black/20"
          >
            Stop
          </button>
        </div>
      </div>

      {error && <p className="text-xs text-rose-600">{error}</p>}

      {loading && !question && (
        <div className="flex flex-col items-center gap-2 py-8 text-neutral-400">
          <div className="h-5 w-5 animate-spin rounded-full border-2 border-accent/40 border-t-accent" />
          <p className="text-xs">Generating a {difficulty} question…</p>
        </div>
      )}

      {question && (
        <>
          <McqCard
            question={question}
            index={answered}
            result={result}
            onAnswer={answer}
          />
          {result && (
            <button
              onClick={next}
              className="self-end rounded-full bg-accent px-5 py-2 text-sm font-medium text-white transition hover:opacity-90"
            >
              Next question →
            </button>
          )}
        </>
      )}
    </div>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <div className="font-display text-2xl font-semibold">{value}</div>
      <div className="text-[11px] uppercase tracking-wider text-neutral-400">
        {label}
      </div>
    </div>
  );
}
