# Knowledge — daily engineering learning + local‑LLM repo Q&A

A macOS desktop app, in the spirit of [thuki](https://github.com/quiet-node/thuki),
that does two things for senior / staff / principal engineers:

1. **Daily challenge popup** — each day it surfaces a topic and a set of
   **big‑tech‑interview‑style multiple‑choice questions**, tracks your **streak**
   and accuracy, and scales difficulty to your seniority.
2. **Topics panel** — pick the areas you want to study (blockchain, EVM, smart
   contract security, system design, transport protocols, operating systems,
   DS&A coding, distributed systems, concurrency, databases, reliability,
   security, performance, leadership…), choose a target **difficulty tier**
   (easy / medium / hard / advanced) per topic, and track your grades per topic
   and per tier. The daily challenge draws only from your selected topics.
3. **Follow‑ups** — after answering any question you can ask the local model to
   **explain in depth**, pose a harder **interview‑style follow‑up**, or answer
   **your own question** about the concept — all streamed locally.
4. **Repo Q&A** — link a **GitHub repository by URL**; it’s cloned and indexed
   **locally**, and you can ask questions about the codebase with answers that
   cite `file:line` ranges.

**Everything runs locally.** All inference (chat + embeddings) goes through a
local [Ollama](https://ollama.com) server — no cloud APIs, no data leaving your
machine.

---

## Architecture

| Layer | Tech | Notes |
|---|---|---|
| Shell | **Tauri v2** | NSPanel overlay, Accessory (dock‑hidden) activation, tray menu |
| Core logic | **Rust** (`crates/core`, no Tauri deps) | DB, Ollama client, MCQ generation/validation, RAG, scoring — **fully unit‑tested on any OS** |
| App / IPC | **Rust** (`src-tauri`) | thin command layer + macOS platform code |
| Frontend | **React 18 + TypeScript + Vite + Tailwind** | daily challenge, topics, repo chat, progress, settings |
| Storage | **SQLite** (`rusqlite`, bundled) | topics, sessions, attempts, streaks, repos, chunks, embeddings |
| Vectors | f32 BLOBs + in‑Rust cosine search | no native extension required; `sqlite-vec` is a future drop‑in |
| LLM | **Ollama** at `127.0.0.1:11434` | chat: `llama3.1:8b` (default), embeddings: `nomic-embed-text` |

```
knowledge-app/
├── crates/core/        # knowledge-core: platform-agnostic logic (testable everywhere)
│   ├── src/            #   db, ollama, learning/{catalog,generator,scoring,fallback}, repo/{chunker,clone,index,search}, scheduler
│   └── migrations/     #   embedded SQL (user_version migrations)
├── src-tauri/          # Tauri binary: commands/, platform/{macos,stub}, lib.rs
├── src/                # React frontend (views/, components/, lib/ipc.ts, types.ts)
└── resources/          # catalog.json (curated topics) + seed_questions.json (offline fallback bank)
```

### How daily content works (hybrid)

- A **curated catalog** (`resources/catalog.json`) defines high‑signal topics per
  seniority level. Each day a topic is chosen (recent topics are avoided).
- The local model is asked to generate fresh MCQs for that topic as **strict JSON**,
  which is parsed and **validated** (4 choices, valid correct index, non‑empty
  explanation). Invalid output is rejected.
- If the model is unavailable or returns invalid JSON, the app falls back to a
  **bundled, pre‑validated seed bank** (`resources/seed_questions.json`) so the
  daily challenge always works offline.

### How repo Q&A works

`link_repo(url)` → shallow `git clone` into the app data dir → walk files honoring
`.gitignore` (skipping binaries, lockfiles, `node_modules`, etc.) → split into
overlapping line‑window chunks → embed each chunk via Ollama → store vectors in
SQLite. Asking a question embeds the question, runs cosine top‑k retrieval, and
streams a grounded answer that cites the source files.

---

## Prerequisites

- **macOS** (the shippable app is macOS; see the build note below).
- **[Ollama](https://ollama.com)** installed and running, with the models pulled:
  ```bash
  ollama serve            # if not already running
  ollama pull llama3.1:8b      # chat / question generation (configurable)
  ollama pull nomic-embed-text # embeddings for repo Q&A
  ```
- **Rust** (stable) + **[Bun](https://bun.sh)** (or npm) for the frontend.
- Xcode command line tools (`xcode-select --install`).

## Build & run (macOS)

```bash
bun install
bun run tauri dev      # develop with hot reload
bun run tauri build    # produce a .app / .dmg in src-tauri/target/release/bundle
```

The default chat/embedding models, seniority level, daily popup hour, and Ollama
URL are all editable in‑app under **Settings**.

### Daily scheduling

The app installs a launchd LaunchAgent (and supports launch‑at‑login via
`tauri-plugin-autostart`) to open at your chosen hour; an in‑app once‑per‑day
gate (`scheduler::should_show`) ensures the popup appears only once per day.

---

## Development & verification

The core logic lives in a separate crate with **no Tauri/UI dependencies**, so it
compiles and tests on any platform:

```bash
cargo test -p knowledge-core     # 40+ unit tests: MCQ parse/validate, scoring/streaks,
                                 # chunking, cosine retrieval, scheduler dates, DB layer
cargo clippy -p knowledge-core
bun run typecheck                # tsc --noEmit
bun run build                    # vite production build
```

> **Note on building the Tauri binary off macOS:** the `src-tauri` crate links
> macOS/desktop system libraries (and, on macOS, NSPanel via `tauri-nspanel`).
> On a Linux box without the GTK/WebKit dev packages, `cargo check -p knowledge-app`
> fails at `pkg-config` for `gdk-3.0`/`webkit2gtk` — this is a missing‑system‑lib
> issue, **not** an application code error. Build the app on macOS, or install
> `libwebkit2gtk-4.1-dev libgtk-3-dev` on Linux to compile the shell there.

### Manual smoke test (on a Mac)

1. `ollama serve` and pull the two models above.
2. `bun run tauri dev`.
3. **Today** tab → answer the daily questions → confirm the score + streak update.
4. **Repos** tab → paste a small public repo URL → watch it index → ask a question
   and confirm a streamed, file‑citing answer.
5. **Settings** → switch seniority level / models → Save.

---

## License

Apache‑2.0.
