import { useEffect, useState } from "react";
import ReactMarkdown from "react-markdown";
import type { ProgressStats } from "../types";
import { generateWeaknessReport, getProgress } from "../lib/ipc";

export default function Progress() {
  const [stats, setStats] = useState<ProgressStats | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [report, setReport] = useState("");
  const [reporting, setReporting] = useState(false);

  useEffect(() => {
    getProgress().then(setStats).catch((e) => setError(String(e)));
  }, []);

  async function runReport() {
    if (reporting) return;
    setReporting(true);
    setReport("");
    try {
      await generateWeaknessReport((chunk) => {
        if (chunk.kind === "token") setReport((p) => p + chunk.text);
        else if (chunk.kind === "error") setError(chunk.message);
      });
    } catch (e) {
      setError(String(e));
    } finally {
      setReporting(false);
    }
  }

  if (error) return <p className="text-sm text-rose-600">{error}</p>;
  if (!stats) return <p className="text-sm text-neutral-500">Loading…</p>;

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
        <h3 className="mb-2 text-xs font-semibold uppercase tracking-wider text-neutral-500">
          Accuracy by area
        </h3>
        {stats.by_area.length === 0 ? (
          <p className="text-sm text-neutral-400">
            Answer some questions to see your strengths by area.
          </p>
        ) : (
          <div className="flex flex-col gap-2">
            {stats.by_area.map((a) => {
              const pct = a.total > 0 ? Math.round((a.correct / a.total) * 100) : 0;
              return (
                <div key={a.area}>
                  <div className="mb-1 flex justify-between text-xs text-neutral-600">
                    <span className="capitalize">{a.area.replace(/-/g, " ")}</span>
                    <span className="text-neutral-500">
                      {a.correct}/{a.total} · {pct}%
                    </span>
                  </div>
                  <div className="h-2 overflow-hidden rounded-full bg-black/[0.06]">
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

      {/* AI weakness report + study plan */}
      <div className="border-t border-black/[0.08] pt-3">
        <button
          onClick={runReport}
          disabled={reporting}
          className="rounded-lg bg-accent/20 px-4 py-2 text-sm font-medium text-accent transition hover:bg-accent/30 disabled:opacity-50"
        >
          {reporting ? "Analyzing…" : "✨ Weakness report + study plan"}
        </button>
        {(report || reporting) && (
          <div className="md mt-3 max-w-none rounded-lg bg-white p-3 text-sm text-neutral-800">
            <ReactMarkdown>{report || "Analyzing your progress…"}</ReactMarkdown>
          </div>
        )}
      </div>
    </div>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-xl border border-black/[0.08] bg-white p-3 text-center">
      <p className="text-lg font-semibold text-neutral-900">{value}</p>
      <p className="mt-0.5 text-[11px] text-neutral-500">{label}</p>
    </div>
  );
}
