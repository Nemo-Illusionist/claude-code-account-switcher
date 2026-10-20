import { useCallback, useEffect, useState } from "react";
import { api } from "../api";
import { Empty, Failure, Icon } from "../components";
import type { Session } from "../contract";

/** Rows rendered per step. The list itself is metadata only and cheap. */
export const PAGE = 100;

export function SessionsPage() {
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
        <span>
          Session handoff is available through the existing CLI. This browser
          reads metadata only and preserves every transcript.
        </span>
      </div>
      {error != null && <Failure error={error} retry={load} />}
      {sessions == null ? (
        error == null && (
          <div className="empty" role="status">
            Discovering session metadata…
          </div>
        )
      ) : sessions.length === 0 ? (
        <Empty icon="sessions" title="No sessions found">
          Start Claude Code under an account to create a conversation.
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
              Show more ({sessions.length - shown} remaining)
            </button>
          )}
        </>
      )}
      <p className="footnote">
        Project names are the CLI’s stored directory slugs, not reversible
        filesystem paths. Copying and resuming remain a CLI workflow in this MVP.
      </p>
    </>
  );
}

function SessionRow({ session: s }: { session: Session }) {
  return (
    <div className="session-row">
      <Icon name="sessions" />
      <div>
        <strong className="path" title={s.project}>
          {s.project}
        </strong>
        <code>{s.id}</code>
        <span>
          {s.modified
            ? new Date(s.modified * 1000).toLocaleString()
            : "Modified time unknown"}{" "}
          · {(s.size / 1024).toFixed(1)} KB
          {s.duplicate ? " · Copies in multiple accounts" : ""}
        </span>
      </div>
      <span className="badge">{s.account}</span>
    </div>
  );
}
