import { useEffect, useRef, useState } from "react";
import { api } from "../api";
import { Dialog, Icon, Meter } from "../components";
import type { Account, IdentitySource, Installation, LockStatus } from "../contract";
import { freshness, usageLabel } from "../freshness";
import { useLaunch, useNotices } from "../notices";

const identityLabels: Record<IdentitySource, string> = {
  live: "● Verified live",
  local: "○ Local identity",
  offline: "○ Offline identity",
  no_credentials: "○ No credentials",
  unknown: "○ Unverified",
};
const blocked = (lock: LockStatus) => lock === "drift" || lock === "corrupt";
const title = (a: Account) => (a.name === "default" ? "Standard account" : a.name);

type Open = "details" | "login" | "default" | null;

export function AccountCard({
  account: a,
  index,
  now,
  setDefault,
}: {
  account: Account;
  index: number;
  now: number;
  setDefault: (name: string) => Promise<void>;
}) {
  const [open, setOpen] = useState<Open>(null);
  const [launching, setLaunching] = useState(false);
  const menu = useRef<HTMLDetailsElement>(null);
  const launch = useLaunch();
  const { fail } = useNotices();
  const close = () => setOpen(null);

  // A <details> menu stays open until toggled again; close it the way a
  // menu is expected to close: a click elsewhere, or Escape.
  useEffect(() => {
    const dismiss = (e: PointerEvent | KeyboardEvent) => {
      const details = menu.current;
      if (!details?.open) return;
      const outside =
        e instanceof KeyboardEvent ? e.key === "Escape" : !details.contains(e.target as Node);
      if (outside) details.open = false;
    };
    document.addEventListener("pointerdown", dismiss);
    document.addEventListener("keydown", dismiss);
    return () => {
      document.removeEventListener("pointerdown", dismiss);
      document.removeEventListener("keydown", dismiss);
    };
  }, []);
  const usageState = freshness(a.usage, now).state;

  const run = async () => {
    setLaunching(true);
    try {
      await launch(a.name, null, "run");
    } catch (e) {
      fail(e);
    } finally {
      setLaunching(false);
    }
  };

  return (
    <article className="account-card">
      <div className="card-top">
        <div className={`avatar color-${index % 3}`}>
          {a.name === "default" ? "S" : a.name[0].toUpperCase()}
        </div>
        <div className="account-heading">
          <h3 title={a.name}>{title(a)}</h3>
          <p title={a.email ?? ""}>{a.email ?? "Identity not yet known"}</p>
        </div>
        {a.is_default && <span className="badge default">★ Default</span>}
      </div>
      <div className="card-meta">
        <span className="badge">{a.plan ?? "Plan unknown"}</span>
        <span
          className={`identity ${blocked(a.lock) ? "warning" : ""}`}
          title={`Identity: ${a.identity_source}; pin: ${a.lock}`}
        >
          {a.lock === "drift"
            ? "Identity drift"
            : a.lock === "corrupt"
              ? "Corrupt pin"
              : identityLabels[a.identity_source]}
        </span>
      </div>
      <div className="usage-meters">
        <Meter label="5-hour window" value={a.usage.five_hour} now={now} />
        <Meter label="7-day window" value={a.usage.seven_day} now={now} />
      </div>
      <div className="usage-source">
        <span className={`status-dot ${usageState === "live" ? "live" : "stale"}`} />
        {usageLabel(a.usage, now)}
      </div>
      <div className="card-actions">
        <button
          className="launch"
          disabled={launching || !a.available || blocked(a.lock)}
          onClick={() => void run()}
        >
          {launching ? "Opening…" : "Open Claude"}
          <Icon name="arrow" />
        </button>
        <details ref={menu}>
          <summary aria-label={`Actions for ${a.name}`} title="Account actions">
            •••
          </summary>
          <div className="context-menu" onClick={() => menu.current?.removeAttribute("open")}>
            <button onClick={() => setOpen("details")}>Details & diagnostics</button>
            <button disabled={!a.available} onClick={() => setOpen("login")}>
              Authenticate / re-login
            </button>
            <button
              disabled={a.is_default || (!a.available && a.name !== "default")}
              onClick={() => setOpen("default")}
            >
              Make default…
            </button>
          </div>
        </details>
      </div>
      {open === "details" && <Details account={a} onClose={close} />}
      {open === "login" && (
        <Dialog
          title={`Authenticate ${a.name}?`}
          confirmLabel="Open login"
          onConfirm={() => launch(a.name, null, "login")}
          onClose={close}
        >
          <p>
            Open the existing Claude login flow in Terminal. Check the browser
            identity before signing in. Return here and refresh after the login
            finishes.
          </p>
        </Dialog>
      )}
      {open === "default" && (
        <Dialog
          title={`Make ${a.name} the default?`}
          confirmLabel="Change default"
          onConfirm={() => setDefault(a.name)}
          onClose={close}
        >
          <p>
            New terminals in directories without a binding will use{" "}
            <strong>{a.name}</strong>. Existing Claude processes keep their
            account.
          </p>
        </Dialog>
      )}
    </article>
  );
}

function Details({ account: a, onClose }: { account: Account; onClose: () => void }) {
  const [install, setInstall] = useState<Installation | null>(null);
  useEffect(() => {
    api.installation().then(setInstall, () => setInstall(null));
  }, []);
  return (
    <Dialog title={title(a)} onClose={onClose}>
      <dl>
        <dt>Identity</dt>
        <dd>{a.email ?? "Not known"}</dd>
        <dt>Verification</dt>
        <dd>
          {a.identity_source} · pin {a.lock}
        </dd>
        <dt>Subscription</dt>
        <dd>{a.plan ?? "Not known"}</dd>
        <dt>Organization</dt>
        <dd>{a.organization ?? "Not known"}</dd>
        <dt>Configuration</dt>
        <dd>{a.available ? "Directory available" : "Directory missing or unsafe"}</dd>
        <dt>Usage source</dt>
        <dd>{a.usage.source}</dd>
        <dt>Bundled CLI</dt>
        <dd>
          {install
            ? `${install.bundled_version} · ${install.bundled_available ? "ready" : "missing"}`
            : "Checking…"}
        </dd>
        <dt>Installed CLI</dt>
        <dd>{install?.installed_version ?? "Not installed at ~/.claude-switch/bin"}</dd>
      </dl>
      <p className="muted">
        Local identity is a disk record. Refresh checks the profile API. Pin
        status compares the recorded identity with the CLI identity lock; it does
        not prove a live login.
      </p>
    </Dialog>
  );
}
