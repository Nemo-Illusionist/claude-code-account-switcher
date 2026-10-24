import { useState, type FormEvent } from "react";
import { Dialog, Empty, Failure, Icon } from "../components";
import { formatAge } from "../freshness";
import { useNow } from "../hooks";
import { useCopy } from "../i18n";
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
  const t = useCopy();
  const { snapshot, error, loading, refresh, setDefault, removeAccount, pinAccount } = state;
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
            <span>{t.availableAccounts}</span>
          </div>
        </div>
        <div>
          <span className="summary-icon">★</span>
          <div>
            <strong className="default-name">{snapshot?.default_account ?? "—"}</strong>
            <span>{t.configuredDefault}</span>
          </div>
        </div>
        <div>
          <span className="summary-icon">
            <Icon name="refresh" />
          </span>
          <div>
            <strong>
              {snapshot ? formatAge(t, Math.max(0, now - snapshot.observed_at)) : "—"}
            </strong>
            <span>{t.lastChecked}</span>
          </div>
        </div>
      </div>
      <div className="section-toolbar">
        <h2>
          {t.allAccounts} <span>{accounts.length}</span>
        </h2>
        <input
          className="search"
          aria-label={t.searchAria}
          placeholder={t.searchPlaceholder}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
      </div>
      {loading ? (
        <div role="status" className="empty">
          {t.readingAccounts}
        </div>
      ) : !snapshot ? (
        <Empty title={t.accountsUnavailableTitle}>{t.accountsUnavailableBody}</Empty>
      ) : accounts.every((a) => !a.available) ? (
        <Empty title={t.noAccountsTitle}>{t.noAccountsBody}</Empty>
      ) : shown.length === 0 ? (
        <Empty title={t.noMatchTitle}>{t.noMatchBody}</Empty>
      ) : (
        <div className="account-grid">
          {shown.map((a, i) => (
            <AccountCard
              key={a.name}
              account={a}
              index={i}
              now={now}
              setDefault={setDefault}
              removeAccount={removeAccount}
              pinAccount={pinAccount}
            />
          ))}
        </div>
      )}
      <p className="footnote">{t.accountsFootnote}</p>
    </>
  );
}

function AddAccount({ onDone }: { onDone: () => void }) {
  const t = useCopy();
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
          {t.accountName}
          <input
            autoFocus
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder={t.accountNamePlaceholder}
            aria-invalid={invalid}
            required
          />
        </label>
        <button className="primary" type="submit">
          {t.continue}
        </button>
        <button type="button" onClick={onDone}>
          {t.cancel}
        </button>
        {invalid && (
          <p className="field-error" role="alert">
            {t.accountNameInvalid}
          </p>
        )}
      </form>
      {confirming && (
        <Dialog
          title={t.addTitle(name)}
          confirmLabel={t.addConfirm}
          onConfirm={async () => {
            await launch(name, null, "add");
            onDone();
          }}
          onClose={() => setConfirming(false)}
        >
          <p>{t.addBody}</p>
        </Dialog>
      )}
    </>
  );
}
