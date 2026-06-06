import { useEffect, useState } from "react";
import type { ProgressStats } from "../types";
import { getProgress } from "../lib/ipc";

export default function Progress() {
  const [stats, setStats] = useState<ProgressStats | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getProgress().then(setStats).catch((e) => setError(String(e)));
  }, []);

  if (error) return <p className="text-sm text-rose-300">{error}</p>;
  if (!stats) return <p className="text-sm text-slate-400">Loading…</p>;

  const accuracy =
    stats.total_questions > 0
      ? Math.round((stats.total_correct / stats.total_questions) * 100)
      : 0;

  return (
    <div className="flex flex-col gap-4">
      <div className="grid grid-cols-3 gap-3">
        <Stat label="Current streak" value={`${stats.streak.current_streak}🔥`} />
        <Stat label="Longest streak" value={`${stats.streak.longest_streak}`} />
        <Stat label="Days completed" value={`${stats.days_completed}`} />
        <Stat label="Questions" value={`${stats.total_questions}`} />
        <Stat label="Correct" value={`${stats.total_correct}`} />
        <Stat label="Accuracy" value={`${accuracy}%`} />
      </div>

      <div>
        <h3 className="mb-2 text-xs font-semibold uppercase tracking-wider text-slate-400">
          Accuracy by area
        </h3>
        {stats.by_area.length === 0 ? (
          <p className="text-sm text-slate-500">
            Answer some questions to see your strengths by area.
          </p>
        ) : (
          <div className="flex flex-col gap-2">
            {stats.by_area.map((a) => {
              const pct = a.total > 0 ? Math.round((a.correct / a.total) * 100) : 0;
              return (
                <div key={a.area}>
                  <div className="mb-1 flex justify-between text-xs text-slate-300">
                    <span className="capitalize">{a.area.replace(/-/g, " ")}</span>
                    <span className="text-slate-400">
                      {a.correct}/{a.total} · {pct}%
                    </span>
                  </div>
                  <div className="h-2 overflow-hidden rounded-full bg-white/10">
                    <div
                      className="h-full rounded-full bg-accent"
                      style={{ width: `${pct}%` }}
                    />
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-xl border border-white/10 bg-panel/60 p-3 text-center">
      <p className="text-lg font-semibold text-slate-100">{value}</p>
      <p className="mt-0.5 text-[11px] text-slate-400">{label}</p>
    </div>
  );
}
