import type { AttemptResult, Question } from "../types";

interface Props {
  question: Question;
  index: number;
  result?: AttemptResult;
  onAnswer: (chosenIndex: number) => void;
}

const LETTERS = ["A", "B", "C", "D", "E", "F"];

export default function McqCard({ question, index, result, onAnswer }: Props) {
  const answered = result !== undefined;

  function choiceClass(i: number): string {
    if (!answered) {
      return "border-white/10 bg-white/5 hover:border-accent/60 hover:bg-accent/10";
    }
    if (i === result!.correct_index) {
      return "border-emerald-500/60 bg-emerald-500/15 text-emerald-200";
    }
    if (i === result!.chosen_index) {
      return "border-rose-500/60 bg-rose-500/15 text-rose-200";
    }
    return "border-white/5 bg-white/5 opacity-60";
  }

  return (
    <div className="rounded-xl border border-white/10 bg-panel/60 p-4">
      <div className="mb-3 flex items-start gap-2">
        <span className="mt-0.5 rounded bg-white/10 px-1.5 py-0.5 text-[10px] font-semibold text-slate-300">
          Q{index + 1}
        </span>
        <p className="text-sm leading-relaxed text-slate-100">{question.prompt}</p>
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
            <span className="font-semibold text-slate-400">{LETTERS[i]}</span>
            <span>{choice}</span>
          </button>
        ))}
      </div>
      {answered && (
        <div className="mt-3 rounded-lg bg-white/5 p-3 text-xs leading-relaxed text-slate-300">
          <span className="font-semibold text-slate-200">
            {result!.is_correct ? "Correct. " : "Not quite. "}
          </span>
          {result!.explanation}
        </div>
      )}
    </div>
  );
}
