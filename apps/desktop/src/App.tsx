import { useEffect, useLayoutEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { AccountsPage } from "./accounts/AccountsPage";
import { Icon } from "./components";
import { usePreference } from "./hooks";
import { NoticeBanner, NoticeProvider, useNotices } from "./notices";
import { ProjectsPage } from "./projects/ProjectsPage";
import { SessionsPage } from "./sessions/SessionsPage";
import { useSnapshot } from "./useSnapshot";

const TABS = ["accounts", "projects", "sessions"] as const;
type Tab = (typeof TABS)[number];
const THEMES = ["system", "dark", "light"] as const;
const INTERVALS = [0, 5, 15, 30] as const;

const pages: Record<Tab, { label: string; title: string; lead: string }> = {
  accounts: {
    label: "Accounts",
    title: "Your accounts",
    lead: "A clear view of every identity. The right account for every project.",
  },
  projects: {
    label: "Projects",
    title: "Project bindings",
    lead: "Choose once. The account follows the directory and its children.",
  },
  sessions: {
    label: "Sessions",
    title: "Session library",
    lead: "Discover conversations across accounts without opening their content.",
  },
};

export function App() {
  return (
    <NoticeProvider>
      <Shell />
    </NoticeProvider>
  );
}

function Shell() {
  const [tab, setTab] = useState<Tab>("accounts");
  const [adding, setAdding] = useState(false);
  const [theme, setTheme] = usePreference("theme", "system", THEMES);
  const [minutes, setMinutes] = usePreference("refresh-minutes", 0, INTERVALS);
  const state = useSnapshot();
  const { refresh, refreshing } = state;
  const { fail, dismiss } = useNotices();

  // Before paint: `index.html` starts on "system", and an effect would show
  // that for one frame before a stored "dark" or "light".
  useLayoutEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);

  useEffect(() => {
    if (!minutes) return;
    const timer = window.setInterval(() => void refresh(true), minutes * 60_000);
    return () => window.clearInterval(timer);
  }, [minutes, refresh]);

  // Sent by the menu bar, which works with this window closed.
  useEffect(() => {
    const stops = [
      listen<string>("navigate", (e) => {
        if (e.payload === "projects") setTab("projects");
      }),
      listen("backend-error", (e) => fail(e.payload)),
    ];
    return () => stops.forEach((p) => void p.then((stop) => stop()).catch(() => {}));
  }, [fail]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey)) return;
      const index = ["1", "2", "3"].indexOf(e.key);
      if (e.key === "r") void refresh(true);
      else if (e.key === "n") {
        setTab("accounts");
        setAdding(true);
      } else if (index >= 0) setTab(TABS[index]);
      else return;
      e.preventDefault();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [refresh]);

  const page = pages[tab];
  return (
    <div className="app">
      <aside>
        <div className="brand">
          <div className="brand-mark">✳</div>
          <div>
            Claude Accounts<span>PROJECT-AWARE WORKSPACE</span>
          </div>
        </div>
        <div className="nav-label">WORKSPACE</div>
        <nav aria-label="Main navigation">
          {TABS.map((t, i) => (
            <button
              key={t}
              className={tab === t ? "active" : ""}
              onClick={() => {
                setTab(t);
                dismiss();
              }}
              aria-current={tab === t ? "page" : undefined}
            >
              <Icon name={t} />
              <span>{pages[t].label}</span>
              <kbd>⌘{i + 1}</kbd>
            </button>
          ))}
        </nav>
        <div className="sidebar-note">
          <span className="status-dot" />
          Directory-based by design
          <p>
            Each terminal has its own account. Choosing a default never switches
            a running session.
          </p>
        </div>
        <div className="preferences">
          <label>
            Appearance
            <select
              aria-label="Appearance"
              value={theme}
              onChange={(e) => setTheme(e.target.value as (typeof THEMES)[number])}
            >
              <option value="system">System</option>
              <option value="dark">Dark</option>
              <option value="light">Light</option>
            </select>
          </label>
          <label>
            Live refresh
            <select
              aria-label="Live refresh interval"
              value={minutes}
              onChange={(e) =>
                setMinutes(Number(e.target.value) as (typeof INTERVALS)[number])
              }
            >
              <option value={0}>Manual only</option>
              <option value={5}>Every 5 min</option>
              <option value={15}>Every 15 min</option>
              <option value={30}>Every 30 min</option>
            </select>
          </label>
        </div>
        <div className="sidebar-footer">
          LOCAL & PRIVATE <span>v0.1.0</span>
        </div>
      </aside>
      <main>
        <header>
          <div className="breadcrumb">
            Workspace <span>/</span> {page.label}
          </div>
          <button
            className="quiet refresh"
            onClick={() => void refresh(true)}
            disabled={refreshing}
            title="Refresh live identity and usage (⌘R)"
          >
            <Icon name="refresh" />
            {refreshing ? "Refreshing…" : "Refresh status"}
          </button>
        </header>
        <div className="content">
          <div className="page-heading">
            <div>
              <span className="eyebrow">CLAUDE CODE · ACCOUNT MANAGER</span>
              <h1>{page.title}</h1>
              <p>{page.lead}</p>
            </div>
            {tab === "accounts" && (
              <button className="primary" onClick={() => setAdding(true)}>
                <Icon name="plus" />
                Add account
              </button>
            )}
          </div>
          <NoticeBanner />
          {tab === "accounts" && (
            <AccountsPage state={state} adding={adding} setAdding={setAdding} />
          )}
          {tab === "projects" && (
            <ProjectsPage accounts={state.snapshot?.accounts ?? []} />
          )}
          {tab === "sessions" && <SessionsPage />}
        </div>
        <footer>
          <span>
            <span className="status-dot" />
            Runs on your Mac · no telemetry
          </span>
          <span>Accounts follow projects. Sessions stay yours.</span>
        </footer>
      </main>
    </div>
  );
}
