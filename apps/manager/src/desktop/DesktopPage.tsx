import { useCallback, useEffect, useState, type FormEvent } from "react";
import { api } from "../api";
import { Dialog, Empty, Failure, Icon } from "../components";
import type { DesktopAction, DesktopProfile, DesktopProfiles } from "../contract";
import { useCopy } from "../i18n";
import { useNotices } from "../notices";

const NAME = /^[A-Za-z0-9_-]+$/;

type Pending =
  | { kind: "open"; name: string; action: DesktopAction }
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
  remove,
}: {
  profile: DesktopProfile;
  open: () => void;
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
