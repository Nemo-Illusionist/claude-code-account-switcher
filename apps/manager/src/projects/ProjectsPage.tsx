import { useCallback, useEffect, useState } from "react";
import { api } from "../api";
import { Dialog, Empty, Failure, Icon } from "../components";
import type { Account, Binding, Bindings } from "../contract";
import { useCopy } from "../i18n";
import { useLaunch, useNotices } from "../notices";
import { VscodePanel } from "./VscodePanel";

type Pending = { kind: "save"; account: string } | { kind: "remove"; binding: Binding } | null;

export function ProjectsPage({ accounts }: { accounts: Account[] }) {
  const t = useCopy();
  const [data, setData] = useState<Bindings>({ bindings: [], selected: null });
  const [error, setError] = useState<unknown>(null);
  const [directory, setDirectory] = useState("");
  const [choice, setChoice] = useState("default");
  const [pending, setPending] = useState<Pending>(null);
  const [busy, setBusy] = useState(false);
  const launch = useLaunch();
  const { fail } = useNotices();
  const selected = data.selected;

  // Every read goes through here, so the list and the selection always come
  // from the same response.
  const load = useCallback(async (path: string | null) => {
    const next = await api.bindings(path);
    setData(next);
    setError(null);
    if (next.selected) {
      setDirectory(next.selected.directory);
      setChoice(next.selected.account);
    }
  }, []);

  const attempt = async (work: () => Promise<void>) => {
    setBusy(true);
    try {
      await work();
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    load(null).catch(setError);
  }, [load]);

  return (
    <>
      {error != null && <Failure error={error} retry={() => void load(null).catch(setError)} />}
      <section className="project-picker">
        <div className="panel-heading">
          <div className="summary-icon">
            <Icon name="projects" />
          </div>
          <div>
            <h2>{t.choosePrompt}</h2>
            <p>{t.chooseLead}</p>
          </div>
        </div>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void attempt(() => load(directory));
          }}
        >
          <label className="sr-only" htmlFor="directory">
            {t.directoryLabel}
          </label>
          <input
            id="directory"
            value={directory}
            onChange={(e) => setDirectory(e.target.value)}
            placeholder={t.directoryPlaceholder}
            required
          />
          <button
            type="button"
            disabled={busy}
            onClick={() =>
              void attempt(async () => {
                const path = await api.chooseDirectory();
                if (path) await load(path);
              })
            }
          >
            {t.browse}
          </button>
          <button type="submit" className="primary" disabled={busy}>
            {t.inspect}
          </button>
        </form>
        {selected && (
          <div className="effective">
            <span className="eyebrow">{t.effectiveAccount}</span>
            <h3>{selected.account}</h3>
            <p className="path">{selected.directory}</p>
            <p>
              {selected.inherited_from
                ? t.inheritedFrom(selected.inherited_from)
                : t.fromDefault}
            </p>
            <div className="binding-actions">
              <select
                aria-label={t.bindingAccountAria}
                value={choice}
                onChange={(e) => setChoice(e.target.value)}
              >
                {accounts.map((a) => (
                  <option key={a.name} value={a.name}>
                    {a.name}
                  </option>
                ))}
              </select>
              <button
                className="primary"
                disabled={busy}
                onClick={() => setPending({ kind: "save", account: choice })}
              >
                {t.saveBinding}
              </button>
              <button
                disabled={busy}
                onClick={() =>
                  void attempt(() => launch(selected.account, selected.directory, "run"))
                }
              >
                {t.openClaude}
                <Icon name="arrow" />
              </button>
            </div>
          </div>
        )}
      </section>
      <div className="section-toolbar">
        <h2>
          {t.directoryBindings} <span>{data.bindings.length}</span>
        </h2>
        <button
          className="quiet"
          disabled={busy}
          onClick={() => void attempt(() => load(selected?.directory ?? null))}
        >
          {t.reloadBindings}
        </button>
      </div>
      {data.bindings.length === 0 ? (
        <Empty icon="projects" title={t.noBindingsTitle}>
          {t.noBindingsBody}
        </Empty>
      ) : (
        <div className="binding-list">
          {data.bindings.map((b) => (
            <div className="binding-row" key={b.directory}>
              <Icon name="projects" />
              <div>
                <strong className="path" title={b.directory}>
                  {b.directory}
                </strong>
                <span>
                  {b.exists ? t.bindingIncludesChildren : t.bindingDirectoryMissing}
                </span>
              </div>
              <span className="badge">{b.account}</span>
              <button
                disabled={!b.exists || busy}
                onClick={() => void attempt(() => load(b.directory))}
              >
                {t.inspect}
              </button>
              <button
                disabled={busy}
                onClick={() => setPending({ kind: "remove", binding: b })}
              >
                {t.remove}
              </button>
            </div>
          ))}
        </div>
      )}
      <p className="footnote">{t.projectsFootnote}</p>
      <VscodePanel />
      {pending?.kind === "save" && selected && (
        <Dialog
          title={t.saveTitle}
          confirmLabel={t.saveBinding}
          onConfirm={async () => {
            await api.bind(selected.directory, pending.account);
            await load(selected.directory);
          }}
          onClose={() => setPending(null)}
        >
          <p>
            {t.saveBody(
              <strong>{pending.account}</strong>,
              <span className="path">{selected.directory}</span>,
            )}
          </p>
        </Dialog>
      )}
      {pending?.kind === "remove" && (
        <Dialog
          title={t.removeTitle}
          confirmLabel={t.removeConfirm}
          onConfirm={async () => {
            await api.bind(pending.binding.directory, null);
            await load(selected?.directory ?? null);
          }}
          onClose={() => setPending(null)}
        >
          <p>{t.removeBody(<span className="path">{pending.binding.directory}</span>)}</p>
        </Dialog>
      )}
    </>
  );
}
