import { useEffect, useState } from "react";
import type { Level, Settings as SettingsT } from "../types";
import {
  checkOllamaHealth,
  getSettings,
  listOllamaModels,
  setDailySchedule,
  updateSettings,
} from "../lib/ipc";

const LEVELS: Level[] = ["senior", "staff", "principal"];

export default function Settings() {
  const [settings, setSettings] = useState<SettingsT | null>(null);
  const [models, setModels] = useState<string[]>([]);
  const [healthy, setHealthy] = useState<boolean | null>(null);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    getSettings().then(setSettings);
    checkOllamaHealth()
      .then(setHealthy)
      .then(() => listOllamaModels().then(setModels).catch(() => setModels([])));
  }, []);

  if (!settings) return <p className="text-sm text-slate-400">Loading…</p>;

  function patch<K extends keyof SettingsT>(key: K, value: SettingsT[K]) {
    setSettings((prev) => (prev ? { ...prev, [key]: value } : prev));
    setSaved(false);
  }

  async function save() {
    if (!settings) return;
    await updateSettings(settings);
    await setDailySchedule(settings.schedule_hour, true);
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

      <Field label="Chat model">
        <ModelInput
          value={settings.chat_model}
          onChange={(v) => patch("chat_model", v)}
        />
      </Field>

      <Field label="Embedding model">
        <ModelInput
          value={settings.embed_model}
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

      <button
        onClick={save}
        className="mt-2 rounded-lg bg-accent/20 px-4 py-2 font-medium text-accent transition hover:bg-accent/30"
      >
        {saved ? "Saved ✓" : "Save settings"}
      </button>

      {/* Autocomplete source for the model inputs. */}
      <datalist id="ollama-models">
        {models.map((m) => (
          <option key={m} value={m} />
        ))}
      </datalist>
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

function ModelInput({
  value,
  onChange,
}: {
  value: string;
  onChange: (v: string) => void;
}) {
  // Autocomplete options come from the shared <datalist id="ollama-models">.
  return (
    <input
      value={value}
      list="ollama-models"
      onChange={(e) => onChange(e.target.value)}
      className="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-2 outline-none focus:border-accent/60"
      placeholder="e.g. llama3.1:8b"
    />
  );
}
