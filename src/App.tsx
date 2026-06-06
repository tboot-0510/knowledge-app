import { useEffect, useState } from "react";
import DailyChallenge from "./views/DailyChallenge";
import RepoChat from "./views/RepoChat";
import Progress from "./views/Progress";
import Settings from "./views/Settings";
import { onNavigate } from "./lib/ipc";

type Route = "daily" | "repos" | "progress" | "settings";

const TABS: { id: Route; label: string }[] = [
  { id: "daily", label: "Today" },
  { id: "repos", label: "Repos" },
  { id: "progress", label: "Progress" },
  { id: "settings", label: "Settings" },
];

export default function App() {
  const [route, setRoute] = useState<Route>("daily");

  // The tray menu emits "navigate" events to jump to a section.
  useEffect(() => {
    const unlisten = onNavigate((r) => {
      if (TABS.some((t) => t.id === r)) setRoute(r as Route);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  return (
    <div className="flex h-full flex-col bg-ink/95 text-slate-100 backdrop-blur-xl">
      <header className="flex items-center justify-between border-b border-white/10 px-4 py-3">
        <div className="flex items-center gap-2">
          <span className="text-lg">📚</span>
          <h1 className="text-sm font-semibold tracking-wide">Knowledge</h1>
        </div>
        <nav className="flex gap-1">
          {TABS.map((t) => (
            <button
              key={t.id}
              onClick={() => setRoute(t.id)}
              className={`rounded-md px-2.5 py-1 text-xs transition ${
                route === t.id
                  ? "bg-accent/20 text-accent"
                  : "text-slate-400 hover:text-slate-200"
              }`}
            >
              {t.label}
            </button>
          ))}
        </nav>
      </header>

      <main className="min-h-0 flex-1 overflow-y-auto p-4">
        {route === "daily" && <DailyChallenge />}
        {route === "repos" && <RepoChat />}
        {route === "progress" && <Progress />}
        {route === "settings" && <Settings />}
      </main>
    </div>
  );
}
