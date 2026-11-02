import { useEffect, useLayoutEffect, useState } from "react";
import { AccountsPage } from "./accounts/AccountsPage";
import { events } from "./api";
import { Icon } from "./components";
import { usePreference } from "./hooks";
import { CopyProvider, useCopy, type Lang } from "./i18n";
import { NoticeBanner, NoticeProvider, useNotices } from "./notices";
import { ProjectsPage } from "./projects/ProjectsPage";
import { SessionsPage } from "./sessions/SessionsPage";
import { UpdateButton } from "./UpdateButton";
import { useSnapshot } from "./useSnapshot";

const TABS = ["accounts", "projects", "sessions"] as const;
type Tab = (typeof TABS)[number];
const THEMES = ["system", "dark", "light"] as const;
const INTERVALS = [0, 5, 15, 30] as const;
const VERSION = "0.1.0";

// The only language with a complete table today (#166). How the language is
// chosen once there are two is an open question in the pull request.
const LANG: Lang = "en";

export function App() {
  return (
    <CopyProvider lang={LANG}>
      <NoticeProvider>
        <Shell />
      </NoticeProvider>
    </CopyProvider>
  );
}

function Shell() {
  const [tab, setTab] = useState<Tab>("accounts");
  const [adding, setAdding] = useState(false);
  const [theme, setTheme] = usePreference("theme", "system", THEMES);
  const [minutes, setMinutes] = usePreference("refresh-minutes", 0, INTERVALS);
  const t = useCopy();
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
      events.navigate((page) => {
        if (page === "projects") setTab("projects");
      }),
      events.backendError(fail),
    ];
    return () => stops.forEach((stop) => stop());
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

  const page = t.pages[tab];
  return (
    <div className="app">
      <aside>
        <div className="brand">
          <div className="brand-mark">✳</div>
          <div>
            {t.productName}
            <span>{t.productTagline}</span>
          </div>
        </div>
        <div className="nav-label">{t.navHeading}</div>
        <nav aria-label={t.navAria}>
          {TABS.map((tabName, i) => (
            <button
              key={tabName}
              className={tab === tabName ? "active" : ""}
              onClick={() => {
                setTab(tabName);
                dismiss();
              }}
              aria-current={tab === tabName ? "page" : undefined}
            >
              <Icon name={tabName} />
              <span>{t.pages[tabName].label}</span>
              <kbd>⌘{i + 1}</kbd>
            </button>
          ))}
        </nav>
        <div className="sidebar-note">
          <span className="status-dot" />
          {t.sidebarNoteTitle}
          <p>{t.sidebarNoteBody}</p>
        </div>
        <div className="preferences">
          <label>
            {t.appearance}
            <select
              aria-label={t.appearance}
              value={theme}
              onChange={(e) => setTheme(e.target.value as (typeof THEMES)[number])}
            >
              {THEMES.map((name) => (
                <option key={name} value={name}>
                  {t.themes[name]}
                </option>
              ))}
            </select>
          </label>
          <label>
            {t.liveRefresh}
            <select
              aria-label={t.liveRefreshAria}
              value={minutes}
              onChange={(e) =>
                setMinutes(Number(e.target.value) as (typeof INTERVALS)[number])
              }
            >
              {INTERVALS.map((n) => (
                <option key={n} value={n}>
                  {n ? t.everyMinutes(n) : t.manualOnly}
                </option>
              ))}
            </select>
          </label>
        </div>
        <div className="sidebar-footer">
          {t.sidebarFooter} <span>{t.version(VERSION)}</span>
          <UpdateButton />
        </div>
      </aside>
      <main>
        <header>
          <div className="breadcrumb">
            {t.breadcrumbRoot} <span>/</span> {page.label}
          </div>
          <button
            className="quiet refresh"
            onClick={() => void refresh(true)}
            disabled={refreshing}
            title={t.refreshTitle}
          >
            <Icon name="refresh" />
            {refreshing ? t.refreshing : t.refresh}
          </button>
        </header>
        <div className="content">
          <div className="page-heading">
            <div>
              <span className="eyebrow">{t.eyebrow}</span>
              <h1>{page.title}</h1>
              <p>{page.lead}</p>
            </div>
            {tab === "accounts" && (
              <button className="primary" onClick={() => setAdding(true)}>
                <Icon name="plus" />
                {t.addAccount}
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
          {tab === "sessions" && (
            <SessionsPage accounts={state.snapshot?.accounts ?? []} />
          )}
        </div>
        <footer>
          <span>
            <span className="status-dot" />
            {t.footerPrivacy}
          </span>
          <span>{t.footerMotto}</span>
        </footer>
      </main>
    </div>
  );
}
