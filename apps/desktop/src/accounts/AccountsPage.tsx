import { useState, type FormEvent } from "react";
import { Dialog, Empty, Failure, Icon } from "../components";
import { formatAge } from "../freshness";
import { useNow } from "../hooks";
import { useLaunch } from "../notices";
import type { SnapshotState } from "../useSnapshot";
import { AccountCard } from "./AccountCard";

const NAME = /^[A-Za-z0-9_-]+$/;

export function AccountsPage({
  state,
  adding,
  setAdding,
}: {
  state: SnapshotState;
  adding: boolean;
  setAdding: (adding: boolean) => void;
}) {
  const { snapshot, error, loading, refresh, setDefault } = state;
  const [search, setSearch] = useState("");
  const now = useNow();
  const accounts = snapshot?.accounts ?? [];
  const query = search.toLowerCase();
  const shown = accounts.filter((a) =>
    `${a.name} ${a.email ?? ""}`.toLowerCase().includes(query),
  );

  return (
    <>
      {adding && <AddAccount onDone={() => setAdding(false)} />}
      {error != null && <Failure error={error} retry={() => void refresh(false)} />}
      <div className="summary">
        <div>
          <span className="summary-icon">
            <Icon name="accounts" />
          </span>
          <div>
            <strong>{accounts.filter((a) => a.available).length}</strong>
            <span>Available accounts</span>
          </div>
        </div>
        <div>
          <span className="summary-icon">★</span>
          <div>
            <strong className="default-name">{snapshot?.default_account ?? "—"}</strong>
            <span>Configured default</span>
          </div>
        </div>
        <div>
          <span className="summary-icon">
            <Icon name="refresh" />
          </span>
          <div>
            <strong>{snapshot ? formatAge(Math.max(0, now - snapshot.observed_at)) : "—"}</strong>
            <span>Last checked</span>
          </div>
        </div>
      </div>
      <div className="section-toolbar">
        <h2>
          All accounts <span>{accounts.length}</span>
        </h2>
        <input
          className="search"
          aria-label="Search accounts"
          placeholder="Search accounts or email…"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
      </div>
      {loading ? (
        <div role="status" className="empty">
          Reading account configuration…
        </div>
      ) : !snapshot ? (
        <Empty title="Account data unavailable">Retry to reach the desktop backend.</Empty>
      ) : accounts.every((a) => !a.available) ? (
        <Empty title="No configured accounts yet">
          Add an account to open the existing Claude login flow in Terminal.
        </Empty>
      ) : shown.length === 0 ? (
        <Empty title="No matching accounts">Try another name or email address.</Empty>
      ) : (
        <div className="account-grid">
          {shown.map((a, i) => (
            <AccountCard
              key={a.name}
              account={a}
              index={i}
              now={now}
              setDefault={setDefault}
            />
          ))}
        </div>
      )}
      <p className="footnote">
        Usage is read from Claude’s API only when you refresh. Every reading
        shows its age, and becomes stale after ten minutes. No account rotation
        or background session sharing.
      </p>
    </>
  );
}

function AddAccount({ onDone }: { onDone: () => void }) {
  const [name, setName] = useState("");
  const [invalid, setInvalid] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const launch = useLaunch();
  const submit = (e: FormEvent) => {
    e.preventDefault();
    const ok = NAME.test(name) && name !== "default";
    setInvalid(!ok);
    setConfirming(ok);
  };
  return (
    <>
      <form className="add-form" onSubmit={submit}>
        <label>
          Account name
          <input
            autoFocus
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="e.g. work"
            aria-invalid={invalid}
            required
          />
        </label>
        <button className="primary" type="submit">
          Continue
        </button>
        <button type="button" onClick={onDone}>
          Cancel
        </button>
        {invalid && (
          <p className="field-error" role="alert">
            Use letters, numbers, hyphens, or underscores. “default” is reserved.
          </p>
        )}
      </form>
      {confirming && (
        <Dialog
          title={`Add ${name}?`}
          confirmLabel="Create & authenticate"
          onConfirm={async () => {
            await launch(name, null, "add");
            onDone();
          }}
          onClose={() => setConfirming(false)}
        >
          <p>
            Create an isolated account using the CLI and open its login flow in
            Terminal. Existing settings and conversations are not copied.
          </p>
        </Dialog>
      )}
    </>
  );
}
