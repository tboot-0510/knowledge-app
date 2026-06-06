import { useEffect, useRef, useState } from "react";
import ReactMarkdown from "react-markdown";
import type { IndexProgress, Repo } from "../types";
import {
  askRepo,
  deleteRepo,
  linkRepo,
  listRepos,
  onIndexProgress,
} from "../lib/ipc";

export default function RepoChat() {
  const [repos, setRepos] = useState<Repo[]>([]);
  const [url, setUrl] = useState("");
  const [linking, setLinking] = useState(false);
  const [progress, setProgress] = useState<Record<number, IndexProgress>>({});
  const [selected, setSelected] = useState<number | null>(null);
  const [question, setQuestion] = useState("");
  const [answer, setAnswer] = useState("");
  const [asking, setAsking] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const answerRef = useRef<HTMLDivElement>(null);

  async function refresh() {
    setRepos(await listRepos());
  }

  useEffect(() => {
    refresh();
    const unlisten = onIndexProgress((p) => {
      setProgress((prev) => ({ ...prev, [p.repo_id]: p }));
      if (p.done >= p.total) refresh();
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  async function handleLink() {
    if (!url.trim()) return;
    setLinking(true);
    setError(null);
    try {
      await linkRepo(url.trim());
      setUrl("");
      await refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setLinking(false);
    }
  }

  async function handleAsk() {
    if (selected === null || !question.trim() || asking) return;
    setAsking(true);
    setAnswer("");
    setError(null);
    try {
      await askRepo(selected, question.trim(), (chunk) => {
        if (chunk.kind === "token") {
          setAnswer((prev) => prev + chunk.text);
          answerRef.current?.scrollTo(0, answerRef.current.scrollHeight);
        } else if (chunk.kind === "error") {
          setError(chunk.message);
        }
      });
    } catch (e) {
      setError(String(e));
    } finally {
      setAsking(false);
    }
  }

  const ready = repos.filter((r) => r.status === "ready");

  return (
    <div className="flex flex-col gap-4">
      {/* Link a new repo */}
      <div className="flex gap-2">
        <input
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          placeholder="https://github.com/owner/repo"
          className="flex-1 rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm outline-none focus:border-accent/60"
          onKeyDown={(e) => e.key === "Enter" && handleLink()}
        />
        <button
          onClick={handleLink}
          disabled={linking}
          className="rounded-lg bg-accent/20 px-3 py-2 text-sm font-medium text-accent transition hover:bg-accent/30 disabled:opacity-50"
        >
          {linking ? "Linking…" : "Link"}
        </button>
      </div>

      {/* Repo list */}
      <div className="flex flex-col gap-2">
        {repos.length === 0 && (
          <p className="text-sm text-slate-500">
            Link a public GitHub repo to index it locally and ask questions about it.
          </p>
        )}
        {repos.map((r) => {
          const p = progress[r.id];
          const indexing = r.status === "indexing" || r.status === "cloning";
          return (
            <div
              key={r.id}
              onClick={() => r.status === "ready" && setSelected(r.id)}
              className={`cursor-pointer rounded-lg border px-3 py-2 text-sm transition ${
                selected === r.id
                  ? "border-accent/60 bg-accent/10"
                  : "border-white/10 bg-white/5 hover:border-white/20"
              }`}
            >
              <div className="flex items-center justify-between">
                <span className="font-medium">{r.name}</span>
                <div className="flex items-center gap-2">
                  <StatusPill status={r.status} />
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      deleteRepo(r.id).then(refresh);
                    }}
                    className="text-xs text-slate-500 hover:text-rose-300"
                  >
                    ✕
                  </button>
                </div>
              </div>
              {indexing && p && (
                <div className="mt-1.5 h-1.5 overflow-hidden rounded-full bg-white/10">
                  <div
                    className="h-full bg-accent transition-all"
                    style={{ width: `${p.total ? (p.done / p.total) * 100 : 0}%` }}
                  />
                </div>
              )}
              {r.status === "ready" && (
                <p className="mt-1 text-[11px] text-slate-500">
                  {r.file_count} files · {r.chunk_count} chunks indexed
                </p>
              )}
              {r.status === "error" && r.error && (
                <p className="mt-1 text-[11px] text-rose-300">{r.error}</p>
              )}
            </div>
          );
        })}
      </div>

      {/* Q&A */}
      {ready.length > 0 && (
        <div className="flex flex-col gap-2 border-t border-white/10 pt-4">
          <div className="flex gap-2">
            <input
              value={question}
              onChange={(e) => setQuestion(e.target.value)}
              placeholder={
                selected === null
                  ? "Select a ready repo above…"
                  : "Ask about this codebase…"
              }
              disabled={selected === null}
              onKeyDown={(e) => e.key === "Enter" && handleAsk()}
              className="flex-1 rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm outline-none focus:border-accent/60 disabled:opacity-50"
            />
            <button
              onClick={handleAsk}
              disabled={asking || selected === null}
              className="rounded-lg bg-accent/20 px-3 py-2 text-sm font-medium text-accent transition hover:bg-accent/30 disabled:opacity-50"
            >
              {asking ? "…" : "Ask"}
            </button>
          </div>
          {(answer || asking) && (
            <div
              ref={answerRef}
              className="prose prose-invert max-h-72 max-w-none overflow-y-auto rounded-lg bg-panel/60 p-3 text-sm text-slate-200"
            >
              <ReactMarkdown>{answer || "Thinking…"}</ReactMarkdown>
            </div>
          )}
        </div>
      )}

      {error && <p className="text-xs text-rose-300">{error}</p>}
    </div>
  );
}

function StatusPill({ status }: { status: Repo["status"] }) {
  const map: Record<Repo["status"], string> = {
    pending: "bg-slate-500/20 text-slate-300",
    cloning: "bg-amber-500/20 text-amber-200",
    indexing: "bg-amber-500/20 text-amber-200",
    ready: "bg-emerald-500/20 text-emerald-200",
    error: "bg-rose-500/20 text-rose-200",
  };
  return (
    <span className={`rounded-full px-2 py-0.5 text-[10px] ${map[status]}`}>
      {status}
    </span>
  );
}
