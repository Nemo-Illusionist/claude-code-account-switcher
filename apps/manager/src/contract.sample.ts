// `contract.sample.json` written as typed TypeScript. The compiler checks this
// object against `contract.ts`, `contract.test.ts` checks it equals the JSON,
// and the Rust tests check the JSON against what the backend serializes.
import type {
  Bindings,
  ManagerError,
  GuiError,
  Installation,
  Launch,
  Session,
  Snapshot,
} from "./contract";

export const sample: {
  snapshot: Snapshot;
  bindings: Bindings;
  sessions: Session[];
  launch: Launch;
  installation: Installation;
  gui_errors: GuiError[];
  manager_errors: ManagerError[];
} = {
  snapshot: {
    revision: 3,
    observed_at: 1700000900,
    default_account: "work",
    accounts: [
      {
        name: "default",
        email: null,
        plan: null,
        organization: null,
        is_default: false,
        available: false,
        identity_source: "unknown",
        lock: "unknown",
        usage: {
          source: "unavailable",
          fetched_at: null,
          five_hour: null,
          seven_day: null,
        },
      },
      {
        name: "work",
        email: "developer@example.test",
        plan: "Pro",
        organization: "Example Org",
        is_default: true,
        available: true,
        identity_source: "local",
        lock: "matched",
        usage: {
          source: "cached",
          fetched_at: 1700000000,
          five_hour: { percent: 42, resets_at: 1700007200 },
          seven_day: { percent: 97.5, resets_at: 1699990000 },
        },
      },
      {
        name: "personal",
        email: "personal@example.test",
        plan: null,
        organization: null,
        is_default: false,
        available: true,
        identity_source: "live",
        lock: "drift",
        usage: {
          source: "live",
          fetched_at: 1700000900,
          five_hour: { percent: 5, resets_at: null },
          seven_day: null,
        },
      },
    ],
  },
  bindings: {
    bindings: [{ directory: "/Projects", account: "work", exists: true }],
    selected: {
      directory: "/Projects/nested",
      account: "work",
      inherited_from: "/Projects",
    },
  },
  sessions: [
    {
      id: "11111111-2222-3333-4444-555555555555",
      account: "work",
      project: "-Projects",
      modified: 1700000000,
      size: 2048,
      duplicate: false,
    },
  ],
  launch: { state: "started" },
  installation: {
    bundled_version: "0.21.0",
    bundled_available: true,
    installed_version: null,
  },
  gui_errors: [
    "invalid_account",
    "account_missing",
    "account_exists",
    "symlink_storage",
    "storage_unreadable",
    "storage_write_failed",
    "invalid_directory",
    "unrepresentable_directory",
    "directory_missing",
    "shared_credential_store",
    "identity_drift",
    "trash_unavailable",
  ],
  manager_errors: [
    "cli_missing",
    "launcher_failed",
    "terminal_failed",
    "terminal_macos_only",
    "backend_failed",
  ],
};
