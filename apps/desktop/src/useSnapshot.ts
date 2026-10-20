import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import type { Snapshot } from "./contract";

export type SnapshotState = {
  snapshot: Snapshot | null;
  /** The last refresh failed; `snapshot` still holds the last good one. */
  error: unknown;
  loading: boolean;
  refreshing: boolean;
  refresh: (live: boolean) => Promise<void>;
  setDefault: (account: string) => Promise<void>;
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

  useEffect(() => {
    void refresh(false);
    const unlisten = listen<Snapshot>("snapshot", (e) => apply(e.payload));
    return () => void unlisten.then((stop) => stop()).catch(() => {});
  }, [apply, refresh]);

  return { snapshot, error, loading, refreshing, refresh, setDefault };
}
