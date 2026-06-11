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
3. **Focus mode (Pomodoro)** — pick a duration (5/10/25 min) and a topic, and
   get a continuous, timed stream of questions that **adapt** — harder after each
   correct answer, easier after a miss — to keep you at the edge of your ability.
   Every answer still counts toward progress and spaced repetition.
4. **Follow‑ups** — after answering any question you can ask the local model to
   **explain in depth**, pose a harder **interview‑style follow‑up**, or answer
   **your own question** about the concept — all streamed locally.
4. **Free‑response practice** — open‑ended interview questions you answer in
   prose, **graded by the local LLM** against a rubric (score + feedback).
5. **Spaced repetition** — every answered question is scheduled with **SM‑2** and
   resurfaces in a **Review** queue right before you'd forget it.
6. **Custom topics & learning paths** — synthesize any topic from free text
   (e.g. "Kafka internals") and follow curated **tracks** (Distributed Systems,
   Interview Prep, Web3, Systems & Networking) with progress.
7. **Weakness report** — turn your stats into a local‑LLM diagnosis of weak
   areas plus a one‑week study plan.
8. **Coding practice (LeetCode‑style)** — a **Code** tab with problems across
   every data structure (arrays/hashing, two pointers, sliding window, stacks,
   queues, linked lists, trees, BSTs, heaps, tries, graphs, union‑find,
   backtracking, DP, greedy, binary search, intervals, matrices, bit
   manipulation, math). Pick a category + difficulty, write a solution in the
   editor, and the local LLM reviews **correctness, time/space complexity, edge
   cases, and the optimal approach**. Works offline via a bundled seed bank.
9. **Repo Q&A** — link a **GitHub repository by URL**; it’s cloned and indexed
   **locally**, and you can ask questions about the codebase with answers that
   cite `file:line` ranges.
10. **Web tools (`/search`, `/url`)** — in the **Ask** tab, `/search <query>`
    runs a web search and reads the top results, `/url <address>` fetches a page,
    and your local model answers with cited sources. Plain text asks the model
    directly. Only the URL/search you request leaves the machine.

A first‑run **onboarding** flow lets you pick your level and topics of interest,
and the UI uses a clean, light "vocabulary‑app" aesthetic. Plus: a built‑in
**model manager** (pull/recommend Ollama models, separate fast model for
question generation), **daily reminder notifications**, and a **global hotkey**
to summon the popup from anywhere.

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

## Adaptive difficulty & question recommendation

**Goal:** as a learner answers correctly, the app should *harden* the questions —
both raising difficulty and asking deeper, more detailed questions — to keep
them at the edge of their ability and steepen the learning curve. As they
struggle, it should ease off. This is the classic "desirable difficulty" /
flow‑channel idea: keep the challenge just above current skill.

### Recommended approach (research‑backed)

The literature points to the **Elo rating system (ERS)** as the most practical
engine for this in an online, single‑user, local app:

- Each **learner** has a skill rating θ and each **item/topic** a difficulty
  rating b. After every answer, both are nudged: a correct answer raises θ and
  lowers the item's b; a wrong answer does the opposite. The next item is chosen
  so its difficulty matches the updated θ — i.e. it *automatically hardens after
  success*. Elo is "simple, fast, robust and order‑sensitive," updates after
  every question, and—unlike full **Item Response Theory (IRT)**—needs no
  large‑sample pre‑calibration, which makes it ideal for a fresh local DB.
  ([Pelánek, *Applications of the Elo rating system in adaptive educational
  systems*](https://www.sciencedirect.com/science/article/abs/pii/S036013151630080X);
  [overview of Elo for adaptive assessment](https://www.researchgate.net/publication/301635151_ON_THE_USE_OF_ELO_RATING_FOR_ADAPTIVE_ASSESSMENT);
  [multivariate Elo learner model](https://arxiv.org/pdf/1910.12581))
- IRT/CAT gives more principled ability estimates but requires calibrated item
  banks; Elo is the pragmatic online approximation and the two can be combined
  ([on‑the‑fly IRT estimation](https://link.springer.com/article/10.3758/s13428-022-01953-x)).

"Harder" should move along **two axes**, not one:

1. **Difficulty** — the Elo `b` rating, surfaced to the generator as the
   easy → medium → hard → advanced tier.
2. **Cognitive depth** — climb **Bloom's taxonomy** (remember → understand →
   apply → analyze → evaluate → create). On a win streak we ask for *deeper*
   questions ("design/critique/compare trade‑offs") rather than merely harder
   recall, which is what produces richer insight.

Retention is handled separately by **spaced repetition (SM‑2)** — already
implemented — so mastered items resurface right before they'd be forgotten.

### How it maps onto this app

- **Today already exists:** questions carry a difficulty tier; `next_difficulty`
  (in `crates/core/src/learning/scoring.rs`) nudges the tier from rolling
  accuracy with a seniority floor; SM‑2 schedules reviews
  (`crates/core/src/learning/srs.rs`).
- **Focus mode** (the Pomodoro tab) demonstrates the loop end‑to‑end: within a
  timed session it raises the difficulty tier after each correct answer and
  lowers it after a miss.
- **Proposed next step (Elo):** store a per‑topic `skill` rating and per‑item
  `difficulty` rating; update both on every attempt with the Elo formula
  `r' = r + K·(outcome − expected)`; pick the next topic/difficulty whose `b`
  matches the learner's current θ; and pass a **Bloom level** into the generator
  prompt that climbs on streaks. This generalizes the current tier nudging into
  a smooth, self‑calibrating recommender.

### Does the seniority level target the questions?

**Yes.** The selected level (senior / staff / principal) feeds the model in two
ways: it is written directly into the generation prompt ("writing
{difficulty}‑tier interview questions for a {level} engineer", in
`crates/core/src/learning/generator.rs` and `freeresponse.rs`), and it sets the
**baseline difficulty floor** (`Level::baseline_difficulty` in `models.rs`) that
`next_difficulty` adapts around. So level shifts both the *framing* and the
*starting/adaptive difficulty*. In Elo terms it's the **cold‑start prior** for
θ — a sensible starting ability before the app has enough answers to calibrate.

---

## License

Apache‑2.0.
