# Desktop contract

[Русский](../ru/gui/contract.md)

The JSON between the Rust backend and the React frontend is defined once per side and pinned by one sample. Rust: the `Serialize` types in `src/gui.rs` and `apps/desktop/src-tauri/src/{backend,launcher}.rs`. TypeScript: `apps/desktop/src/contract.ts`. Both are tested against `apps/desktop/src/contract.sample.json`, so a field renamed or retyped on one side fails a test on that side. There is no schema version: frontend and backend ship in one bundle.

Errors are machine codes (`GuiError` in the library, `AppError` in the desktop crate) that the frontend maps to copy. A code carries no path, subprocess output or credential data. Every code has copy, and no copy exists for a code that cannot be sent; `contract.test.ts` checks both directions.

## Accounts

`snapshot(live)` returns `revision`, `observed_at` (Unix seconds), `default_account` and `accounts`, the standard account first. The backend builds snapshots one at a time and numbers them; replies and `snapshot` events (sent after every build, including tray refreshes) are applied only when their `revision` is not lower than the one shown.

Each account has `name`, nullable `email`, `plan`, `organization`, booleans `is_default` and `available`, `identity_source` (`local`, `live`, `unknown`, `offline`, `no_credentials`), `lock` (`matched`, `drift`, `unpinned`, `unknown`, `corrupt`) and `usage`. A local pin match does not prove live authentication.

`usage.source` is `live`, `cached`, `no_credentials` or `unavailable`. `fetched_at` is when the reading was taken, in Unix seconds, or null without one. The frontend ages it against its own clock: under ten minutes a reading is live or cached by source, after that it is stale. Each of `five_hour` and `seven_day` is null or `{percent, resets_at}`, with `resets_at` in nullable Unix seconds. Once `resets_at` has passed, the percentage is not shown, whatever the source.

`set_default(account)` returns the updated snapshot. It keeps the readings an earlier refresh paid for; only the default moves.

## Bindings and mutations

`list_bindings(directory?)` returns sorted `bindings` (`directory`, `account`, `exists`) and an optional `selected`: the physical directory, its effective `account` and nullable `inherited_from`, resolved by the CLI's own `resolve_account` and `find_linked_dir`. `set_binding(directory, account?)` saves a binding, or removes the exact stored one when `account` is null, including for a missing directory. "default" means the standard account, not the configured named default. `choose_directory()` opens the native folder picker.

Writes to `~/.claude-switch/config` and `links` hold an exclusive lock on `~/.claude-switch/.config.lock` for the whole read-modify-write, and replace the file by rename. The CLI, the app and `claude-switch.sh` all take the same lock. The script uses zsh's `zsystem flock`, which excludes Rust's `File::lock` on macOS. It falls back to its old unlocked edit if the lock cannot be taken within five seconds.

## Launching and diagnostics

`open_terminal(account, directory?, action)` takes `action` as `run`, `login` or `add`. `gui::check_launch` decides whether the action may run: valid name, existing safe storage (or, for `add`, a free name), and no pin drift or corrupt pin for `run`. It returns the physical directory, defaulting to the home directory. The reply is `{state}`: `started` once the launcher has run in Terminal and handed over to the bundled CLI, or `pending` if Terminal accepted the request but had not run it within ten seconds. Neither means Claude Code authenticated. Its output is in the Terminal window. Launching is macOS-only.

`list_sessions()` returns metadata (`id`, `account`, `project`, `modified`, `size`, `duplicate`), newest first. No transcript paths or content are exposed.

`installation()` returns `bundled_version`, `bundled_available` and the nullable version of a separately installed CLI, read once per run. The app never uses an installed CLI for its own operations.

## Testing policy

Use temporary roots or a separate process with an isolated HOME. Do not exercise mutations or live authentication against the owner's accounts. Native screenshots should use clearly labeled fixture records. Existing CLI Keychain tests use dedicated `claude-acc-test-*` services, not real credential entries. The one test that opens Terminal for real runs a stand-in CLI and is `#[ignore]`d. Run it by hand with `cargo test --release -- --ignored real_terminal` in `apps/desktop/src-tauri`.
