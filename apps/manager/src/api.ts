// The only module that talks to the Tauri runtime. Every request and every
// event subscription goes through here, so changing how the backend is
// reached, or how its updates arrive, touches this file and nothing else.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  Bindings,
  Installation,
  Launch,
  LaunchAction,
  Session,
  Snapshot,
} from "./contract";

export const api = {
  snapshot: (live: boolean) => invoke<Snapshot>("snapshot", { live }),
  setDefault: (account: string) => invoke<Snapshot>("set_default", { account }),
  removeAccount: (account: string) =>
    invoke<Snapshot>("remove_account", { account }),
  pinAccount: (account: string, force: boolean) =>
    invoke<Snapshot>("pin_account", { account, force }),
  bindings: (directory: string | null) =>
    invoke<Bindings>("list_bindings", { directory }),
  bind: (directory: string, account: string | null) =>
    invoke<void>("set_binding", { directory, account }),
  sessions: () => invoke<Session[]>("list_sessions"),
  copySession: (from: string, project: string, id: string, to: string, replace: boolean) =>
    invoke<Session[]>("copy_session", { from, project, id, to, replace }),
  terminal: (account: string, directory: string | null, action: LaunchAction) =>
    invoke<Launch>("open_terminal", { account, directory, action }),
  installation: () => invoke<Installation>("installation"),
  chooseDirectory: () => invoke<string | null>("choose_directory"),
};

export type Unsubscribe = () => void;

function on<T>(event: string, handler: (payload: T) => void): Unsubscribe {
  const stop = listen<T>(event, (e) => handler(e.payload));
  return () => void stop.then((unlisten) => unlisten()).catch(() => {});
}

/**
 * What the backend pushes without being asked: the menu bar refreshes and
 * navigates with the window closed. Each returns its own unsubscribe, which
 * is what an effect's cleanup wants.
 */
export const events = {
  snapshot: (handler: (snapshot: Snapshot) => void) => on("snapshot", handler),
  navigate: (handler: (page: string) => void) => on("navigate", handler),
  backendError: (handler: (code: unknown) => void) =>
    on("backend-error", handler),
};
