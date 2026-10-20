import { useCallback, useEffect, useState } from "react";
import { api } from "../api";
import { Dialog, Empty, Failure, Icon } from "../components";
import type { Account, Binding, Bindings } from "../contract";
import { useLaunch, useNotices } from "../notices";

type Pending = { kind: "save"; account: string } | { kind: "remove"; binding: Binding } | null;

export function ProjectsPage({ accounts }: { accounts: Account[] }) {
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
            <h2>Choose a project</h2>
            <p>Inspect the account resolved by the CLI’s directory rules.</p>
          </div>
        </div>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void attempt(() => load(directory));
          }}
        >
          <label className="sr-only" htmlFor="directory">
            Project directory
          </label>
          <input
            id="directory"
            value={directory}
            onChange={(e) => setDirectory(e.target.value)}
            placeholder="/Users/you/Projects/my-project"
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
            Browse…
          </button>
          <button type="submit" className="primary" disabled={busy}>
            Inspect
          </button>
        </form>
        {selected && (
          <div className="effective">
            <span className="eyebrow">EFFECTIVE ACCOUNT</span>
            <h3>{selected.account}</h3>
            <p className="path">{selected.directory}</p>
            <p>
              {selected.inherited_from
                ? `From binding: ${selected.inherited_from}`
                : "From configured default (no parent-directory binding)"}
            </p>
            <div className="binding-actions">
              <select
                aria-label="Account for selected project"
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
                Save binding
              </button>
              <button
                disabled={busy}
                onClick={() =>
                  void attempt(() => launch(selected.account, selected.directory, "run"))
                }
              >
                Open Claude
                <Icon name="arrow" />
              </button>
            </div>
          </div>
        )}
      </section>
      <div className="section-toolbar">
        <h2>
          Directory bindings <span>{data.bindings.length}</span>
        </h2>
        <button
          className="quiet"
          disabled={busy}
          onClick={() => void attempt(() => load(selected?.directory ?? null))}
        >
          Reload bindings
        </button>
      </div>
      {data.bindings.length === 0 ? (
        <Empty icon="projects" title="Give your projects an account">
          Choose a folder above to create your first binding.
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
                  {b.exists
                    ? "Includes child directories unless overridden"
                    : "Directory missing — binding is still stored"}
                </span>
              </div>
              <span className="badge">{b.account}</span>
              <button
                disabled={!b.exists || busy}
                onClick={() => void attempt(() => load(b.directory))}
              >
                Inspect
              </button>
              <button
                disabled={busy}
                onClick={() => setPending({ kind: "remove", binding: b })}
              >
                Remove
              </button>
            </div>
          ))}
        </div>
      )}
      <p className="footnote">
        The closest parent binding wins. A binding to “default” explicitly
        selects the standard ~/.claude account.
      </p>
      {pending?.kind === "save" && selected && (
        <Dialog
          title="Save project binding?"
          confirmLabel="Save binding"
          onConfirm={async () => {
            await api.bind(selected.directory, pending.account);
            await load(selected.directory);
          }}
          onClose={() => setPending(null)}
        >
          <p>
            Use <strong>{pending.account}</strong> for{" "}
            <span className="path">{selected.directory}</span> and its child
            directories, except where a closer binding exists.
          </p>
        </Dialog>
      )}
      {pending?.kind === "remove" && (
        <Dialog
          title="Remove directory binding?"
          confirmLabel="Remove binding"
          onConfirm={async () => {
            await api.bind(pending.binding.directory, null);
            await load(selected?.directory ?? null);
          }}
          onClose={() => setPending(null)}
        >
          <p>
            Remove the binding for{" "}
            <span className="path">{pending.binding.directory}</span>. Future
            resolution will fall back to a parent binding or the configured
            default.
          </p>
        </Dialog>
      )}
    </>
  );
}
