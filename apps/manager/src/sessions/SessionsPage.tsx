import { useCallback, useEffect, useState } from "react";
import { api } from "../api";
import { Empty, Failure, Icon } from "../components";
import type { Session } from "../contract";
import { kilobytes, useCopy } from "../i18n";

/** Rows rendered per step. The list itself is metadata only and cheap. */
export const PAGE = 100;

export function SessionsPage() {
  const t = useCopy();
  const [sessions, setSessions] = useState<Session[] | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [shown, setShown] = useState(PAGE);

  const load = useCallback(() => {
    setError(null);
    api.sessions().then(setSessions, setError);
  }, []);
  useEffect(load, [load]);

  return (
    <>
      <div className="banner">
        <span>{t.sessionsBanner}</span>
      </div>
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
              <SessionRow key={`${s.account}:${s.project}:${s.id}`} session={s} />
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
    </>
  );
}

function SessionRow({ session: s }: { session: Session }) {
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
    </div>
  );
}
