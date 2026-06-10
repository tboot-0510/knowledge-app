import type { Streak } from "../types";

export default function StreakBadge({ streak }: { streak: Streak }) {
  return (
    <div className="flex items-center gap-1.5 rounded-full bg-orange-500/15 px-3 py-1 text-xs text-orange-700">
      <span>🔥</span>
      <span className="font-semibold">{streak.current_streak}</span>
      <span className="text-orange-700/70">day streak</span>
    </div>
  );
}
