// Every piece of prose in the manager lives here (#166). `index.html` keeps a
// title and `lang` of its own only until `CopyProvider` replaces them.
//
// Every language supplies the same keys with the same shapes as English.
// Nothing checks that a value is translated, and nothing falls back from one
// language to another.
//
// Which ways of writing a table `tsc` catches mistakes in, and which slip, is
// in `copy.types.test.ts` as compiled cases, not here. Keep the
// `satisfies Record<…>` sub-tables as they are: a cast would accept a missing
// state.
//
// While English is the only language, the shape comparison in `copy.test.ts`
// compares English with itself and catches nothing; it starts to matter with
// the second one.
//
// Values that interpolate are functions, and their arguments are typed. A
// translation that ignores an argument, or drops the ReactNode it is handed
// for inline markup, still compiles, so check that in review.
import { createContext, useContext, useLayoutEffect, type ReactNode } from "react";
import type {
  ErrorCode,
  IdentitySource,
  Launch,
  LockStatus,
  UsageSource,
} from "./contract";

const en = {
  locale: "en",

  productName: "Claude Accounts",
  productTagline: "PROJECT-AWARE WORKSPACE",
  eyebrow: "CLAUDE CODE · ACCOUNT MANAGER",
  dialogEyebrow: "ACCOUNT MANAGER",
  navHeading: "WORKSPACE",
  navAria: "Main navigation",
  breadcrumbRoot: "Workspace",
  pages: {
    accounts: {
      label: "Accounts",
      title: "Your accounts",
      lead: "A clear view of every identity. The right account for every project.",
    },
    projects: {
      label: "Projects",
      title: "Project bindings",
      lead: "Choose once. The account follows the directory and its children.",
    },
    sessions: {
      label: "Sessions",
      title: "Session library",
      lead: "Discover conversations across accounts without opening their content.",
    },
  },
  sidebarNoteTitle: "Directory-based by design",
  sidebarNoteBody:
    "Each terminal has its own account. Choosing a default never switches a running session.",
  appearance: "Appearance",
  themes: { system: "System", dark: "Dark", light: "Light" },
  liveRefresh: "Live refresh",
  liveRefreshAria: "Live refresh interval",
  manualOnly: "Manual only",
  everyMinutes: (minutes: number) => `Every ${minutes} min`,
  sidebarFooter: "LOCAL & PRIVATE",
  version: (version: string) => `v${version}`,
  refreshTitle: "Refresh live identity and usage (⌘R)",
  refreshing: "Refreshing…",
  refresh: "Refresh status",
  addAccount: "Add account",
  footerPrivacy: "Runs on your Mac · no telemetry",
  footerMotto: "Accounts follow projects. Sessions stay yours.",

  // Shared controls
  cancel: "Cancel",
  done: "Done",
  working: "Working…",
  retry: "Retry",
  needsAttention: "Something needs attention",
  dismissNotice: "Dismiss notification",
  notKnown: "Not known",
  openClaude: "Open Claude",
  opening: "Opening…",

  // Accounts
  standardAccount: "Standard account",
  identityUnknown: "Identity not yet known",
  defaultBadge: "★ Default",
  planUnknown: "Plan unknown",
  identityDrift: "Identity drift",
  corruptPin: "Corrupt pin",
  identitySources: {
    live: "● Verified live",
    local: "○ Local identity",
    offline: "○ Offline identity",
    no_credentials: "○ No credentials",
    unknown: "○ Unverified",
  } satisfies Record<IdentitySource, string>,
  lockStates: {
    matched: "pin matches",
    drift: "pin differs from the signed-in identity",
    unpinned: "not pinned",
    unknown: "pinned, signed-in identity unknown",
    corrupt: "pin unreadable",
  } satisfies Record<LockStatus, string>,
  usageSources: {
    live: "Live from the API",
    cached: "Claude Code's own last reading",
    no_credentials: "No credentials",
    unavailable: "No reading",
  } satisfies Record<UsageSource, string>,
  identityTooltip: (identity: string, lock: string) =>
    `Identity: ${identity}; pin: ${lock}`,
  fiveHourWindow: "5-hour window",
  sevenDayWindow: "7-day window",
  actionsFor: (name: string) => `Actions for ${name}`,
  accountActions: "Account actions",
  detailsAction: "Details & diagnostics",
  loginAction: "Authenticate / re-login",
  makeDefaultAction: "Make default…",
  pinAction: "Pin identity…",
  repinAction: "Re-pin to signed-in identity…",
  cloneSettingsAction: "Copy settings from the standard account…",
  trashAction: "Move to Trash…",
  loginTitle: (name: string) => `Authenticate ${name}?`,
  loginConfirm: "Open login",
  loginBody:
    "Open the existing Claude login flow in Terminal. Check the browser identity before signing in. Return here and refresh after the login finishes.",
  pinTitle: (name: string) => `Pin ${name} to its identity?`,
  pinConfirm: "Pin",
  pinBody: (name: ReactNode) => (
    <>
      Pin {name} to the identity it is signed in as now. If it is later signed
      in as someone else, launching it is blocked until it is re-pinned.
    </>
  ),
  repinTitle: (name: string) => `Re-pin ${name}?`,
  repinConfirm: "Re-pin",
  repinBody: (name: ReactNode) => (
    <>
      {name} is signed in as someone other than its pin, or its pin cannot be
      read. Re-pinning accepts the identity signed in now and unblocks
      launching. Do it only if signing in as that identity was intended.
    </>
  ),
  cloneSettingsTitle: (name: string) => `Copy settings into ${name}?`,
  cloneSettingsConfirm: "Copy settings",
  cloneSettingsBody: (name: ReactNode) => (
    <>
      Copy settings.json, CLAUDE.md, agents, commands, output styles, skills and
      plugins from the standard account into {name}. A file or folder {name}
      already has is kept, and plugins are copied only if it has none
      installed. Sign-in, sessions and local settings are not copied.
    </>
  ),
  cloneSettingsDone: (name: string, items: string) => `Copied ${items} into ${name}.`,
  cloneSettingsNothing: (name: string) =>
    `Nothing to copy: ${name} already has these, or the standard account has none of them.`,
  trashTitle: (name: string) => `Move ${name} to the Trash?`,
  trashConfirm: "Move to Trash",
  trashBody: (name: ReactNode) => (
    <>
      The account directory of {name}, with its settings and sessions, moves to
      the Trash and stays there until you empty it. Directories bound to it lose
      their binding, and if it is the default, the standard account becomes the
      default.
    </>
  ),
  defaultTitle: (name: string) => `Make ${name} the default?`,
  defaultConfirm: "Change default",
  defaultBody: (name: ReactNode) => (
    <>
      New terminals in directories without a binding will use {name}. Existing
      Claude processes keep their account.
    </>
  ),
  details: {
    identity: "Identity",
    verification: "Verification",
    subscription: "Subscription",
    organization: "Organization",
    configuration: "Configuration",
    usageSource: "Usage source",
    bundledCli: "Bundled CLI",
    installedCli: "Installed CLI",
    verificationValue: (identity: string, lock: string) => `${identity} · ${lock}`,
    directoryAvailable: "Directory available",
    directoryUnavailable: "Directory missing or unsafe",
    bundledValue: (version: string, available: boolean) =>
      `${version} · ${available ? "ready" : "missing"}`,
    checking: "Checking…",
    notInstalled: "Not installed at ~/.claude-switch/bin",
    note: "Local identity is a disk record. Refresh checks the profile API. Pin status compares the recorded identity with the CLI identity lock; it does not prove a live login.",
  },
  availableAccounts: "Available accounts",
  configuredDefault: "Configured default",
  lastChecked: "Last checked",
  allAccounts: "All accounts",
  searchAria: "Search accounts",
  searchPlaceholder: "Search accounts or email…",
  readingAccounts: "Reading account configuration…",
  accountsUnavailableTitle: "Account data unavailable",
  accountsUnavailableBody: "Retry to reach the manager's backend.",
  noAccountsTitle: "No configured accounts yet",
  noAccountsBody: "Add an account to open the existing Claude login flow in Terminal.",
  noMatchTitle: "No matching accounts",
  noMatchBody: "Try another name or email address.",
  accountsFootnote:
    "Usage is read from Claude’s API only when you refresh. Every reading shows its age, and becomes stale after ten minutes. No account rotation or background session sharing.",
  accountName: "Account name",
  accountNamePlaceholder: "e.g. work",
  continue: "Continue",
  accountNameInvalid:
    "Use letters, numbers, hyphens, or underscores. “default” is reserved.",
  addTitle: (name: string) => `Add ${name}?`,
  addConfirm: "Create & authenticate",
  addBody:
    "Create an isolated account using the CLI and open its login flow in Terminal. Conversations are not copied.",
  addSeedLabel:
    "Start with the standard account's settings.json, CLAUDE.md, agents, commands, output styles, skills and plugins",

  // Usage and freshness
  resetPassed: "Reset passed",
  meterUnavailable: "Unavailable",
  resetsAt: (when: string, passed: boolean) =>
    `${passed ? "Reset" : "Resets"} ${when}`,
  resetUnknown: "Reset time unavailable",
  ages: {
    justNow: "just now",
    minutes: (n: number) => `${n}m ago`,
    hours: (n: number) => `${n}h ago`,
    days: (n: number) => `${n}d ago`,
  },
  usage: {
    live: (age: string) => `Live · checked ${age}`,
    cached: (age: string) => `Cached by Claude Code · ${age}`,
    stale: (age: string) => `Stale · ${age} · refresh to update`,
    noCredentials: "No credentials available",
    unavailable: "No usage reading · refresh to check",
  },

  // Projects
  choosePrompt: "Choose a project",
  chooseLead: "Inspect the account resolved by the CLI’s directory rules.",
  directoryLabel: "Project directory",
  directoryPlaceholder: "/Users/you/Projects/my-project",
  browse: "Browse…",
  inspect: "Inspect",
  effectiveAccount: "EFFECTIVE ACCOUNT",
  inheritedFrom: (directory: string) => `From binding: ${directory}`,
  fromDefault: "From configured default (no parent-directory binding)",
  bindingAccountAria: "Account for selected project",
  saveBinding: "Save binding",
  directoryBindings: "Directory bindings",
  reloadBindings: "Reload bindings",
  noBindingsTitle: "Give your projects an account",
  noBindingsBody: "Choose a folder above to create your first binding.",
  bindingIncludesChildren: "Includes child directories unless overridden",
  bindingDirectoryMissing: "Directory missing — binding is still stored",
  remove: "Remove",
  projectsFootnote:
    "The closest parent binding wins. A binding to “default” explicitly selects the standard ~/.claude account.",
  saveTitle: "Save project binding?",
  saveBody: (account: ReactNode, directory: ReactNode) => (
    <>
      Use {account} for {directory} and its child directories, except where a
      closer binding exists.
    </>
  ),
  removeTitle: "Remove directory binding?",
  removeConfirm: "Remove binding",
  removeBody: (directory: ReactNode) => (
    <>
      Remove the binding for {directory}. Future resolution will fall back to a
      parent binding or the configured default.
    </>
  ),

  // Sessions
  sessionsBanner:
    "This browser reads session metadata only, never a transcript's content. Copying a session to another account leaves the original where it is.",
  discoveringSessions: "Discovering session metadata…",
  noSessionsTitle: "No sessions found",
  noSessionsBody: "Start Claude Code under an account to create a conversation.",
  showMore: (remaining: number) => `Show more (${remaining} remaining)`,
  sessionsFootnote:
    "Project names are the CLI’s stored directory slugs, not reversible filesystem paths. Resuming a copied session happens in Terminal.",
  modifiedUnknown: "Modified time unknown",
  sessionSize: (kilobytes: string) => `${kilobytes} KB`,
  copiesElsewhere: "Copies in multiple accounts",
  copySessionAction: "Copy to…",
  copySessionAria: (id: string, account: string) =>
    `Copy session ${id} from ${account} to another account`,
  copySessionTitle: "Copy this session to another account?",
  copySessionBody: (account: ReactNode, id: ReactNode) => (
    <>
      The transcript of {id}, with its subagent transcripts, is copied from{" "}
      {account}. The original stays where it is.
    </>
  ),
  copySessionDestination: "Destination account",
  copySessionReplaceWarning: (account: ReactNode) => (
    <>{account} already has a copy of this session. Copying replaces it.</>
  ),
  copySessionCost:
    "The prompt cache is per account, so the first message after resuming under another account re-sends the whole transcript: slower and more expensive than a normal turn.",
  copySessionConfirm: "Copy session",
  copySessionReplaceConfirm: "Replace copy",
  copySessionDone: (account: string, id: string) =>
    `Copied to ${account}. To continue it, run claude-acc run ${account} --resume ${id} in Terminal.`,

  // Backend results
  errors: {
    session_missing:
      "This session is no longer in that account, so nothing was copied. The list has been reloaded.",
    session_exists:
      "The destination already has a copy of this session, so nothing was copied. The list has been reloaded; confirm again to replace it.",
    not_signed_in:
      "This account has never signed in, so there is no identity to pin. Sign in first, then pin it.",
    trash_unavailable:
      "The account could not be moved to the Trash, so nothing was removed. `claude-acc remove --purge` in Terminal deletes it outright.",
    identity_drift:
      "The account identity differs from its pin, or the pin is corrupt. If signing in as this identity was intended, re-pin it from the account menu; otherwise sign in again as the pinned one.",
    shared_credential_store:
      "A shared credential-store override is set. Live refresh is disabled because it could report one account for every directory. Launch the manager without CLAUDE_SECURESTORAGE_CONFIG_DIR.",
    invalid_account:
      "Use letters, numbers, hyphens, or underscores for an account name.",
    account_missing: "This account directory is missing. Refresh the account list.",
    account_exists: "An account with that name already exists.",
    invalid_directory:
      "Choose an absolute directory path without control characters or surrounding whitespace.",
    unrepresentable_directory:
      "The CLI binding format cannot represent this path. Avoid equals signs and parent-directory segments.",
    directory_missing: "This directory no longer exists. Choose another folder.",
    symlink_storage:
      "Symlinked account storage is not supported for safety. Use the CLI to inspect this configuration.",
    storage_unreadable:
      "The account configuration could not be read. Check file permissions.",
    storage_write_failed:
      "The change could not be saved, or was saved only in part. Check file permissions and free disk space, then try again.",
    cli_missing: "The bundled CLI is missing. Reinstall Claude Accounts.",
    terminal_failed: "Terminal could not be opened.",
    launcher_failed: "The terminal launch request could not be prepared.",
    terminal_macos_only: "Terminal launch is currently supported on macOS only.",
    backend_failed: "The backend operation failed. Try again.",
  } satisfies Record<ErrorCode, string>,
  unreachable:
    "Unable to reach the manager's backend. Open the installed app, then try again.",
  unrecognised:
    "The backend answered with an error this window does not recognise. Try again; if it repeats, reinstall Claude Accounts.",
  launchStarted: (account: string) =>
    `Started in Terminal under ${account}. Claude’s output, including any sign-in prompt, is in that window.`,
  launchPending:
    "Terminal has the request but has not run it yet. If macOS asked for permission, allow it and the launch continues.",
};

export type Messages = typeof en;

/**
 * The languages with a complete table. `ru` joins when its table exists
 * (#166); adding it here makes `tsc` require every key above.
 */
export const LANGS = ["en"] as const;
export type Lang = (typeof LANGS)[number];

export const messages: Record<Lang, Messages> = { en };

// No default value: a component rendered outside the provider is a bug to
// surface, not a reason to quietly pick a language.
const Copy = createContext<Messages | null>(null);

export function CopyProvider({ lang, children }: { lang: Lang; children: ReactNode }) {
  const t = messages[lang];
  // `index.html` cannot know the language; the document's `lang` decides how
  // a screen reader pronounces the page, so it follows the table, as does the
  // window title.
  useLayoutEffect(() => {
    document.documentElement.lang = t.locale;
    document.title = t.productName;
  }, [t]);
  return <Copy.Provider value={t}>{children}</Copy.Provider>;
}

/** The active language's table. */
export function useCopy(): Messages {
  const t = useContext(Copy);
  if (!t) throw new Error("useCopy outside CopyProvider");
  return t;
}

/**
 * Copy for a failed request. A backend code gets its own message. Any other
 * string means the backend did answer, with something this window does not
 * recognise, so it must not be told the backend is unreachable. Only a
 * non-string, a request that never got an answer, is that. Tauri's own
 * rejections are strings of the second kind, read from tauri 2.12.4: its
 * errors reach the window through `From<crate::Error> for InvokeError`
 * (`ipc/mod.rs`) as `Value::String(error.to_string())`, bad arguments
 * included, and an unknown command is rejected with
 * `format!("Command {command} not found")` (`webview/mod.rs`).
 */
export function describeError(t: Messages, error: unknown): string {
  if (typeof error !== "string") return t.unreachable;
  // `hasOwn`, not `in`: `in` also admits "toString", "constructor" and the
  // rest of the prototype chain, which are functions, not copy.
  return Object.hasOwn(t.errors, error) ? t.errors[error as ErrorCode] : t.unrecognised;
}

/** A size in kilobytes, with the table's decimal separator. */
export function kilobytes(t: Messages, bytes: number): string {
  return t.sessionSize(
    (bytes / 1024).toLocaleString(t.locale, {
      minimumFractionDigits: 1,
      maximumFractionDigits: 1,
    }),
  );
}

/** Items joined the table's way: "a, b, and c" in English. */
export function list(t: Messages, items: string[]): string {
  return new Intl.ListFormat(t.locale, { type: "conjunction" }).format(items);
}

export function describeLaunch(t: Messages, account: string, launch: Launch): string {
  return launch.state === "started" ? t.launchStarted(account) : t.launchPending;
}
