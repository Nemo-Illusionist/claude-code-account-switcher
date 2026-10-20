# Optional Claude Accounts manager

[Русский](../ru/gui/README.md)

An optional Tauri 2 + React desktop frontend for the existing CLI, macOS only. The CLI remains independently buildable with no desktop dependencies. There is no released build: build it from source as described below. Proposed in [#158](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/158).

## Using the app

Open `Claude Accounts.app` from the local bundle, or drag it from the DMG into Applications. The local build is ad-hoc signed, **not Developer ID signed or notarized**. Claude Code itself must already be installed and discoverable in your Terminal login environment. No Rust, Node, checkout, or separate `claude-acc` install is needed at runtime.

1. **Accounts:** discover configured directories and the standard `~/.claude` account. Details & diagnostics shows identity source, identity pin, subscription when known, and bundled/installed CLI status.
2. **Add account:** enter a name and confirm. The matching bundled CLI opens the existing login flow in Terminal. Authenticate there, then return and refresh. Re-login is in the account action menu. The app does not collect credentials.
3. **Open Claude:** starts a new terminal session with the explicitly selected account. It does not change existing processes. A detected local pin drift or corrupt pin blocks launch until resolved with the CLI. The confirmation says *started* once Terminal has actually run the launch and handed over to the CLI. If macOS is still asking for permission, it says the request is waiting.
4. **Make default:** requires confirmation and affects only new, otherwise-unbound contexts. It does not switch running Claude sessions.
5. **Projects:** choose or enter a directory, inspect its effective account and nearest inherited binding, choose an account, and confirm Save binding. More-specific child bindings take precedence. Remove a binding to restore inheritance. Missing-directory bindings can still be removed.
6. **Usage:** Refresh status requests current identity and 5-hour/7-day usage. The initial view reads local records only. Every reading shows how long ago it was taken. That age keeps counting while the window is open, and after ten minutes the reading is marked stale. A window whose reset time has passed shows “Reset passed” without a percentage, whether the reading was live or cached. API failure and missing credentials remain explicit. Automatic refresh is opt-in: 5, 15, or 30 minutes.
7. **Sessions:** browse account, stored project slug, ID, size, modified time, and duplicate presence. Newest entries come first. Graphical copying/resuming is deferred; use the existing CLI, review its conflict prompt, and preserve the source:

   ```sh
   claude-acc session copy <session-id> --to <destination-account>
   cd /original/project
   claude-acc run <destination-account> --resume <session-id>
   ```

   Run these using an installed compatible CLI or the `claude-acc` executable inside the app's `Contents/MacOS` directory. The first request under the destination account may rebuild its prompt cache. No automatic rotation or background synchronization is implemented.

Closing the window keeps the menu bar running. Use its account entries to launch Claude from your home directory, open project bindings, refresh status, reopen the manager, or quit. Menu entries show the usage source, not a global active account. Keyboard shortcuts: ⌘1/2/3 navigation, ⌘N add, ⌘R refresh.

## Build and checks

Prerequisites: macOS, Xcode Command Line Tools, stable Rust with clippy/rustfmt, Node **22.19+**, npm. The desktop crate is separate from the root package; root Cargo commands do not compile Tauri.

```sh
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo build --release
cargo test --release
cd apps/desktop
npm ci
npm run check
npm test
npm run desktop:build
```

To try the app without touching your real accounts, `node apps/desktop/scripts/create-review-fixture.mjs` creates two fake accounts, bindings and a session under `artifacts/review-home`. Start the built app with that folder as its home: `HOME="$PWD/artifacts/review-home" "apps/desktop/src-tauri/target/release/bundle/macos/Claude Accounts.app/Contents/MacOS/claude-acc-desktop"`.

`prepare:cli` builds and copies the CLI with the host-target suffix required by Tauri. `desktop:build` runs it automatically. For development use `npm run desktop:dev`. Keep `~/.cargo/bin` on PATH. Bundles are under `apps/desktop/src-tauri/target/release/bundle/{macos,dmg}`.

Native checks (after `npm run prepare:cli` and `npm run build`):

```sh
cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --all --check
cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --release
```

The desktop CI validates macOS builds without publishing. Root CI continues its existing macOS/Linux/Windows checks. Windows and Linux desktop packaging and terminal launching are follow-up work; do not interpret portable UI source as tested packages.

## Architecture and contract

The root `claude_acc` library holds the configuration, identity, usage, resolver, session, environment and quoting modules, so the CLI binary and the desktop crate share one implementation. The CLI's commands are unchanged. Only `gui` is meant for the desktop app; the other modules are `#[doc(hidden)]` and carry no stability promise. `gui` defines typed, serde-serialized results and machine error codes. Account policy lives there too: name and storage checks, binding resolution, and whether a launch may run. It never serializes credential structures, token hashes, raw config files or transcripts. See [contract](contract.md).

The native backend owns the account snapshot. It builds one at a time, with one thread per account during a live refresh, numbers each one, and sends it to both the menu bar and the window, so either works without the other. A refresh that queued behind another one reuses that result instead of repeating the network calls. Reads, writes and launches never wait for a refresh, and every command runs off the main thread. Only named Tauri commands are exposed. JavaScript has no filesystem, HTTP, shell, or remote-origin permissions. A CSP restricts content to bundled assets and local IPC. Only theme and refresh interval go in localStorage; account data stays in memory.

Terminal launch uses `open -a Terminal` with a typed file argument. Terminal runs it in a new login shell that inherits nothing from the app. A fixed POSIX launcher reads one private request file (0600, in a 0700 directory) as data and passes quoted arguments to the matching bundled CLI. Paths and account names are never interpolated into shell or AppleScript source. The launcher exports the app's `HOME`, so the CLI resolves the same accounts the app showed, and it deletes its request as soon as it has read it. The app waits for that deletion to report the launch as started. The CLI retains authentication-environment stripping, login Keychain snapshot/restore, identity locks, and account isolation. The launcher clears the shared credential-store override; live GUI refresh refuses that override because it could conflate identities.

## Boundaries and limitations

- The GUI is English in this MVP; the CLI's existing English/Russian interface is unchanged.
- Usage and identity API/Keychain behavior is reused from the CLI, with its existing undocumented-provider limitations. There is no new OAuth implementation. No online request success or real login is implied by a local identity record.
- Symlinked account storage and credential/identity files are refused. Project directory symlinks resolve to physical directories, matching the CLI's physical cwd. Paths containing `=` cannot be saved as bindings because the shared CLI format cannot represent them. Newline/control-character paths are refused. Unicode, spaces, and apostrophes are supported.
- Configuration writes reuse the CLI format and setters. Each read-modify-write holds an exclusive lock on `~/.claude-switch/.config.lock` and replaces the file by rename, so the app and a concurrent `claude-acc` cannot drop each other's change or leave a truncated file. `claude-switch.sh` takes the same lock, and falls back to its old unlocked edit if it cannot get it within five seconds.
- Existing external CLI versions are reported in diagnostics at the standard installed location. The GUI always launches its bundled matching CLI; an external mismatch does not break the app or silently update it.
- A *started* launch means the launcher ran and handed over to the bundled CLI, not that Claude authenticated. The terminal displays command failures. Account creation/login completion is observed on a subsequent refresh.
- Session metadata uses stored project slugs rather than guessed filesystem paths. Session contents are not read by the browser. Graphical handoff and resume are follow-up work.
- No account deletion, session deletion, telemetry, cloud account storage, auto-launch at login, auto-update, public publishing, or notarization is included.

## Status

Proposed in [#158](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/158), where naming, signing and whether this belongs in this repository are open questions. Graphical session handoff, Windows and Linux builds, Developer ID signing and notarization are follow-up work.
