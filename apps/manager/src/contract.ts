// The JSON the Rust backend sends, mirrored from `src/gui.rs` and the manager's
// shell crate. `contract.sample.json` is checked against both sides, so a field
// renamed on one side fails a test on the other.

export type IdentitySource =
  | "local"
  | "live"
  | "unknown"
  | "offline"
  | "no_credentials";
export type LockStatus = "matched" | "drift" | "unpinned" | "unknown" | "corrupt";
export type UsageSource = "live" | "cached" | "no_credentials" | "unavailable";

export type UsageWindow = {
  percent: number;
  /** Unix seconds; the reading means nothing once this has passed. */
  resets_at: number | null;
};

export type Usage = {
  source: UsageSource;
  /** Unix seconds when the reading was taken, not when it was sent. */
  fetched_at: number | null;
  five_hour: UsageWindow | null;
  seven_day: UsageWindow | null;
};

export type Account = {
  name: string;
  email: string | null;
  plan: string | null;
  organization: string | null;
  is_default: boolean;
  available: boolean;
  identity_source: IdentitySource;
  lock: LockStatus;
  usage: Usage;
};

export type Snapshot = {
  /** Higher is newer, whichever path (reply or event) delivered it. */
  revision: number;
  observed_at: number;
  default_account: string;
  accounts: Account[];
};

export type Binding = { directory: string; account: string; exists: boolean };
export type Bindings = {
  bindings: Binding[];
  selected: {
    directory: string;
    account: string;
    inherited_from: string | null;
  } | null;
};

export type Session = {
  id: string;
  account: string;
  project: string;
  modified: number | null;
  size: number;
  duplicate: boolean;
};

export type LaunchAction = "run" | "login" | "add" | "add_seeded";
export type Launch = { state: "started" | "pending" };

export type Installation = {
  bundled_version: string;
  bundled_available: boolean;
  installed_version: string | null;
};

export type GuiError =
  | "invalid_account"
  | "account_missing"
  | "account_exists"
  | "symlink_storage"
  | "storage_unreadable"
  | "storage_write_failed"
  | "invalid_directory"
  | "unrepresentable_directory"
  | "directory_missing"
  | "shared_credential_store"
  | "identity_drift"
  | "trash_unavailable"
  | "not_signed_in"
  | "session_missing"
  | "session_exists"
  | "import_source_refused"
  | "cli_not_installed"
  | "statusline_exists";
export type ManagerError =
  | "cli_missing"
  | "launcher_failed"
  | "terminal_failed"
  | "terminal_macos_only"
  | "backend_failed";
export type ErrorCode = GuiError | ManagerError;
