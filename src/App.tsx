import { useEffect, useState } from "react";
import DailyChallenge from "./views/DailyChallenge";
import Topics from "./views/Topics";
import Practice from "./views/Practice";
import Code from "./views/Code";
import Review from "./views/Review";
import RepoChat from "./views/RepoChat";
import Progress from "./views/Progress";
import Settings from "./views/Settings";
import Onboarding from "./views/Onboarding";
import {
  countDueReviews,
  getSettings,
  onNavigate,
  sendReminderIfDue,
} from "./lib/ipc";

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
  const [onboarded, setOnboarded] = useState<boolean | null>(null);

  useEffect(() => {
    getSettings()
      .then((s) => setOnboarded(s.onboarded))
      .catch(() => setOnboarded(true));
  }, []);

  useEffect(() => {
    if (!onboarded) return;
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
  }, [onboarded]);

  // Refresh the due-review badge whenever we leave the review tab.
  useEffect(() => {
    if (route !== "review") countDueReviews().then(setDueCount).catch(() => {});
  }, [route]);

  if (onboarded === null) return <div className="h-full bg-paper/95" />;
  if (!onboarded) return <Onboarding onDone={() => setOnboarded(true)} />;

  return (
    <div className="flex h-full flex-col bg-paper/95 text-ink backdrop-blur-xl">
      <header className="flex flex-col gap-2 border-b border-black/[0.06] px-5 pb-2 pt-4">
        <h1 className="font-display text-xl font-semibold tracking-tight text-ink">
          Knowledge
        </h1>
        <nav className="-mx-1 flex gap-1 overflow-x-auto pb-1">
          {TABS.map((t) => (
            <button
              key={t.id}
              onClick={() => setRoute(t.id)}
              className={`relative whitespace-nowrap rounded-full px-3 py-1 text-xs transition ${
                route === t.id
                  ? "bg-accent/10 font-medium text-accent"
                  : "text-neutral-400 hover:text-neutral-700"
              }`}
            >
              {t.label}
              {t.id === "review" && dueCount > 0 && (
                <span className="ml-1 rounded-full bg-rose-500 px-1 text-[9px] font-semibold text-white">
                  {dueCount}
                </span>
              )}
            </button>
          ))}
        </nav>
      </header>

      <main className="min-h-0 flex-1 overflow-y-auto px-5 py-4">{renderRoute()}</main>
    </div>
  );

  function renderRoute() {
    switch (route) {
      case "daily":
        return <DailyChallenge />;
      case "topics":
        return <Topics />;
      case "practice":
        return <Practice />;
      case "code":
        return <Code />;
      case "review":
        return <Review />;
      case "repos":
        return <RepoChat />;
      case "progress":
        return <Progress />;
      case "settings":
        return <Settings />;
    }
  }
}
