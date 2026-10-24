# The manager's contract

[Русский](../ru/manager/contract.md)

What the Rust side sends the window, and how both sides are kept in agreement. In this repository `claude_acc::gui` is ours to change whenever the CLI needs it to; what keeps the sides honest is not stability but a test failing on each side when a field is renamed.

## Where it is defined

- **The library:** `claude_acc::gui` in `src/gui.rs`. Its wire shape is pinned by `src/gui.contract.json`, which a Rust test compares with what the types serialize. The module documentation is the written description.
- **The shell:** `apps/manager/src-tauri/src/{backend,launcher}.rs` adds the snapshot's `revision`, the launch result, installation diagnostics and its own error codes.
- **The window:** `apps/manager/src/contract.ts` mirrors all of it in TypeScript. `apps/manager/src/contract.sample.json` is checked by the shell's tests, and by the frontend's against a typed copy and field for field against `src/gui.contract.json`.

## Accounts

`snapshot(live)` returns `revision`, `observed_at` (Unix seconds), `default_account` and `accounts`, the standard account first. The shell builds snapshots one at a time and numbers them. The window applies a reply, or a `snapshot` event sent after every build including menu bar refreshes, only if its `revision` is not lower than the one shown.

Each account has `name`, nullable `email`, `plan` and `organization`, `is_default`, `available`, `identity_source` (`local`, `live`, `unknown`, `offline`, `no_credentials`), `lock` (`matched`, `drift`, `unpinned`, `unknown`, `corrupt`) and `usage`. A matching pin does not prove a live sign-in.

`usage.source` is `live`, `cached`, `no_credentials` or `unavailable`. `fetched_at` is when the reading was taken, in Unix seconds, or null. `five_hour` and `seven_day` are null or `{percent, resets_at}`, with `resets_at` in nullable Unix seconds. The window ages a reading against its own clock: live or cached for ten minutes, stale after that. A window whose `resets_at` has passed shows no figure, whatever the source.

`set_default(account)` returns the updated snapshot, keeping readings an earlier refresh paid for.

## Bindings, sessions and launching

- `list_bindings(directory?)` returns sorted `bindings` (`directory`, `account`, `exists`) and an optional `selected`: the physical directory, its effective `account` and nullable `inherited_from`, resolved by the CLI's own `resolve_account` and `find_linked_dir`.
- `set_binding(directory, account?)` saves a binding, or with a null account removes the exact stored one, even for a missing directory. "default" means the standard account, not the configured named default.
- `choose_directory()` opens the system folder picker.
- `list_sessions()` returns metadata (`id`, `account`, `project`, `modified`, `size`, `duplicate`), newest first. No transcript paths or content.
- `open_terminal(account, directory?, action)` takes `run`, `login` or `add`. `gui::check_launch` decides whether the action may run: a valid name, safe existing storage (a free name for `add`), and for `run` no drifted or unreadable pin. The reply is `{state}`: `started` once Terminal ran the launcher, or `pending` if it had not within ten seconds. Neither means Claude Code signed in.
- `installation()` returns `bundled_version`, `bundled_available` and the nullable version of a separately installed CLI. Both are read by running the binaries, when Details first opens and again after a live refresh.

## Errors

Every failure is a machine code the window words: the library's (`invalid_account`, `account_missing`, `account_exists`, `symlink_storage`, `storage_unreadable`, `storage_write_failed`, `invalid_directory`, `unrepresentable_directory`, `directory_missing`, `shared_credential_store`, `identity_drift`) and the shell's (`cli_missing`, `launcher_failed`, `terminal_failed`, `terminal_macos_only`, `backend_failed`). A code never carries a path, subprocess output or credential data. Every code has copy in every language, and nothing has copy that cannot be sent; `copy.test.ts` checks both directions.

## Never sent

No token, token hash, credential file, Keychain entry, raw config file, or transcript path or content. The types have no field that could hold one, and a test plants a credentials file, extra secret fields in `.claude.json` and a profile cache's token hash, and checks that none of them reaches the serialized snapshot.
