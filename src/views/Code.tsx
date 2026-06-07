import { useEffect, useState } from "react";
import ReactMarkdown from "react-markdown";
import type {
  CodeReview,
  CodingProblem,
  Difficulty,
  DsCategoryCard,
} from "../types";
import {
  generateCodingProblem,
  listDsCategories,
  reviewSolution,
} from "../lib/ipc";

const DIFFICULTIES: Difficulty[] = ["easy", "medium", "hard"];
const LANGUAGES = ["python", "javascript", "typescript", "java", "c++", "go", "rust"];

const DIFF_BADGE: Record<string, string> = {
  easy: "bg-emerald-500/20 text-emerald-300",
  medium: "bg-amber-500/20 text-amber-300",
  hard: "bg-rose-500/20 text-rose-300",
  advanced: "bg-rose-500/20 text-rose-300",
};

export default function Code() {
  const [categories, setCategories] = useState<DsCategoryCard[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [difficulty, setDifficulty] = useState<Difficulty>("medium");
  const [problem, setProblem] = useState<CodingProblem | null>(null);
  const [language, setLanguage] = useState("python");
  const [code, setCode] = useState("");
  const [review, setReview] = useState<CodeReview | null>(null);
  const [generating, setGenerating] = useState(false);
  const [reviewing, setReviewing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function refresh() {
    listDsCategories().then(setCategories).catch((e) => setError(String(e)));
  }
  useEffect(refresh, []);

  async function generate() {
    if (!selected || generating) return;
    setGenerating(true);
    setError(null);
    setReview(null);
    setProblem(null);
    setCode("");
    try {
      const p = await generateCodingProblem(selected, difficulty);
      setProblem(p);
      setCode(p.starter_signature ? `${p.starter_signature}\n    ` : "");
    } catch (e) {
      setError(String(e));
    } finally {
      setGenerating(false);
    }
  }

  async function submit() {
    if (!problem || !code.trim() || reviewing) return;
    setReviewing(true);
    setError(null);
    try {
      setReview(await reviewSolution(problem, code, language));
      refresh(); // update category progress
    } catch (e) {
      setError(String(e));
    } finally {
      setReviewing(false);
    }
  }

  const verdictClass = (v: string) =>
    v === "correct"
      ? "bg-emerald-500/20 text-emerald-200"
      : v === "incorrect"
        ? "bg-rose-500/20 text-rose-200"
        : "bg-amber-500/20 text-amber-200";

  return (
    <div className="flex flex-col gap-4">
      <p className="text-xs text-slate-400">
        LeetCode-style problems across every data structure. Pick a category,
        write a solution, and have your local model review correctness, time/space
        complexity, edge cases, and the optimal approach.
      </p>

      {/* Category grid */}
      <div className="grid grid-cols-2 gap-2">
        {categories.map((c) => (
          <button
            key={c.slug}
            onClick={() => setSelected(c.slug)}
            className={`rounded-lg border p-2 text-left transition ${
              selected === c.slug
                ? "border-accent/60 bg-accent/10"
                : "border-white/10 bg-white/5 hover:border-white/20"
            }`}
          >
            <div className="flex items-center justify-between">
              <span className="text-xs font-medium">{c.title}</span>
              {c.attempted > 0 && (
                <span className="text-[10px] text-emerald-300">
                  {c.solved}/{c.attempted}
                </span>
              )}
            </div>
          </button>
        ))}
      </div>

      {/* Difficulty + generate */}
      <div className="flex items-center gap-2">
        <div className="flex gap-1">
          {DIFFICULTIES.map((d) => (
            <button
              key={d}
              onClick={() => setDifficulty(d)}
              className={`rounded-md border px-2 py-1 text-[11px] capitalize transition ${
                difficulty === d
                  ? "border-accent/60 bg-accent/15 text-accent"
                  : "border-white/10 bg-white/5 text-slate-400"
              }`}
            >
              {d}
            </button>
          ))}
        </div>
        <button
          onClick={generate}
          disabled={!selected || generating}
          className="ml-auto rounded-lg bg-accent/20 px-3 py-1.5 text-sm font-medium text-accent transition hover:bg-accent/30 disabled:opacity-50"
        >
          {generating ? "Generating…" : problem ? "New problem" : "Generate problem"}
        </button>
      </div>

      {/* Problem */}
      {problem && (
        <div className="rounded-xl border border-white/10 bg-panel/60 p-4">
          <div className="mb-2 flex items-center justify-between">
            <h2 className="text-sm font-semibold">{problem.title}</h2>
            <span
              className={`rounded-full px-2 py-0.5 text-[10px] capitalize ${
                DIFF_BADGE[problem.difficulty] ?? ""
              }`}
            >
              {problem.difficulty}
            </span>
          </div>
          <p className="whitespace-pre-wrap text-sm leading-relaxed text-slate-200">
            {problem.prompt}
          </p>
          {problem.examples.length > 0 && (
            <div className="mt-2">
              <p className="text-[11px] uppercase tracking-wider text-slate-500">Examples</p>
              <ul className="mt-1 space-y-0.5 font-mono text-xs text-slate-300">
                {problem.examples.map((e, i) => (
                  <li key={i}>{e}</li>
                ))}
              </ul>
            </div>
          )}
          {problem.constraints.length > 0 && (
            <div className="mt-2">
              <p className="text-[11px] uppercase tracking-wider text-slate-500">Constraints</p>
              <ul className="mt-1 list-inside list-disc text-xs text-slate-400">
                {problem.constraints.map((e, i) => (
                  <li key={i}>{e}</li>
                ))}
              </ul>
            </div>
          )}
          {(problem.optimal_time || problem.optimal_space) && (
            <p className="mt-2 text-[11px] text-slate-500">
              Target complexity: time {problem.optimal_time || "?"}, space{" "}
              {problem.optimal_space || "?"}
            </p>
          )}
        </div>
      )}

      {/* Editor */}
      {problem && (
        <div className="flex flex-col gap-2">
          <div className="flex items-center gap-2">
            <span className="text-xs text-slate-400">Language</span>
            <select
              value={language}
              onChange={(e) => setLanguage(e.target.value)}
              className="rounded-md border border-white/10 bg-white/5 px-2 py-1 text-xs outline-none focus:border-accent/60"
            >
              {LANGUAGES.map((l) => (
                <option key={l} value={l}>
                  {l}
                </option>
              ))}
            </select>
          </div>
          <textarea
            value={code}
            onChange={(e) => setCode(e.target.value)}
            spellCheck={false}
            rows={12}
            placeholder="Write your solution…"
            className="w-full resize-y rounded-lg border border-white/10 bg-ink/80 px-3 py-2 font-mono text-xs leading-relaxed text-slate-100 outline-none focus:border-accent/60"
          />
          <button
            onClick={submit}
            disabled={reviewing || !code.trim()}
            className="self-start rounded-lg bg-accent/20 px-4 py-2 text-sm font-medium text-accent transition hover:bg-accent/30 disabled:opacity-50"
          >
            {reviewing ? "Reviewing…" : "Submit for review"}
          </button>
        </div>
      )}

      {/* Review */}
      {review && (
        <div className="rounded-xl border border-white/10 bg-panel/60 p-4">
          <div className="mb-2 flex flex-wrap items-center gap-2">
            <span
              className={`rounded-full px-2 py-0.5 text-xs font-medium capitalize ${verdictClass(
                review.verdict,
              )}`}
            >
              {review.verdict}
            </span>
            <span className="text-sm font-semibold">
              {review.score}/{review.max_score}
            </span>
            <span className="ml-auto font-mono text-[11px] text-slate-400">
              time {review.time_complexity} · space {review.space_complexity}
            </span>
          </div>
          <p className="text-sm text-slate-200">{review.correctness}</p>
          {review.edge_cases_missed.length > 0 && (
            <div className="mt-2">
              <p className="text-[11px] font-semibold uppercase tracking-wider text-rose-300">
                Edge cases missed
              </p>
              <ul className="mt-0.5 list-inside list-disc text-xs text-slate-300">
                {review.edge_cases_missed.map((s, i) => (
                  <li key={i}>{s}</li>
                ))}
              </ul>
            </div>
          )}
          <div className="prose prose-invert mt-2 max-w-none text-xs text-slate-200">
            <ReactMarkdown>{review.feedback}</ReactMarkdown>
          </div>
          <details className="mt-2">
            <summary className="cursor-pointer text-xs font-medium text-accent">
              Show optimal approach
            </summary>
            <div className="prose prose-invert mt-1 max-w-none text-xs text-slate-200">
              <ReactMarkdown>{review.optimal_approach}</ReactMarkdown>
            </div>
          </details>
        </div>
      )}

      {error && <p className="text-xs text-rose-300">{error}</p>}
    </div>
  );
}
