import { useEffect, useRef, useState } from "react";
import { api } from "../api";
import { Dialog, Icon, Meter } from "../components";
import type { Account, Installation, LockStatus } from "../contract";
import { freshness, usageLabel } from "../freshness";
import { list, useCopy, type Messages } from "../i18n";
import { useLaunch, useNotices } from "../notices";

const blocked = (lock: LockStatus) => lock === "drift" || lock === "corrupt";
const title = (t: Messages, a: Account) =>
  a.name === "default" ? t.standardAccount : a.name;

type Open = "details" | "login" | "default" | "remove" | "pin" | "clone" | null;

export function AccountCard({
  account: a,
  index,
  now,
  setDefault,
  removeAccount,
  pinAccount,
}: {
  account: Account;
  index: number;
  now: number;
  setDefault: (name: string) => Promise<void>;
  removeAccount: (name: string) => Promise<void>;
  pinAccount: (name: string, force: boolean) => Promise<void>;
}) {
  const t = useCopy();
  const [open, setOpen] = useState<Open>(null);
  const [launching, setLaunching] = useState(false);
  const menu = useRef<HTMLDetailsElement>(null);
  const launch = useLaunch();
  const { fail, info } = useNotices();
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
          {a.name === "default" ? t.standardAccount[0] : a.name[0].toUpperCase()}
        </div>
        <div className="account-heading">
          <h3 title={a.name}>{title(t, a)}</h3>
          <p title={a.email ?? ""}>{a.email ?? t.identityUnknown}</p>
        </div>
        {a.is_default && <span className="badge default">{t.defaultBadge}</span>}
      </div>
      <div className="card-meta">
        <span className="badge">{a.plan ?? t.planUnknown}</span>
        <span
          className={`identity ${blocked(a.lock) ? "warning" : ""}`}
          title={t.identityTooltip(
            t.identitySources[a.identity_source],
            t.lockStates[a.lock],
          )}
        >
          {a.lock === "drift"
            ? t.identityDrift
            : a.lock === "corrupt"
              ? t.corruptPin
              : t.identitySources[a.identity_source]}
        </span>
      </div>
      <div className="usage-meters">
        <Meter label={t.fiveHourWindow} value={a.usage.five_hour} now={now} />
        <Meter label={t.sevenDayWindow} value={a.usage.seven_day} now={now} />
      </div>
      <div className="usage-source">
        <span className={`status-dot ${usageState === "live" ? "live" : "stale"}`} />
        {usageLabel(t, a.usage, now)}
      </div>
      <div className="card-actions">
        <button
          className="launch"
          disabled={launching || !a.available || blocked(a.lock)}
          onClick={() => void run()}
        >
          {launching ? t.opening : t.openClaude}
          <Icon name="arrow" />
        </button>
        <details ref={menu}>
          <summary aria-label={t.actionsFor(a.name)} title={t.accountActions}>
            •••
          </summary>
          <div className="context-menu" onClick={() => menu.current?.removeAttribute("open")}>
            <button onClick={() => setOpen("details")}>{t.detailsAction}</button>
            <button disabled={!a.available} onClick={() => setOpen("login")}>
              {t.loginAction}
            </button>
            <button
              disabled={a.is_default || (!a.available && a.name !== "default")}
              onClick={() => setOpen("default")}
            >
              {t.makeDefaultAction}
            </button>
            {(a.lock === "unpinned" || blocked(a.lock)) && (
              <button disabled={!a.available} onClick={() => setOpen("pin")}>
                {blocked(a.lock) ? t.repinAction : t.pinAction}
              </button>
            )}
            {a.name !== "default" && (
              <button disabled={!a.available} onClick={() => setOpen("clone")}>
                {t.cloneSettingsAction}
              </button>
            )}
            {a.name !== "default" && (
              <button onClick={() => setOpen("remove")}>{t.trashAction}</button>
            )}
          </div>
        </details>
      </div>
      {open === "details" && <Details account={a} onClose={close} />}
      {open === "login" && (
        <Dialog
          title={t.loginTitle(a.name)}
          confirmLabel={t.loginConfirm}
          onConfirm={() => launch(a.name, null, "login")}
          onClose={close}
        >
          <p>{t.loginBody}</p>
        </Dialog>
      )}
      {open === "default" && (
        <Dialog
          title={t.defaultTitle(a.name)}
          confirmLabel={t.defaultConfirm}
          onConfirm={() => setDefault(a.name)}
          onClose={close}
        >
          <p>{t.defaultBody(<strong>{a.name}</strong>)}</p>
        </Dialog>
      )}
      {open === "pin" && (
        <Dialog
          title={blocked(a.lock) ? t.repinTitle(a.name) : t.pinTitle(a.name)}
          confirmLabel={blocked(a.lock) ? t.repinConfirm : t.pinConfirm}
          onConfirm={() => pinAccount(a.name, blocked(a.lock))}
          onClose={close}
        >
          <p>
            {blocked(a.lock)
              ? t.repinBody(<strong>{a.name}</strong>)
              : t.pinBody(<strong>{a.name}</strong>)}
          </p>
        </Dialog>
      )}
      {open === "clone" && (
        <Dialog
          title={t.cloneSettingsTitle(a.name)}
          confirmLabel={t.cloneSettingsConfirm}
          onConfirm={async () => {
            const copied = await api.cloneSettings(a.name);
            info(
              copied.length
                ? t.cloneSettingsDone(a.name, list(t, copied))
                : t.cloneSettingsNothing(a.name),
            );
          }}
          onClose={close}
        >
          <p>{t.cloneSettingsBody(<strong>{a.name}</strong>)}</p>
        </Dialog>
      )}
      {open === "remove" && (
        <Dialog
          title={t.trashTitle(a.name)}
          confirmLabel={t.trashConfirm}
          onConfirm={() => removeAccount(a.name)}
          onClose={close}
        >
          <p>{t.trashBody(<strong>{a.name}</strong>)}</p>
        </Dialog>
      )}
    </article>
  );
}

function Details({ account: a, onClose }: { account: Account; onClose: () => void }) {
  const t = useCopy();
  const d = t.details;
  const [install, setInstall] = useState<Installation | null>(null);
  useEffect(() => {
    api.installation().then(setInstall, () => setInstall(null));
  }, []);
  return (
    <Dialog title={title(t, a)} onClose={onClose}>
      <dl>
        <dt>{d.identity}</dt>
        <dd>{a.email ?? t.notKnown}</dd>
        <dt>{d.verification}</dt>
        <dd>
          {d.verificationValue(
            t.identitySources[a.identity_source],
            t.lockStates[a.lock],
          )}
        </dd>
        <dt>{d.subscription}</dt>
        <dd>{a.plan ?? t.notKnown}</dd>
        <dt>{d.organization}</dt>
        <dd>{a.organization ?? t.notKnown}</dd>
        <dt>{d.configuration}</dt>
        <dd>{a.available ? d.directoryAvailable : d.directoryUnavailable}</dd>
        <dt>{d.usageSource}</dt>
        <dd>{t.usageSources[a.usage.source]}</dd>
        <dt>{d.bundledCli}</dt>
        <dd>
          {install
            ? d.bundledValue(install.bundled_version, install.bundled_available)
            : d.checking}
        </dd>
        <dt>{d.installedCli}</dt>
        <dd>{install?.installed_version ?? d.notInstalled}</dd>
      </dl>
      <p className="muted">{d.note}</p>
    </Dialog>
  );
}
