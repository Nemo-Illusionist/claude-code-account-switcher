import { useCallback, useEffect, useState } from "react";
import { api } from "../api";
import { Dialog, Empty, Failure, Icon } from "../components";
import type { Account, Session } from "../contract";
import { kilobytes, useCopy } from "../i18n";
import { useNotices } from "../notices";

/** Rows rendered per step. The list itself is metadata only and cheap. */
export const PAGE = 100;

export function SessionsPage({ accounts }: { accounts: Account[] }) {
  const t = useCopy();
  const [sessions, setSessions] = useState<Session[] | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [shown, setShown] = useState(PAGE);
  const [copying, setCopying] = useState<Session | null>(null);

  const load = useCallback(() => {
    setError(null);
    api.sessions().then(setSessions, setError);
  }, []);
  useEffect(load, [load]);

  const destinations = (s: Session) =>
    accounts.filter((a) => a.available && a.name !== s.account);

  return (
    <>
      <div className="banner">
        <span>{t.sessionsBanner}</span>
      </div>
      <ResumeHook />
      {error != null && <Failure error={error} retry={load} />}
      {sessions == null ? (
        error == null && (
          <div className="empty" role="status">
            {t.discoveringSessions}
          </div>
        )
      ) : sessions.length === 0 ? (
        <Empty icon="sessions" title={t.noSessionsTitle}>
          {t.noSessionsBody}
        </Empty>
      ) : (
        <>
          <div className="session-list">
            {sessions.slice(0, shown).map((s) => (
              <SessionRow
                key={`${s.account}:${s.project}:${s.id}`}
                session={s}
                copy={destinations(s).length > 0 ? () => setCopying(s) : null}
              />
            ))}
          </div>
          {shown < sessions.length && (
            <button className="quiet" onClick={() => setShown((n) => n + PAGE)}>
              {t.showMore(sessions.length - shown)}
            </button>
          )}
        </>
      )}
      <p className="footnote">{t.sessionsFootnote}</p>
      {copying && sessions && (
        <CopyDialog
          session={copying}
          destinations={destinations(copying)}
          sessions={sessions}
          copied={setSessions}
          reload={load}
          onClose={() => setCopying(null)}
        />
      )}
    </>
  );
}

/** The `claude` wrapper's `--resume` check, with its own load and errors. */
function ResumeHook() {
  const t = useCopy();
  const [on, setOn] = useState<boolean | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [confirming, setConfirming] = useState(false);
  const load = useCallback(() => {
    setError(null);
    api.resumeHook().then(setOn, setError);
  }, []);
  useEffect(load, [load]);
  if (error != null) return <Failure error={error} retry={load} />;
  if (on == null) return null;
  return (
    <div className="setting-row">
      <div>
        <strong>{t.resumeHookTitle}</strong>
        <span>{on ? t.resumeHookOn : t.resumeHookOff}</span>
      </div>
      <button onClick={() => setConfirming(true)}>
        {on ? t.resumeHookTurnOff : t.resumeHookTurnOn}
      </button>
      {confirming && (
        <Dialog
          title={on ? t.resumeHookOffTitle : t.resumeHookOnTitle}
          confirmLabel={on ? t.resumeHookOffConfirm : t.resumeHookOnConfirm}
          onConfirm={async () => setOn(await api.setResumeHook(!on))}
          onClose={() => setConfirming(false)}
        >
          <p>{on ? t.resumeHookOffBody : t.resumeHookOnBody}</p>
        </Dialog>
      )}
    </div>
  );
}

function SessionRow({ session: s, copy }: { session: Session; copy: (() => void) | null }) {
  const t = useCopy();
  const details = [
    s.modified
      ? new Date(s.modified * 1000).toLocaleString(t.locale)
      : t.modifiedUnknown,
    kilobytes(t, s.size),
    ...(s.duplicate ? [t.copiesElsewhere] : []),
  ];
  return (
    <div className="session-row">
      <Icon name="sessions" />
      <div>
        <strong className="path" title={s.project}>
          {s.project}
        </strong>
        <code>{s.id}</code>
        <span>{details.join(" · ")}</span>
      </div>
      <span className="badge">{s.account}</span>
      <button
        className="quiet"
        disabled={copy == null}
        onClick={copy ?? undefined}
        aria-label={t.copySessionAria(s.id, s.account)}
      >
        {t.copySessionAction}
      </button>
    </div>
  );
}

function CopyDialog({
  session: s,
  destinations,
  sessions,
  copied,
  reload,
  onClose,
}: {
  session: Session;
  destinations: Account[];
  sessions: Session[];
  copied: (sessions: Session[]) => void;
  reload: () => void;
  onClose: () => void;
}) {
  const t = useCopy();
  const { info } = useNotices();
  const [to, setTo] = useState(destinations[0].name);
  // Decides the warning, the button and what is sent, so none can disagree.
  const replace = sessions.some((x) => x.id === s.id && x.account === to);
  return (
    <Dialog
      title={t.copySessionTitle}
      confirmLabel={replace ? t.copySessionReplaceConfirm : t.copySessionConfirm}
      onConfirm={async () => {
        try {
          copied(await api.copySession(s.account, s.project, s.id, to, replace));
        } catch (failure) {
          // The list behind the dialog was out of date: reload it, so the
          // warning and the button follow what is there now.
          reload();
          throw failure;
        }
        info(t.copySessionDone(to, s.id));
      }}
      onClose={onClose}
    >
      <p>{t.copySessionBody(<strong>{s.account}</strong>, <code>{s.id}</code>)}</p>
      <label>
        {t.copySessionDestination}
        <select value={to} onChange={(e) => setTo(e.target.value)}>
          {destinations.map((a) => (
            <option key={a.name} value={a.name}>
              {a.name}
            </option>
          ))}
        </select>
      </label>
      {replace && <p>{t.copySessionReplaceWarning(<strong>{to}</strong>)}</p>}
      <p>{t.copySessionCost}</p>
    </Dialog>
  );
}
