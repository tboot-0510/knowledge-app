import { useEffect, useState } from "react";
import DailyChallenge from "./views/DailyChallenge";
import Topics from "./views/Topics";
import Practice from "./views/Practice";
import Code from "./views/Code";
import Review from "./views/Review";
import RepoChat from "./views/RepoChat";
import Progress from "./views/Progress";
import Settings from "./views/Settings";
import { countDueReviews, onNavigate, sendReminderIfDue } from "./lib/ipc";

type Route =
  | "daily"
  | "topics"
  | "practice"
  | "code"
  | "review"
  | "repos"
  | "progress"
  | "settings";

const TABS: { id: Route; label: string }[] = [
  { id: "daily", label: "Today" },
  { id: "topics", label: "Topics" },
  { id: "practice", label: "Practice" },
  { id: "code", label: "Code" },
  { id: "review", label: "Review" },
  { id: "repos", label: "Repos" },
  { id: "progress", label: "Stats" },
  { id: "settings", label: "Settings" },
];

export default function App() {
  const [route, setRoute] = useState<Route>("daily");
  const [dueCount, setDueCount] = useState(0);

  useEffect(() => {
    // Tray menu navigation.
    const unlisten = onNavigate((r) => {
      if (TABS.some((t) => t.id === r)) setRoute(r as Route);
    });
    // One-time launch tasks: reminder check + due-review badge.
    sendReminderIfDue().catch(() => {});
    countDueReviews().then(setDueCount).catch(() => {});
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Refresh the due-review badge whenever we leave the review tab.
  useEffect(() => {
    if (route !== "review") countDueReviews().then(setDueCount).catch(() => {});
  }, [route]);

  return (
    <div className="flex h-full flex-col bg-ink/95 text-slate-100 backdrop-blur-xl">
      <header className="flex items-center justify-between border-b border-white/10 px-4 py-3">
        <div className="flex items-center gap-2">
          <span className="text-lg">📚</span>
          <h1 className="text-sm font-semibold tracking-wide">Knowledge</h1>
        </div>
        <nav className="flex flex-wrap gap-1">
          {TABS.map((t) => (
            <button
              key={t.id}
              onClick={() => setRoute(t.id)}
              className={`relative rounded-md px-2 py-1 text-xs transition ${
                route === t.id
                  ? "bg-accent/20 text-accent"
                  : "text-slate-400 hover:text-slate-200"
              }`}
            >
              {t.label}
              {t.id === "review" && dueCount > 0 && (
                <span className="ml-1 rounded-full bg-rose-500/80 px-1 text-[9px] font-semibold text-white">
                  {dueCount}
                </span>
              )}
            </button>
          ))}
        </nav>
      </header>

      <main className="min-h-0 flex-1 overflow-y-auto p-4">
        {route === "daily" && <DailyChallenge />}
        {route === "topics" && <Topics />}
        {route === "practice" && <Practice />}
        {route === "code" && <Code />}
        {route === "review" && <Review />}
        {route === "repos" && <RepoChat />}
        {route === "progress" && <Progress />}
        {route === "settings" && <Settings />}
      </main>
    </div>
  );
}
