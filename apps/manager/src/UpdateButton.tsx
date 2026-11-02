import { useState } from "react";
import { api } from "./api";
import { Dialog } from "./components";
import { describeUpdate, useCopy } from "./i18n";
import { useNotices } from "./notices";

/** `claude-acc update --check`, asked for, never on its own. */
export function UpdateButton() {
  const t = useCopy();
  const { info } = useNotices();
  const [open, setOpen] = useState(false);
  return (
    <>
      <button className="quiet" onClick={() => setOpen(true)}>
        {t.updateCheckAction}
      </button>
      {open && (
        <Dialog
          title={t.updateCheckTitle}
          confirmLabel={t.updateCheckConfirm}
          onConfirm={async () => info(describeUpdate(t, await api.checkUpdate()))}
          onClose={() => setOpen(false)}
        >
          <p>{t.updateCheckBody}</p>
        </Dialog>
      )}
    </>
  );
}
