import { useEffect, useState } from "react";
import type {
  Level,
  ModelInfo,
  PullProgress,
  RecommendedModel,
  Settings as SettingsT,
} from "../types";
import {
  checkOllamaHealth,
  deleteModel,
  getSettings,
  listInstalledModels,
  listOllamaModels,
  pullModel,
  recommendedModels,
  setDailySchedule,
  setGlobalShortcut,
  updateSettings,
} from "../lib/ipc";

const LEVELS: Level[] = ["senior", "staff", "principal"];

function formatSize(bytes: number): string {
  if (bytes <= 0) return "";
  const gb = bytes / 1e9;
  return gb >= 1 ? `${gb.toFixed(1)} GB` : `${Math.round(bytes / 1e6)} MB`;
}

export default function Settings() {
  const [settings, setSettings] = useState<SettingsT | null>(null);
  const [models, setModels] = useState<string[]>([]);
  const [healthy, setHealthy] = useState<boolean | null>(null);
  const [saved, setSaved] = useState(false);

  function refreshModels() {
    listOllamaModels()
      .then(setModels)
      .catch(() => setModels([]));
  }

  useEffect(() => {
    getSettings().then(setSettings);
    checkOllamaHealth()
      .then(setHealthy)
      .then(() => refreshModels());
  }, []);

  if (!settings) return <p className="text-sm text-slate-400">Loading…</p>;

  function patch<K extends keyof SettingsT>(key: K, value: SettingsT[K]) {
    setSettings((prev) => (prev ? { ...prev, [key]: value } : prev));
    setSaved(false);
  }

  async function save() {
    if (!settings) return;
    await updateSettings(settings);
    await setDailySchedule(settings.schedule_hour, settings.reminders_enabled);
    await setGlobalShortcut(settings.global_shortcut);
    setSaved(true);
  }

  return (
    <div className="flex flex-col gap-4 text-sm">
      <Field label="Ollama status">
        <span
          className={`inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-xs ${
            healthy
              ? "bg-emerald-500/20 text-emerald-200"
              : "bg-rose-500/20 text-rose-200"
          }`}
        >
          <span className={`h-2 w-2 rounded-full ${healthy ? "bg-emerald-400" : "bg-rose-400"}`} />
          {healthy === null ? "Checking…" : healthy ? "Connected" : "Not reachable"}
        </span>
      </Field>

      <Field label="Seniority level">
        <div className="flex gap-2">
          {LEVELS.map((l) => (
            <button
              key={l}
              onClick={() => patch("level", l)}
              className={`flex-1 rounded-lg border px-3 py-1.5 text-xs capitalize transition ${
                settings.level === l
                  ? "border-accent/60 bg-accent/15 text-accent"
                  : "border-white/10 bg-white/5 text-slate-300"
              }`}
            >
              {l}
            </button>
          ))}
        </div>
      </Field>

      <Field label="Chat model (explanations, grading, repo Q&A)">
        <ModelSelect
          value={settings.chat_model}
          models={models}
          onChange={(v) => patch("chat_model", v)}
        />
      </Field>

      <Field label="MCQ model (fast daily question generation)">
        <ModelSelect
          value={settings.mcq_model}
          models={models}
          onChange={(v) => patch("mcq_model", v)}
        />
      </Field>

      <Field label="Embedding model">
        <ModelSelect
          value={settings.embed_model}
          models={models}
          onChange={(v) => patch("embed_model", v)}
        />
      </Field>

      <Field label="Ollama URL">
        <input
          value={settings.ollama_url}
          onChange={(e) => patch("ollama_url", e.target.value)}
          className="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-2 outline-none focus:border-accent/60"
        />
      </Field>

      <Field label="Daily popup hour (0–23)">
        <input
          type="number"
          min={0}
          max={23}
          value={settings.schedule_hour}
          onChange={(e) => patch("schedule_hour", Number(e.target.value))}
          className="w-24 rounded-lg border border-white/10 bg-white/5 px-3 py-2 outline-none focus:border-accent/60"
        />
      </Field>

      <Field label="Daily reminder notifications">
        <label className="flex items-center gap-2 text-xs text-slate-300">
          <input
            type="checkbox"
            checked={settings.reminders_enabled}
            onChange={(e) => patch("reminders_enabled", e.target.checked)}
            className="h-4 w-4 accent-[#6ea8fe]"
          />
          Notify me if I haven't done today's challenge
        </label>
      </Field>

      <Field label="Global shortcut">
        <input
          value={settings.global_shortcut}
          onChange={(e) => patch("global_shortcut", e.target.value)}
          placeholder="CmdOrCtrl+Shift+K"
          className="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-2 outline-none focus:border-accent/60"
        />
        <p className="mt-1 text-[11px] text-slate-500">
          Tauri accelerator syntax, e.g. <code>CmdOrCtrl+Shift+K</code>. Summons
          the popup from anywhere.
        </p>
      </Field>

      <button
        onClick={save}
        className="mt-1 rounded-lg bg-accent/20 px-4 py-2 font-medium text-accent transition hover:bg-accent/30"
      >
        {saved ? "Saved ✓" : "Save settings"}
      </button>

      <ModelManager onChanged={refreshModels} />
    </div>
  );
}

function ModelManager({ onChanged }: { onChanged?: () => void }) {
  const [installed, setInstalled] = useState<ModelInfo[]>([]);
  const [recommended, setRecommended] = useState<RecommendedModel[]>([]);
  const [progress, setProgress] = useState<Record<string, PullProgress>>({});
  const [error, setError] = useState<string | null>(null);

  function refresh() {
    listInstalledModels()
      .then(setInstalled)
      .catch(() => setInstalled([]));
    onChanged?.();
  }

  useEffect(() => {
    refresh();
    recommendedModels().then(setRecommended).catch(() => setRecommended([]));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const installedNames = new Set(installed.map((m) => m.name));

  async function pull(name: string) {
    setError(null);
    try {
      await pullModel(name, (p) =>
        setProgress((prev) => ({ ...prev, [name]: p })),
      );
      setProgress((prev) => {
        const next = { ...prev };
        delete next[name];
        return next;
      });
      refresh();
    } catch (e) {
      setError(String(e));
    }
  }

  async function remove(name: string) {
    await deleteModel(name).catch((e) => setError(String(e)));
    refresh();
  }

  return (
    <div className="mt-3 border-t border-white/10 pt-3">
      <h3 className="mb-2 text-xs font-semibold uppercase tracking-wider text-slate-400">
        Model manager
      </h3>

      {installed.length > 0 && (
        <div className="mb-3 flex flex-col gap-1.5">
          {installed.map((m) => (
            <div
              key={m.name}
              className="flex items-center justify-between rounded-lg border border-white/10 bg-white/5 px-3 py-1.5 text-xs"
            >
              <span className="font-medium text-slate-200">{m.name}</span>
              <div className="flex items-center gap-2">
                <span className="text-slate-500">{formatSize(m.size_bytes)}</span>
                <button
                  onClick={() => remove(m.name)}
                  className="text-slate-500 hover:text-rose-300"
                >
                  ✕
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      <p className="mb-1.5 text-[11px] uppercase tracking-wider text-slate-500">
        Recommended
      </p>
      <div className="flex flex-col gap-1.5">
        {recommended.map((r) => {
          const p = progress[r.name];
          const have = installedNames.has(r.name);
          return (
            <div
              key={r.name}
              className="rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-xs"
            >
              <div className="flex items-center justify-between">
                <div>
                  <span className="font-medium text-slate-200">{r.name}</span>
                  <span className="ml-2 text-slate-500">{r.purpose}</span>
                </div>
                {have ? (
                  <span className="text-emerald-300">✓ installed</span>
                ) : p ? (
                  <span className="text-amber-300">
                    {p.total > 0
                      ? `${Math.round((p.completed / p.total) * 100)}%`
                      : p.status}
                  </span>
                ) : (
                  <button
                    onClick={() => pull(r.name)}
                    className="rounded-md bg-accent/20 px-2 py-0.5 font-medium text-accent hover:bg-accent/30"
                  >
                    Pull
                  </button>
                )}
              </div>
              <p className="mt-0.5 text-slate-500">{r.note}</p>
              {p && p.total > 0 && (
                <div className="mt-1 h-1 overflow-hidden rounded-full bg-white/10">
                  <div
                    className="h-full bg-accent"
                    style={{ width: `${(p.completed / p.total) * 100}%` }}
                  />
                </div>
              )}
            </div>
          );
        })}
      </div>
      {error && <p className="mt-2 text-[11px] text-rose-300">{error}</p>}
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div>
      <label className="mb-1.5 block text-xs font-semibold uppercase tracking-wider text-slate-400">
        {label}
      </label>
      {children}
    </div>
  );
}

// Thuki-style dropdown: pick from the models actually installed in Ollama.
function ModelSelect({
  value,
  models,
  onChange,
}: {
  value: string;
  models: string[];
  onChange: (v: string) => void;
}) {
  const notInstalled = !!value && !models.includes(value);
  return (
    <>
      <select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-2 outline-none focus:border-accent/60"
      >
        {!value && <option value="">Select a model…</option>}
        {notInstalled && <option value={value}>{value} (not installed)</option>}
        {models.map((m) => (
          <option key={m} value={m}>
            {m}
          </option>
        ))}
      </select>
      {models.length === 0 ? (
        <p className="mt-1 text-[11px] text-amber-300">
          No models installed. Pull one from the model manager below.
        </p>
      ) : notInstalled ? (
        <p className="mt-1 text-[11px] text-amber-300">
          “{value}” isn’t installed — pick an installed model or pull it below to
          avoid “model not found” errors.
        </p>
      ) : null}
    </>
  );
}
