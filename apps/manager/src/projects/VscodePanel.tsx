import { useCallback, useEffect, useState } from "react";
import { api } from "../api";
import { Dialog, Failure } from "../components";
import type { Editor } from "../contract";
import { useCopy } from "../i18n";
import { useNotices } from "../notices";

/**
 * The VS Code process wrapper: the extension's own UI resolves the account
 * from the workspace folder, as a terminal does, once it is set up.
 */
export function VscodePanel() {
  const t = useCopy();
  const { info } = useNotices();
  const [editors, setEditors] = useState<Editor[] | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [open, setOpen] = useState<"install" | "uninstall" | null>(null);
  const [force, setForce] = useState(false);
  const load = useCallback(() => {
    setError(null);
    api.vscodeEditors().then(setEditors, setError);
  }, []);
  useEffect(load, [load]);
  // A failure can come after some editors were written, so the list is read
  // again either way; the restart hint only follows an actual change.
  const apply = async (request: Promise<Editor[]>) => {
    let after: Editor[];
    try {
      after = await request;
    } catch (failure) {
      load();
      throw failure;
    }
    if (JSON.stringify(after) !== JSON.stringify(editors)) info(t.vscodeRestart);
    setEditors(after);
  };
  const close = () => {
    setOpen(null);
    setForce(false);
  };
  // Each decides a button or an option, so it is worked out once.
  const foreign = editors?.some((e) => e.state === "foreign") ?? false;
  const ours = editors?.some((e) => e.state === "ours") ?? false;

  return (
    <>
      <div className="section-toolbar">
        <h2>{t.vscodeTitle}</h2>
      </div>
      {error != null && <Failure error={error} retry={load} />}
      {editors != null && editors.length === 0 && <p className="footnote">{t.vscodeNone}</p>}
      {editors != null && editors.length > 0 && (
        <div className="setting-row">
          <div>
            {editors.map((e) => (
              <span key={e.name} className="editor-state">
                <strong>{e.name}</strong> {t.editorStates[e.state]}
                {e.other_profiles && ` ${t.vscodeOtherProfiles}`}
              </span>
            ))}
          </div>
          <button onClick={() => setOpen("install")}>{t.vscodeInstallAction}</button>
          <button disabled={!ours} onClick={() => setOpen("uninstall")}>
            {t.vscodeUninstallAction}
          </button>
        </div>
      )}
      {open === "install" && (
        <Dialog
          title={t.vscodeInstallTitle}
          confirmLabel={t.vscodeInstallConfirm}
          onConfirm={() => apply(api.vscodeInstall(force))}
          onClose={close}
        >
          <p>{t.vscodeInstallBody}</p>
          {foreign && (
            <label className="check">
              <input type="checkbox" checked={force} onChange={(e) => setForce(e.target.checked)} />
              {t.vscodeReplaceForeign}
            </label>
          )}
        </Dialog>
      )}
      {open === "uninstall" && (
        <Dialog
          title={t.vscodeUninstallTitle}
          confirmLabel={t.vscodeUninstallConfirm}
          onConfirm={() => apply(api.vscodeUninstall())}
          onClose={close}
        >
          <p>{t.vscodeUninstallBody}</p>
        </Dialog>
      )}
    </>
  );
}
