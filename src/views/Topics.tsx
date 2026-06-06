import { useEffect, useMemo, useState } from "react";
import type { Difficulty, TopicCard } from "../types";
import { listTopics, setTopicPref } from "../lib/ipc";

const DIFFICULTIES: Difficulty[] = ["easy", "medium", "hard", "advanced"];

const DIFF_COLOR: Record<Difficulty, string> = {
  easy: "text-emerald-300",
  medium: "text-sky-300",
  hard: "text-amber-300",
  advanced: "text-rose-300",
};

export default function Topics() {
  const [topics, setTopics] = useState<TopicCard[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    listTopics().then(setTopics).catch((e) => setError(String(e)));
  }, []);

  // Group topics by area for a tidy panel.
  const grouped = useMemo(() => {
    const map: Record<string, TopicCard[]> = {};
    for (const t of topics) (map[t.area] ??= []).push(t);
    return Object.entries(map).sort(([a], [b]) => a.localeCompare(b));
  }, [topics]);

  function update(slug: string, patch: Partial<TopicCard>) {
    setTopics((prev) =>
      prev.map((t) => (t.slug === slug ? { ...t, ...patch } : t)),
    );
  }

  async function toggle(t: TopicCard) {
    const enabled = !t.enabled;
    update(t.slug, { enabled });
    await setTopicPref(t.slug, enabled, t.target_difficulty).catch((e) =>
      setError(String(e)),
    );
  }

  async function setDiff(t: TopicCard, d: Difficulty) {
    update(t.slug, { target_difficulty: d });
    await setTopicPref(t.slug, t.enabled, d).catch((e) => setError(String(e)));
  }

  if (error) return <p className="text-sm text-rose-300">{error}</p>;

  return (
    <div className="flex flex-col gap-5">
      <p className="text-xs text-slate-400">
        Choose the topics you want to study and the difficulty to be quizzed at.
        Your daily challenge draws only from selected topics.
      </p>

      {grouped.map(([area, items]) => (
        <div key={area}>
          <h3 className="mb-2 text-xs font-semibold uppercase tracking-wider text-slate-400">
            {area.replace(/-/g, " ")}
          </h3>
          <div className="flex flex-col gap-2">
            {items.map((t) => {
              const pct =
                t.answered > 0 ? Math.round((t.correct / t.answered) * 100) : 0;
              return (
                <div
                  key={t.slug}
                  className={`rounded-xl border p-3 transition ${
                    t.enabled
                      ? "border-white/10 bg-panel/60"
                      : "border-white/5 bg-white/[0.02] opacity-60"
                  }`}
                >
                  <div className="flex items-start gap-3">
                    <input
                      type="checkbox"
                      checked={t.enabled}
                      onChange={() => toggle(t)}
                      className="mt-1 h-4 w-4 accent-[#6ea8fe]"
                    />
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center justify-between gap-2">
                        <span className="font-medium">{t.title}</span>
                        {t.answered > 0 && (
                          <span className="shrink-0 text-[11px] text-slate-400">
                            {t.correct}/{t.answered} · {pct}%
                          </span>
                        )}
                      </div>
                      <p className="mt-0.5 text-xs text-slate-400">{t.summary}</p>

                      {/* difficulty selector */}
                      <div className="mt-2 flex flex-wrap gap-1.5">
                        {DIFFICULTIES.map((d) => (
                          <button
                            key={d}
                            onClick={() => setDiff(t, d)}
                            className={`rounded-md border px-2 py-0.5 text-[11px] capitalize transition ${
                              t.target_difficulty === d
                                ? "border-accent/60 bg-accent/15 text-accent"
                                : "border-white/10 bg-white/5 text-slate-400 hover:text-slate-200"
                            }`}
                          >
                            {d}
                          </button>
                        ))}
                      </div>

                      {/* per-tier grades */}
                      {t.by_difficulty.length > 0 && (
                        <div className="mt-2 flex flex-wrap gap-x-3 gap-y-1 text-[11px]">
                          {t.by_difficulty.map((s) => (
                            <span key={s.difficulty} className={DIFF_COLOR[s.difficulty]}>
                              <span className="capitalize">{s.difficulty}</span>{" "}
                              {s.correct}/{s.total}
                            </span>
                          ))}
                        </div>
                      )}
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      ))}
    </div>
  );
}
