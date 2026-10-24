import { useCallback, useEffect, useState, type FormEvent } from "react";
import { api } from "../api";
import { Dialog, Empty, Failure, Icon } from "../components";
import type { DesktopAction, DesktopProfile, DesktopProfiles } from "../contract";
import { gigabytes, useCopy } from "../i18n";
import { useNotices } from "../notices";

const NAME = /^[A-Za-z0-9_-]+$/;

type Pending =
  | { kind: "open"; name: string; action: DesktopAction }
  | { kind: "copy"; name: string }
  | { kind: "remove"; name: string }
  | null;

export function DesktopPage() {
  const t = useCopy();
  const [data, setData] = useState<DesktopProfiles | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [pending, setPending] = useState<Pending>(null);
  const load = useCallback(() => {
    setError(null);
    api.desktopProfiles().then(setData, setError);
  }, []);
  useEffect(load, [load]);

  return (
    <>
      <div className="banner">
        <span>{t.desktopBanner}</span>
      </div>
      <AddProfile
        canSeed={data?.app_profile_exists ?? false}
        add={(name, action) => setPending({ kind: "open", name, action })}
      />
      {error != null && <Failure error={error} retry={load} />}
      {data == null ? (
        error == null && (
          <div className="empty" role="status">
            {t.desktopReading}
          </div>
        )
      ) : data.profiles.length === 0 ? (
        <Empty icon="desktop" title={t.desktopNoneTitle}>
          {t.desktopNoneBody}
        </Empty>
      ) : (
        <div className="session-list">
          {data.profiles.map((p) => (
            <ProfileRow
              key={p.name}
              profile={p}
              open={() => setPending({ kind: "open", name: p.name, action: "open" })}
              copy={() => setPending({ kind: "copy", name: p.name })}
              remove={() => setPending({ kind: "remove", name: p.name })}
            />
          ))}
        </div>
      )}
      {data?.app_profile_exists && <p className="footnote">{t.desktopAppProfile}</p>}
      {pending?.kind === "open" && (
        <OpenDialog
          name={pending.name}
          action={pending.action}
          opened={setData}
          failed={load}
          onClose={() => setPending(null)}
        />
      )}
      {pending?.kind === "copy" && data && (
        <CopyDialog
          name={pending.name}
          sources={[
            ...(data.app_profile_exists ? [null] : []),
            ...data.profiles.map((p) => p.name).filter((n) => n !== pending.name),
          ]}
          onClose={() => setPending(null)}
        />
      )}
      {pending?.kind === "remove" && (
        <Dialog
          title={t.desktopTrashTitle(pending.name)}
          confirmLabel={t.trashConfirm}
          onConfirm={async () => {
            try {
              setData(await api.desktopRemove(pending.name));
            } catch (failure) {
              load();
              throw failure;
            }
          }}
          onClose={() => setPending(null)}
        >
          <p>{t.desktopTrashBody(<strong>{pending.name}</strong>)}</p>
        </Dialog>
      )}
    </>
  );
}

function ProfileRow({
  profile: p,
  open,
  copy,
  remove,
}: {
  profile: DesktopProfile;
  open: () => void;
  copy: () => void;
  remove: () => void;
}) {
  const t = useCopy();
  return (
    <div className="session-row">
      <Icon name="desktop" />
      <div>
        <strong>{p.name}</strong>
        <span>
          {p.signed_in ? (p.email ?? t.desktopSignedIn) : t.desktopSignedOut}
        </span>
      </div>
      <button onClick={open}>{t.desktopOpen}</button>
      <button className="quiet" onClick={copy}>
        {t.desktopCopyAction}
      </button>
      <button className="quiet" onClick={remove}>
        {t.trashAction}
      </button>
    </div>
  );
}

function AddProfile({
  canSeed,
  add,
}: {
  /** Seeding copies from the app's own profile, so only offered when it exists. */
  canSeed: boolean;
  add: (name: string, action: DesktopAction) => void;
}) {
  const t = useCopy();
  const [name, setName] = useState("");
  const [invalid, setInvalid] = useState(false);
  const [seed, setSeed] = useState(false);
  const submit = (e: FormEvent) => {
    e.preventDefault();
    const ok = NAME.test(name) && name !== "default";
    setInvalid(!ok);
    if (ok) add(name, seed && canSeed ? "add_seeded" : "add");
  };
  return (
    <form className="add-form" onSubmit={submit}>
      <label>
        {t.desktopNewProfile}
        <input
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder={t.accountNamePlaceholder}
          aria-invalid={invalid}
          required
        />
      </label>
      <button className="primary" type="submit">
        {t.desktopAdd}
      </button>
      {canSeed && (
        <label className="check">
          <input type="checkbox" checked={seed} onChange={(e) => setSeed(e.target.checked)} />
          {t.desktopSeedLabel}
        </label>
      )}
      {invalid && (
        <p className="field-error" role="alert">
          {t.accountNameInvalid}
        </p>
      )}
    </form>
  );
}

/**
 * Opens a profile, adding it first for an add. If another Claude Desktop is
 * running when a sign-in is due, the same dialog offers to go ahead anyway.
 */
function OpenDialog({
  name,
  action,
  opened,
  failed,
  onClose,
}: {
  name: string;
  action: DesktopAction;
  opened: (data: DesktopProfiles) => void;
  /** An add can fail after the profile was created, so the list is read again. */
  failed: () => void;
  onClose: () => void;
}) {
  const t = useCopy();
  const { info } = useNotices();
  const [force, setForce] = useState(false);
  const adding = action !== "open";
  return (
    <Dialog
      title={adding ? t.desktopAddTitle(name) : t.desktopOpenTitle(name)}
      confirmLabel={force ? t.desktopOpenAnyway : adding ? t.desktopAdd : t.desktopOpen}
      onConfirm={async () => {
        try {
          opened(await api.desktopOpen(name, action, force));
        } catch (failure) {
          if (failure === "desktop_running") setForce(true);
          failed();
          throw failure;
        }
        info(adding ? t.desktopAdded(name) : t.desktopOpened(name));
      }}
      onClose={onClose}
    >
      <p>{adding ? t.desktopAddBody : t.desktopOpenBody(<strong>{name}</strong>)}</p>
    </Dialog>
  );
}

type What = "config" | "runtime";

/**
 * Copies the MCP config, or shares the downloaded runtime, into a profile.
 * `null` as a source is Claude Desktop's own profile. If the profile has its
 * own already, the same dialog asks again to replace it.
 */
function CopyDialog({
  name,
  sources,
  onClose,
}: {
  name: string;
  sources: (string | null)[];
  onClose: () => void;
}) {
  const t = useCopy();
  const { info } = useNotices();
  const [what, setWhat] = useState<What>("config");
  const [from, setFrom] = useState<string | null>(sources[0] ?? null);
  const [force, setForce] = useState(false);
  const fromLabel = (source: string | null) => source ?? t.desktopAppProfileName;
  const choose = <T,>(set: (value: T) => void) => (value: T) => {
    set(value);
    setForce(false);
  };
  if (sources.length === 0) {
    return (
      <Dialog title={t.desktopCopyTitle(name)} onClose={onClose}>
        <p>{t.desktopCopyNoSource}</p>
      </Dialog>
    );
  }
  return (
    <Dialog
      // A new choice is a new question: an earlier one's error goes with it.
      key={`${what}:${from ?? ""}`}
      title={t.desktopCopyTitle(name)}
      confirmLabel={force ? t.desktopCopyReplace : t.desktopCopyConfirm}
      onConfirm={async () => {
        try {
          if (what === "config") {
            await api.desktopCloneConfig(name, from, force);
            info(t.desktopConfigCopied(name, fromLabel(from)));
          } else {
            const done = await api.desktopCloneRuntime(name, from, force);
            const lost = done.on_disk == null ? null : gigabytes(t, done.on_disk);
            info(t.desktopRuntimeCloned(name, fromLabel(from), gigabytes(t, done.logical), lost));
          }
        } catch (failure) {
          if (failure === "desktop_already_has") setForce(true);
          throw failure;
        }
      }}
      onClose={onClose}
    >
      <label>
        {t.desktopCopyWhat}
        <select value={what} onChange={(e) => choose(setWhat)(e.target.value as What)}>
          <option value="config">{t.desktopCopyConfig}</option>
          <option value="runtime">{t.desktopCopyRuntime}</option>
        </select>
      </label>
      <label>
        {t.desktopCopyFrom}
        <select
          value={from ?? ""}
          onChange={(e) => choose(setFrom)(e.target.value === "" ? null : e.target.value)}
        >
          {sources.map((source) => (
            <option key={source ?? ""} value={source ?? ""}>
              {fromLabel(source)}
            </option>
          ))}
        </select>
      </label>
      <p>{what === "config" ? t.desktopCopyConfigBody : t.desktopCopyRuntimeBody}</p>
    </Dialog>
  );
}
