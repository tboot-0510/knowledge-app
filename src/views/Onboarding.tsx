import { useEffect, useMemo, useState } from "react";
import type { Level, TopicCard } from "../types";
import { getSettings, listTopics, setTopicPref, updateSettings } from "../lib/ipc";

const LEVELS: { id: Level; label: string; blurb: string }[] = [
  { id: "senior", label: "Senior", blurb: "Deep applied fundamentals" },
  { id: "staff", label: "Staff", blurb: "Cross-team trade-offs" },
  { id: "principal", label: "Principal", blurb: "Org-wide strategy" },
];

export default function Onboarding({ onDone }: { onDone: () => void }) {
  const [topics, setTopics] = useState<TopicCard[]>([]);
  const [level, setLevel] = useState<Level>("senior");
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    listTopics()
      .then((t) => {
        setTopics(t);
        // Preselect whatever is already enabled (defaults to all on a fresh DB).
        setSelected(new Set(t.filter((x) => x.enabled).map((x) => x.slug)));
      })
      .catch(() => setTopics([]));
  }, []);

  const grouped = useMemo(() => {
    const map: Record<string, TopicCard[]> = {};
    for (const t of topics) (map[t.area] ??= []).push(t);
    return Object.entries(map).sort(([a], [b]) => a.localeCompare(b));
  }, [topics]);

  function toggle(slug: string) {
    setSelected((prev) => {
      const next = new Set(prev);
      next.has(slug) ? next.delete(slug) : next.add(slug);
      return next;
    });
  }

  async function finish() {
    setSaving(true);
    try {
      const settings = await getSettings();
      await updateSettings({ ...settings, level, onboarded: true });
      // Enable only the chosen topics so the daily challenge draws from interests.
      await Promise.all(
        topics.map((t) =>
          setTopicPref(t.slug, selected.has(t.slug), t.target_difficulty),
        ),
      );
      onDone();
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="flex h-full flex-col bg-paper/95 text-ink backdrop-blur-xl">
      <div className="min-h-0 flex-1 overflow-y-auto px-6 py-8">
        <div className="mx-auto max-w-md">
          <p className="text-xs uppercase tracking-[0.2em] text-accent">Welcome</p>
          <h1 className="mt-1 font-display text-3xl font-semibold leading-tight">
            Let's tailor your daily practice.
          </h1>
          <p className="mt-2 text-sm text-neutral-500">
            Pick your level and the topics you care about. You can change these
            anytime in Topics &amp; Settings.
          </p>

          {/* Level */}
          <h2 className="mt-7 text-xs font-semibold uppercase tracking-wider text-neutral-400">
            Your level
          </h2>
          <div className="mt-2 grid grid-cols-3 gap-2">
            {LEVELS.map((l) => (
              <button
                key={l.id}
                onClick={() => setLevel(l.id)}
                className={`rounded-xl border p-3 text-left transition ${
                  level === l.id
                    ? "border-accent bg-accent/10"
                    : "border-black/[0.08] bg-white hover:border-black/20"
                }`}
              >
                <div className="text-sm font-medium">{l.label}</div>
                <div className="mt-0.5 text-[11px] text-neutral-500">{l.blurb}</div>
              </button>
            ))}
          </div>

          {/* Topics */}
          <h2 className="mt-7 text-xs font-semibold uppercase tracking-wider text-neutral-400">
            Topics you're interested in
          </h2>
          <div className="mt-2 flex flex-col gap-4">
            {grouped.map(([area, items]) => (
              <div key={area}>
                <p className="mb-1.5 text-[11px] capitalize text-neutral-500">
                  {area.replace(/-/g, " ")}
                </p>
                <div className="flex flex-wrap gap-1.5">
                  {items.map((t) => {
                    const on = selected.has(t.slug);
                    return (
                      <button
                        key={t.slug}
                        onClick={() => toggle(t.slug)}
                        className={`rounded-full border px-3 py-1.5 text-xs transition ${
                          on
                            ? "border-accent bg-accent/10 text-accent"
                            : "border-black/[0.08] bg-white text-neutral-600 hover:border-black/20"
                        }`}
                      >
                        {t.title}
                      </button>
                    );
                  })}
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Footer CTA */}
      <div className="border-t border-black/[0.06] px-6 py-3">
        <div className="mx-auto flex max-w-md items-center justify-between">
          <span className="text-xs text-neutral-500">
            {selected.size} topic{selected.size === 1 ? "" : "s"} selected
          </span>
          <button
            onClick={finish}
            disabled={saving || selected.size === 0}
            className="rounded-full bg-accent px-5 py-2 text-sm font-medium text-white transition hover:opacity-90 disabled:opacity-50"
          >
            {saving ? "Setting up…" : "Get started"}
          </button>
        </div>
      </div>
    </div>
  );
}
