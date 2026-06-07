import { useEffect, useState } from "react";
import type { AttemptResult, ReviewItem } from "../types";
import { getDueReviews, submitReview } from "../lib/ipc";
import McqCard from "../components/McqCard";

export default function Review() {
  const [items, setItems] = useState<ReviewItem[]>([]);
  const [results, setResults] = useState<Record<number, AttemptResult>>({});
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getDueReviews()
      .then(setItems)
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, []);

  async function answer(questionId: number, chosenIndex: number) {
    if (results[questionId]) return;
    try {
      const res = await submitReview(questionId, chosenIndex);
      setResults((prev) => ({ ...prev, [questionId]: res }));
    } catch (e) {
      setError(String(e));
    }
  }

  if (loading) return <p className="text-sm text-slate-400">Loading reviews…</p>;
  if (error) return <p className="text-sm text-rose-300">{error}</p>;

  if (items.length === 0) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-2 text-center text-slate-400">
        <span className="text-3xl">🧠</span>
        <p className="text-sm font-medium text-slate-300">Nothing due for review</p>
        <p className="max-w-xs text-xs text-slate-500">
          Questions you answer are scheduled with spaced repetition (SM-2) and
          resurface here right before you'd forget them.
        </p>
      </div>
    );
  }

  const reviewed = Object.keys(results).length;

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between">
        <h2 className="text-sm font-semibold">Due for review</h2>
        <span className="text-xs text-slate-400">
          {reviewed}/{items.length} reviewed
        </span>
      </div>
      {items.map((item, i) => (
        <div key={item.question.id} className="flex flex-col gap-1">
          <span className="text-[11px] uppercase tracking-wider text-accent">
            {item.topic_title}
          </span>
          <McqCard
            question={item.question}
            index={i}
            result={results[item.question.id]}
            onAnswer={(idx) => answer(item.question.id, idx)}
          />
        </div>
      ))}
    </div>
  );
}
