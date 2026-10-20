import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { api, errorMessage, launchMessage } from "./api";
import type { LaunchAction } from "./contract";

type Notice = { kind: "info" | "error"; text: string };
type Notices = {
  notice: Notice | null;
  info: (text: string) => void;
  fail: (error: unknown) => void;
  dismiss: () => void;
};

const NoticeContext = createContext<Notices | null>(null);

/** One banner for results that outlive the control that caused them. */
export function NoticeProvider({ children }: { children: ReactNode }) {
  const [notice, setNotice] = useState<Notice | null>(null);
  const info = useCallback((text: string) => setNotice({ kind: "info", text }), []);
  const fail = useCallback(
    (error: unknown) => setNotice({ kind: "error", text: errorMessage(error) }),
    [],
  );
  const dismiss = useCallback(() => setNotice(null), []);
  const value = useMemo(
    () => ({ notice, info, fail, dismiss }),
    [notice, info, fail, dismiss],
  );
  return (
    <NoticeContext.Provider value={value}>{children}</NoticeContext.Provider>
  );
}

export function useNotices(): Notices {
  const notices = useContext(NoticeContext);
  if (!notices) throw new Error("useNotices outside NoticeProvider");
  return notices;
}

/** Open the bundled CLI in Terminal and report what is actually known. */
export function useLaunch() {
  const { info } = useNotices();
  return useCallback(
    async (account: string, directory: string | null, action: LaunchAction) => {
      info(launchMessage(account, await api.terminal(account, directory, action)));
    },
    [info],
  );
}

export function NoticeBanner() {
  const { notice, dismiss } = useNotices();
  if (!notice) return null;
  return (
    <div
      className={`banner ${notice.kind === "error" ? "error" : ""}`}
      role={notice.kind === "error" ? "alert" : "status"}
    >
      <span>{notice.text}</span>
      <button className="quiet" aria-label="Dismiss notification" onClick={dismiss}>
        ×
      </button>
    </div>
  );
}
