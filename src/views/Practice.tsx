import { useEffect, useState } from "react";
import type { FreeResponseGrade, FreeResponseQuestion, TopicCard } from "../types";
import {
  generateFreeResponse,
  gradeFreeResponse,
  listTopics,
} from "../lib/ipc";

export default function Practice() {
  const [topics, setTopics] = useState<TopicCard[]>([]);
  const [slug, setSlug] = useState<string>("");
  const [question, setQuestion] = useState<FreeResponseQuestion | null>(null);
  const [answer, setAnswer] = useState("");
  const [grade, setGrade] = useState<FreeResponseGrade | null>(null);
  const [generating, setGenerating] = useState(false);
  const [grading, setGrading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    listTopics()
      .then((t) => setTopics(t.filter((x) => x.enabled)))
      .catch((e) => setError(String(e)));
  }, []);

  async function newQuestion() {
    setGenerating(true);
    setError(null);
    setGrade(null);
    setAnswer("");
    setQuestion(null);
    try {
      setQuestion(await generateFreeResponse(slug || undefined));
    } catch (e) {
      setError(String(e));
    } finally {
      setGenerating(false);
    }
  }

  async function submit() {
    if (!question || !answer.trim() || grading) return;
    setGrading(true);
    setError(null);
    try {
      setGrade(await gradeFreeResponse(question, answer.trim()));
    } catch (e) {
      setError(String(e));
    } finally {
      setGrading(false);
    }
  }

  const pct = grade ? Math.round((grade.score / grade.max_score) * 100) : 0;

  return (
    <div className="flex flex-col gap-4">
      <p className="text-xs text-neutral-500">
        Open-ended interview practice: write a full answer and get it graded by
        your local model against a rubric.
      </p>

      <div className="flex gap-2">
        <select
          value={slug}
          onChange={(e) => setSlug(e.target.value)}
          className="flex-1 rounded-lg border border-black/[0.08] bg-black/[0.04] px-3 py-2 text-sm outline-none focus:border-accent/60"
        >
          <option value="">Any selected topic</option>
          {topics.map((t) => (
            <option key={t.slug} value={t.slug}>
              {t.title}
            </option>
          ))}
        </select>
        <button
          onClick={newQuestion}
          disabled={generating}
          className="rounded-lg bg-accent/20 px-3 py-2 text-sm font-medium text-accent transition hover:bg-accent/30 disabled:opacity-50"
        >
          {generating ? "…" : "New question"}
        </button>
      </div>

      {question && (
        <div className="flex flex-col gap-3">
          <div className="rounded-xl border border-black/[0.08] bg-white p-4">
            <p className="text-sm leading-relaxed text-neutral-900">{question.prompt}</p>
            {question.rubric.length > 0 && (
              <div className="mt-3 border-t border-black/[0.08] pt-2">
                <p className="text-[11px] uppercase tracking-wider text-neutral-400">
                  Graded on
                </p>
                <ul className="mt-1 list-inside list-disc text-xs text-neutral-500">
                  {question.rubric.map((r, i) => (
                    <li key={i}>{r}</li>
                  ))}
                </ul>
              </div>
            )}
          </div>

          <textarea
            value={answer}
            onChange={(e) => setAnswer(e.target.value)}
            placeholder="Write your answer…"
            rows={6}
            className="w-full resize-y rounded-lg border border-black/[0.08] bg-black/[0.04] px-3 py-2 text-sm outline-none focus:border-accent/60"
          />
          <button
            onClick={submit}
            disabled={grading || !answer.trim()}
            className="self-start rounded-lg bg-accent/20 px-4 py-2 text-sm font-medium text-accent transition hover:bg-accent/30 disabled:opacity-50"
          >
            {grading ? "Grading…" : "Submit for grading"}
          </button>
        </div>
      )}

      {grade && (
        <div className="rounded-xl border border-black/[0.08] bg-white p-4">
          <div className="mb-2 flex items-center justify-between">
            <span className="text-sm font-semibold">
              Score: {grade.score} / {grade.max_score}
            </span>
            <span
              className={`rounded-full px-2 py-0.5 text-xs ${
                pct >= 70
                  ? "bg-emerald-500/20 text-emerald-700"
                  : pct >= 40
                    ? "bg-amber-500/20 text-amber-700"
                    : "bg-rose-500/20 text-rose-700"
              }`}
            >
              {pct}%
            </span>
          </div>
          <p className="text-sm leading-relaxed text-neutral-800">{grade.feedback}</p>
          {grade.strengths.length > 0 && (
            <Section title="Strengths" items={grade.strengths} color="text-emerald-700" />
          )}
          {grade.gaps.length > 0 && (
            <Section title="Gaps" items={grade.gaps} color="text-rose-600" />
          )}
        </div>
      )}

      {error && <p className="text-xs text-rose-600">{error}</p>}
    </div>
  );
}

function Section({
  title,
  items,
  color,
}: {
  title: string;
  items: string[];
  color: string;
}) {
  return (
    <div className="mt-2">
      <p className={`text-[11px] font-semibold uppercase tracking-wider ${color}`}>
        {title}
      </p>
      <ul className="mt-0.5 list-inside list-disc text-xs text-neutral-600">
        {items.map((s, i) => (
          <li key={i}>{s}</li>
        ))}
      </ul>
    </div>
  );
}
