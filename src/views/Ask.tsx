import { useState } from "react";
import ReactMarkdown from "react-markdown";
import { open } from "@tauri-apps/plugin-shell";
import type { SearchResult } from "../types";
import { askWeb, fetchUrl, webSearch } from "../lib/ipc";

export default function Ask() {
  const [input, setInput] = useState("");
  const [answer, setAnswer] = useState("");
  const [sources, setSources] = useState<SearchResult[]>([]);
  const [status, setStatus] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function stream(question: string, context: string, srcUrls: string[]) {
    return askWeb(question, context, srcUrls, (chunk) => {
      if (chunk.kind === "token") setAnswer((p) => p + chunk.text);
      else if (chunk.kind === "error") setError(chunk.message);
    });
  }

  async function run() {
    const text = input.trim();
    if (!text || busy) return;
    setBusy(true);
    setError(null);
    setAnswer("");
    setSources([]);
    setStatus(null);
    try {
      if (text.startsWith("/url")) {
        const rest = text.slice(4).trim();
        const [url, ...q] = rest.split(/\s+/);
        if (!url) throw new Error("Usage: /url <address> [question]");
        setStatus(`Fetching ${url}…`);
        const page = await fetchUrl(url);
        setSources([{ title: page.title, url: page.url, snippet: "" }]);
        setStatus(null);
        await stream(q.join(" ") || "Summarize this page.", page.text, [page.url]);
      } else if (text.startsWith("/search")) {
        const query = text.slice(7).trim();
        if (!query) throw new Error("Usage: /search <query>");
        setStatus(`Searching “${query}”…`);
        const results = await webSearch(query);
        setSources(results.slice(0, 5));
        if (results.length === 0) {
          setStatus("No results found.");
          return;
        }
        const top = results.slice(0, 3);
        setStatus("Reading top results…");
        const pages = await Promise.allSettled(top.map((r) => fetchUrl(r.url)));
        const context = pages
          .flatMap((p) =>
            p.status === "fulfilled"
              ? [`# ${p.value.title} (${p.value.url})\n${p.value.text}`]
              : [],
          )
          .join("\n\n");
        setStatus(null);
        await stream(
          query,
          context,
          top.map((r) => r.url),
        );
      } else {
        await stream(text, "", []);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
      setStatus(null);
    }
  }

  return (
    <div className="flex flex-col gap-4">
      <div>
        <h2 className="font-display text-lg font-semibold">Ask the web</h2>
        <p className="mt-0.5 text-xs text-neutral-500">
          Use <code className="rounded bg-black/[0.06] px-1">/search query</code> to
          search the web or{" "}
          <code className="rounded bg-black/[0.06] px-1">/url address</code> to fetch
          a page, then ask about it — answered by your local model. Plain text asks
          the model directly.
        </p>
      </div>

      <div className="flex gap-2">
        <input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && run()}
          placeholder="/search rust async runtimes   ·   /url docs.rs/tokio   ·   or ask anything"
          className="flex-1 rounded-lg border border-black/[0.08] bg-white px-3 py-2 text-sm outline-none focus:border-accent/60"
        />
        <button
          onClick={run}
          disabled={busy}
          className="rounded-lg bg-accent px-4 py-2 text-sm font-medium text-white transition hover:opacity-90 disabled:opacity-50"
        >
          {busy ? "…" : "Go"}
        </button>
      </div>

      {status && <p className="text-xs text-neutral-500">{status}</p>}

      {sources.length > 0 && (
        <div className="flex flex-col gap-1">
          <p className="text-[11px] font-semibold uppercase tracking-wider text-neutral-400">
            Sources
          </p>
          {sources.map((s, i) => (
            <button
              key={i}
              onClick={() => open(s.url).catch(() => {})}
              className="truncate text-left text-xs text-accent hover:underline"
              title={s.url}
            >
              {s.title || s.url}
            </button>
          ))}
        </div>
      )}

      {(answer || (busy && !status)) && (
        <div className="md rounded-xl border border-black/[0.08] bg-white p-4 text-sm text-neutral-800">
          <ReactMarkdown>{answer || "Thinking…"}</ReactMarkdown>
        </div>
      )}

      {error && <p className="text-xs text-rose-600">{error}</p>}
    </div>
  );
}
