import { useState } from "react";
import ReactMarkdown from "react-markdown";
import type { AttemptResult, Difficulty, FollowupMode, Question } from "../types";
import { askFollowup } from "../lib/ipc";

interface Props {
  question: Question;
  index: number;
  result?: AttemptResult;
  onAnswer: (chosenIndex: number) => void;
}

const LETTERS = ["A", "B", "C", "D", "E", "F"];

function tierOf(difficulty: number): Difficulty {
  if (difficulty <= 1) return "easy";
  if (difficulty === 2) return "medium";
  if (difficulty <= 4) return "hard";
  return "advanced";
}

const TIER_COLOR: Record<Difficulty, string> = {
  easy: "bg-emerald-500/15 text-emerald-700",
  medium: "bg-sky-500/15 text-sky-700",
  hard: "bg-amber-500/15 text-amber-700",
  advanced: "bg-rose-500/15 text-rose-600",
};

export default function McqCard({ question, index, result, onAnswer }: Props) {
  const answered = result !== undefined;
  const tier = tierOf(question.difficulty);

  // Follow-up interaction state (local to this question).
  const [followup, setFollowup] = useState("");
  const [busy, setBusy] = useState(false);
  const [custom, setCustom] = useState("");
  const [showAsk, setShowAsk] = useState(false);

  async function run(mode: FollowupMode, userQuery?: string) {
    if (busy) return;
    setBusy(true);
    setFollowup("");
    try {
      await askFollowup(
        question.id,
        mode,
        (chunk) => {
          if (chunk.kind === "token") setFollowup((p) => p + chunk.text);
          else if (chunk.kind === "error") setFollowup((p) => p + `\n\n_${chunk.message}_`);
        },
        userQuery,
      );
    } catch (e) {
      setFollowup(String(e));
    } finally {
      setBusy(false);
    }
  }

  function choiceClass(i: number): string {
    if (!answered) {
      return "border-black/[0.08] bg-black/[0.04] hover:border-accent/60 hover:bg-accent/10";
    }
    if (i === result!.correct_index) {
      return "border-emerald-500/60 bg-emerald-500/15 text-emerald-700";
    }
    if (i === result!.chosen_index) {
      return "border-rose-500/60 bg-rose-500/15 text-rose-700";
    }
    return "border-black/[0.05] bg-black/[0.04] opacity-60";
  }

  return (
    <div className="rounded-xl border border-black/[0.08] bg-white p-4">
      <div className="mb-3 flex items-start gap-2">
        <span className="mt-0.5 rounded bg-black/[0.06] px-1.5 py-0.5 text-[10px] font-semibold text-neutral-600">
          Q{index + 1}
        </span>
        <p className="flex-1 text-sm leading-relaxed text-neutral-900">{question.prompt}</p>
        <span className={`shrink-0 rounded-full px-2 py-0.5 text-[10px] capitalize ${TIER_COLOR[tier]}`}>
          {tier}
        </span>
      </div>
      <div className="flex flex-col gap-2">
        {question.choices.map((choice, i) => (
          <button
            key={i}
            disabled={answered}
            onClick={() => onAnswer(i)}
            className={`flex items-start gap-2 rounded-lg border px-3 py-2 text-left text-sm transition ${choiceClass(
              i,
            )}`}
          >
            <span className="font-semibold text-neutral-500">{LETTERS[i]}</span>
            <span>{choice}</span>
          </button>
        ))}
      </div>
      {answered && (
        <div className="mt-3 rounded-lg bg-black/[0.04] p-3 text-xs leading-relaxed text-neutral-600">
          <span className="font-semibold text-neutral-800">
            {result!.is_correct ? "Correct. " : "Not quite. "}
          </span>
          {result!.explanation}
        </div>
      )}

      {/* Follow-up actions, powered by the local LLM. */}
      {answered && (
        <div className="mt-3 flex flex-col gap-2">
          <div className="flex flex-wrap gap-2">
            <FollowBtn disabled={busy} onClick={() => run("explain")}>
              💡 Explain in depth
            </FollowBtn>
            <FollowBtn disabled={busy} onClick={() => run("followup")}>
              ➕ Follow-up question
            </FollowBtn>
            <FollowBtn disabled={busy} onClick={() => setShowAsk((s) => !s)}>
              💬 Ask your own
            </FollowBtn>
          </div>

          {showAsk && (
            <div className="flex gap-2">
              <input
                value={custom}
                onChange={(e) => setCustom(e.target.value)}
                placeholder="Ask anything about this concept…"
                onKeyDown={(e) =>
                  e.key === "Enter" && custom.trim() && run("custom", custom.trim())
                }
                className="flex-1 rounded-lg border border-black/[0.08] bg-black/[0.04] px-3 py-1.5 text-xs outline-none focus:border-accent/60"
              />
              <button
                disabled={busy || !custom.trim()}
                onClick={() => run("custom", custom.trim())}
                className="rounded-lg bg-accent/20 px-3 py-1.5 text-xs font-medium text-accent disabled:opacity-50"
              >
                Ask
              </button>
            </div>
          )}

          {(followup || busy) && (
            <div className="md max-w-none rounded-lg bg-stone-100 p-3 text-xs text-neutral-800">
              <ReactMarkdown>{followup || "Thinking…"}</ReactMarkdown>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function FollowBtn({
  children,
  onClick,
  disabled,
}: {
  children: React.ReactNode;
  onClick: () => void;
  disabled: boolean;
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className="rounded-lg border border-black/[0.08] bg-black/[0.04] px-2.5 py-1 text-[11px] text-neutral-600 transition hover:border-accent/50 hover:text-accent disabled:opacity-50"
    >
      {children}
    </button>
  );
}
