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

`remove_account(account)` moves a managed account's directory to the desktop trash, then clears the default if it named the account and drops its bindings, and returns the updated snapshot. The standard account is refused. If the move fails the reply is `trash_unavailable` and nothing has changed; unlike `claude-acc remove`, it never falls back to deleting.

`clone_settings(account)` seeds a managed account from the standard one through the CLI's own `seed`, as `claude-acc clone-settings` does, and returns the names it copied, a directory with a trailing `/`; an empty list means there was nothing to copy. A file or directory the account already has is kept; plugins are copied only into an account with none installed. The standard account is refused with `invalid_account`. A symlink in the account is refused with `symlink_storage` only where the seed would write through it.

`import_account(account, source, move_into)` adopts an existing config dir as a managed account, by the refusal rules of `claude-acc import` (shared as `import::refusal`), and returns the updated snapshot. `source` is absolute, or starts with `~/` for the home the app resolved accounts under. It is copied, or moved with `move_into`, and on macOS a Keychain entry found for it is copied to the new location; unlike the CLI there is no network check afterwards, so a refresh shows whether the sign-in came along. The CLI's refusal of a source already under `accounts/` is `import_source_refused`, and beyond the CLI's rules so is a source that is itself a link or holds its credential or identity files as links, one that holds the `accounts/` directory (compared as physical paths), or a move of anything that holds the standard `~/.claude`.

`install_statusline(account, replace)` writes the `statusLine` block of `claude-acc statusline --install` (shared as `statusline::entry`) into a named account's `settings.json`, keeping the rest of it. Unlike the CLI, a file that is not a JSON object is refused with `storage_unreadable` rather than replaced; a different `statusLine` is replaced only with `replace`, otherwise the reply is `statusline_exists`; without the CLI installed under the switch directory the reply is `cli_not_installed`; a `settings.json` that is a link is refused with `symlink_storage`; and the file is replaced by a rename.

`pin_account(account, force)` pins an account to the identity it is signed in as, by the rules of `claude-acc lock`, and returns the updated snapshot. Pinning the identity already pinned does nothing. A pin to someone else, or one that cannot be read, is replaced only with `force`; without it the reply is `identity_drift`. An account that has never signed in gets `not_signed_in`. Every snapshot the window receives re-reads each account's pin state, drops accounts no longer on disk and adds a row from local records for one that appeared, so a refresh that started earlier cannot show the old state.

## Bindings, sessions and launching

- `list_bindings(directory?)` returns sorted `bindings` (`directory`, `account`, `exists`) and an optional `selected`: the physical directory, its effective `account` and nullable `inherited_from`, resolved by the CLI's own `resolve_account` and `find_linked_dir`.
- `set_binding(directory, account?)` saves a binding, or with a null account removes the exact stored one, even for a missing directory. "default" means the standard account, not the configured named default.
- `choose_directory()` opens the system folder picker.
- `list_sessions()` returns metadata (`id`, `account`, `project`, `modified`, `size`, `duplicate`), newest first. No transcript paths or content.
- `resume_hook()` returns whether the `claude` wrapper's `--resume` check is on as stored, the value `claude-acc resume-hook` shows; `CLAUDE_ACC_NO_RESUME_HOOK` belongs to a terminal's environment and is not consulted. `set_resume_hook(on)` stores it, as `resume-hook on|off` does, and returns the value read back.
- `copy_session(from, project, id, to, replace)` copies the session `list_sessions` listed under `from` and `project` into `to`, through the CLI's own `sessions::copy_into`, and returns the session list again. The source is looked up in a listing, never joined from the arguments; one that is no longer there gets `session_missing`. If `to` already holds that id, in any project, the reply is `session_exists` unless `replace` is set. Copying an account onto itself is `invalid_account`.
- `vscode_editors()` returns each installed VS Code-family editor as `name`, `state` (`ours`, `unset`, `foreign`, `unreadable`, `linked`) and `other_profiles`; a foreign launcher's path is not sent. `vscode_install(force)` and `vscode_uninstall()` apply the CLI's own `plan_install` and `plan_uninstall` to each editor and return the list again. Unlike the CLI, install needs the CLI installed under the switch directory (`cli_not_installed`), since its fallback, the running binary, would be this app; a linked `settings.json` is left alone; and on Windows the reply is `unsupported_platform`, where the CLI refuses too.
- `open_terminal(account, directory?, action)` takes `run`, `login`, `add` or `add_seeded`, which runs `add --seed`. `gui::check_launch` decides whether the action may run: a valid name, safe existing storage (a free name for either add), and for `run` no drifted or unreadable pin. The reply is `{state}`: `started` once Terminal ran the launcher, or `pending` if it had not within ten seconds. Neither means Claude Code signed in.
- `installation()` returns `bundled_version`, `bundled_available` and the nullable version of a separately installed CLI. Both are read by running the binaries, when Details first opens, again after a live refresh, and again for each `check_update`.
- `check_update()` asks GitHub for the latest release through the CLI's own `update::latest_release_tag` and returns `latest`, nullable `bundled` and `installed` (the versions `installation()` reports, `null` when unknown or absent) and `bundled_outdated` and `installed_outdated`, true only for a known version older than `latest`. Nothing is downloaded. A failed lookup is `update_check_failed`.

## Desktop profiles

- `desktop_profiles()` returns `profiles` (`name`, `signed_in`, nullable `email`) and `app_profile_exists`, from local records only: no Keychain and no network. The email comes from the profile's own cache, or from an account the window shows that is signed in as the same uuid. A profile whose storage would be refused is left out.
- `desktop_open(account, action, force)` takes `open`, `add` or `add_seeded` and returns the list again. It looks for the app first (`desktop_app_missing`), then `gui::desktop_prepare` decides: a valid name other than `default`, an existing profile to open (`account_missing`) or a free name to add (`account_exists`), and, for a sign-in, which an add always is, no other Claude Desktop running (`desktop_running`) unless `force`. An add creates the profile, and `add_seeded` copies the app's own `claude_desktop_config.json` into it; a failed copy does not withhold the profile, as in the CLI. The app is then opened the way `claude-acc desktop run` opens it (`desktop_launch_failed`).
- `desktop_remove(account)` moves a profile to the desktop trash and returns the list again. Unlike `claude-acc desktop remove` it never deletes outright (`trash_unavailable`), and a profile a running instance uses is refused (`desktop_running`).

## Errors

Every failure is a machine code the window words: the library's (`invalid_account`, `account_missing`, `account_exists`, `symlink_storage`, `storage_unreadable`, `storage_write_failed`, `invalid_directory`, `unrepresentable_directory`, `directory_missing`, `shared_credential_store`, `identity_drift`, `trash_unavailable`, `not_signed_in`, `session_missing`, `session_exists`, `import_source_refused`, `cli_not_installed`, `statusline_exists`, `unsupported_platform`, `update_check_failed`, `desktop_running`, `desktop_app_missing`, `desktop_launch_failed`) and the shell's (`cli_missing`, `launcher_failed`, `terminal_failed`, `terminal_macos_only`, `backend_failed`). A code never carries a path, subprocess output or credential data. Every code has copy in every language, and nothing has copy that cannot be sent; `copy.test.ts` checks both directions.

## Never sent

No token, token hash, credential file, Keychain entry, raw config file, or transcript path or content. The types have no field that could hold one, and a test plants a credentials file, extra secret fields in `.claude.json` and a profile cache's token hash, and checks that none of them reaches the serialized snapshot.
