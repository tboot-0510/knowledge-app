import { useEffect, useMemo, useState } from "react";
import type { Difficulty, PathCard, TopicCard } from "../types";
import {
  addCustomTopic,
  listPaths,
  listTopics,
  setTopicPref,
} from "../lib/ipc";

const DIFFICULTIES: Difficulty[] = ["easy", "medium", "hard", "advanced"];

const DIFF_COLOR: Record<Difficulty, string> = {
  easy: "text-emerald-700",
  medium: "text-sky-700",
  hard: "text-amber-700",
  advanced: "text-rose-600",
};

export default function Topics() {
  const [topics, setTopics] = useState<TopicCard[]>([]);
  const [paths, setPaths] = useState<PathCard[]>([]);
  const [customName, setCustomName] = useState("");
  const [adding, setAdding] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function refresh() {
    listTopics().then(setTopics).catch((e) => setError(String(e)));
    listPaths().then(setPaths).catch(() => setPaths([]));
  }

  useEffect(() => {
    refresh();
  }, []);

  async function addTopic() {
    if (!customName.trim() || adding) return;
    setAdding(true);
    setError(null);
    try {
      await addCustomTopic(customName.trim());
      setCustomName("");
      refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setAdding(false);
    }
  }

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

  if (error) return <p className="text-sm text-rose-600">{error}</p>;

  return (
    <div className="flex flex-col gap-5">
      <p className="text-xs text-neutral-500">
        Choose the topics you want to study and the difficulty to be quizzed at.
        Your daily challenge draws only from selected topics.
      </p>

      {/* Custom topic synthesis */}
      <div className="flex gap-2">
        <input
          value={customName}
          onChange={(e) => setCustomName(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && addTopic()}
          placeholder="Add a custom topic, e.g. 'Kafka internals'"
          className="flex-1 rounded-lg border border-black/[0.08] bg-black/[0.04] px-3 py-2 text-sm outline-none focus:border-accent/60"
        />
        <button
          onClick={addTopic}
          disabled={adding}
          className="rounded-lg bg-accent/20 px-3 py-2 text-sm font-medium text-accent transition hover:bg-accent/30 disabled:opacity-50"
        >
          {adding ? "Synthesizing…" : "+ Add"}
        </button>
      </div>

      {/* Learning paths */}
      {paths.length > 0 && (
        <div>
          <h3 className="mb-2 text-xs font-semibold uppercase tracking-wider text-neutral-500">
            Learning paths
          </h3>
          <div className="flex flex-col gap-2">
            {paths.map((p) => {
              const pct = p.total > 0 ? Math.round((p.started / p.total) * 100) : 0;
              return (
                <div key={p.slug} className="rounded-xl border border-black/[0.08] bg-white p-3">
                  <div className="flex items-center justify-between">
                    <span className="text-sm font-medium">{p.title}</span>
                    <span className="text-[11px] text-neutral-500">
                      {p.started}/{p.total} started
                    </span>
                  </div>
                  <p className="mt-0.5 text-xs text-neutral-500">{p.description}</p>
                  <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-black/[0.06]">
                    <div className="h-full bg-accent" style={{ width: `${pct}%` }} />
                  </div>
                  <div className="mt-2 flex flex-wrap gap-1">
                    {p.steps.map((s) => (
                      <span
                        key={s.slug}
                        title={`${s.correct}/${s.answered} correct`}
                        className={`rounded px-1.5 py-0.5 text-[10px] ${
                          s.answered > 0
                            ? "bg-emerald-500/15 text-emerald-700"
                            : "bg-black/[0.04] text-neutral-500"
                        }`}
                      >
                        {s.title}
                      </span>
                    ))}
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {grouped.map(([area, items]) => (
        <div key={area}>
          <h3 className="mb-2 text-xs font-semibold uppercase tracking-wider text-neutral-500">
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
                      ? "border-black/[0.08] bg-white"
                      : "border-black/[0.05] bg-black/[0.02] opacity-60"
                  }`}
                >
                  <div className="flex items-start gap-3">
                    <input
                      type="checkbox"
                      checked={t.enabled}
                      onChange={() => toggle(t)}
                      className="mt-1 h-4 w-4 accent-[#4f46e5]"
                    />
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center justify-between gap-2">
                        <span className="font-medium">{t.title}</span>
                        {t.answered > 0 && (
                          <span className="shrink-0 text-[11px] text-neutral-500">
                            {t.correct}/{t.answered} · {pct}%
                          </span>
                        )}
                      </div>
                      <p className="mt-0.5 text-xs text-neutral-500">{t.summary}</p>

                      {/* difficulty selector */}
                      <div className="mt-2 flex flex-wrap gap-1.5">
                        {DIFFICULTIES.map((d) => (
                          <button
                            key={d}
                            onClick={() => setDiff(t, d)}
                            className={`rounded-md border px-2 py-0.5 text-[11px] capitalize transition ${
                              t.target_difficulty === d
                                ? "border-accent/60 bg-accent/15 text-accent"
                                : "border-black/[0.08] bg-black/[0.04] text-neutral-500 hover:text-neutral-800"
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
