import { useCallback, useEffect, useRef, useState } from "react";
import { api, events } from "./api";
import type { Snapshot } from "./contract";

export type SnapshotState = {
  snapshot: Snapshot | null;
  /** The last refresh failed; `snapshot` still holds the last good one. */
  error: unknown;
  loading: boolean;
  refreshing: boolean;
  refresh: (live: boolean) => Promise<void>;
  setDefault: (account: string) => Promise<void>;
  removeAccount: (account: string) => Promise<void>;
  pinAccount: (account: string, force: boolean) => Promise<void>;
  importAccount: (account: string, source: string, moveInto: boolean) => Promise<void>;
};

/**
 * The account snapshot, owned by the Rust backend. Replies and `snapshot`
 * events (the tray refreshes without the window) arrive in any order; the
 * backend numbers them, so an older one never replaces a newer one.
 */
export function useSnapshot(): SnapshotState {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const live = useRef(0);

  const apply = useCallback((next: Snapshot) => {
    setSnapshot((current) =>
      current && current.revision > next.revision ? current : next,
    );
    setError(null);
  }, []);

  const refresh = useCallback(
    async (isLive: boolean) => {
      if (isLive) setRefreshing(++live.current > 0);
      try {
        apply(await api.snapshot(isLive));
      } catch (e) {
        setError(e);
      } finally {
        if (isLive) setRefreshing(--live.current > 0);
        setLoading(false);
      }
    },
    [apply],
  );

  const setDefault = useCallback(
    async (account: string) => apply(await api.setDefault(account)),
    [apply],
  );

  const removeAccount = useCallback(
    async (account: string) => apply(await api.removeAccount(account)),
    [apply],
  );

  const pinAccount = useCallback(
    async (account: string, force: boolean) => apply(await api.pinAccount(account, force)),
    [apply],
  );

  const importAccount = useCallback(
    async (account: string, source: string, moveInto: boolean) =>
      apply(await api.importAccount(account, source, moveInto)),
    [apply],
  );

  useEffect(() => {
    void refresh(false);
    return events.snapshot(apply);
  }, [apply, refresh]);

  return {
    snapshot,
    error,
    loading,
    refreshing,
    refresh,
    setDefault,
    removeAccount,
    pinAccount,
    importAccount,
  };
}
