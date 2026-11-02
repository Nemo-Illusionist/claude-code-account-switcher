# Claude Accounts: the manager

[Русский](../ru/manager/README.md)

An optional macOS app and menu bar item for this tool: accounts, their identity and usage, and per-directory bindings, in one window. It is built on the same Rust code as the CLI and lives in `apps/manager`, outside the root `cargo` build, so CLI-only users build and install nothing extra.

Not to be confused with `claude-acc desktop`, which runs Claude Desktop profiles ([guide](../desktop.md)).

There is no published build: the manager is built from source, unsigned ([#165](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/165)).

## Using it

Claude Code must already be installed and on `PATH` in your Terminal login environment. The app bundles its own matching `claude-acc`, so no separate install is needed.

1. **Accounts:** every configured account and the standard `~/.claude` one, with identity, plan, pin state and the 5-hour and 7-day windows. Details shows where each fact comes from and which CLI versions are present.
2. **Usage:** the window opens on local records. Refresh status asks the profile and usage APIs. Every reading shows how long ago it was taken, the age keeps counting while the window is open, and after ten minutes the reading is marked stale. A window past its reset time shows "Reset passed" instead of a figure. Automatic refresh is opt-in: every 5, 15 or 30 minutes.
3. **Open Claude:** a new Terminal window running the bundled `claude-acc run <account>`. Running sessions are not affected. A pin that differs from the signed-in identity, or cannot be read, blocks the launch until the account is re-pinned (below) or signed back in as the pinned identity. The confirmation says the launch *started* once Terminal has run it, which is not the same as Claude having signed in.
4. **Add account / re-login:** open the CLI's existing login flow in Terminal. The app never sees or stores credentials. Refresh afterwards.
5. **Make default:** affects new, otherwise unbound contexts only, and asks first.
6. **Pin identity:** an unpinned account can be pinned to the identity it is signed in as, as `claude-acc lock` does. When the pin and the signed-in identity differ, or the pin cannot be read, launching is blocked and the menu offers to re-pin to the identity signed in now; that is the only way the app replaces a pin.
7. **Move to Trash:** a managed account's directory goes to the Trash, and its bindings go with it; if it was the default, the standard account becomes the default. The standard account cannot be removed. If the Trash refuses the move, nothing changes; deleting outright stays `claude-acc remove --purge`.
8. **Projects:** pick or type a folder to see its effective account and the binding it inherits from, then save or remove a binding. The closest binding wins. A binding whose folder no longer exists can still be removed.
9. **Sessions:** session metadata across accounts, newest first. **Copy to…** copies one session, with its subagent transcripts, into another account, as `claude-acc session copy` does; the original stays. If the destination already has that session, the dialog says so and the button replaces it. Resuming stays in Terminal ([sessions guide](../sessions.md)).

Closing the window keeps the menu bar item running. From it you can launch an account in your home directory, open Projects, refresh, or quit, and all of that works with the window closed. Shortcuts: ⌘1/2/3 switch pages, ⌘N adds an account, ⌘R refreshes.

## Building and checking

You need macOS, Xcode Command Line Tools, Rust 1.90 or newer with clippy and rustfmt (the locked Tauri crates require 1.90), Node 22.19 or newer, and npm.

```sh
cd apps/manager
npm ci --ignore-scripts
npm run check
npm test
npm run manager:build
```

`--ignore-scripts` is deliberate: no package in the tree needs an install script (esbuild and fsevents ship prebuilt binaries), and skipping them keeps a dependency's install hook from running with your account's access.

`manager:build` runs `prepare:cli`, which builds the CLI and copies it in with the target suffix Tauri expects, and then bundles. The app lands in `apps/manager/src-tauri/target/release/bundle/macos/`. Use `npm run manager:dev` while developing.

The crate's own checks, after `npm run build` and `npm run prepare:cli`:

```sh
cargo fmt --manifest-path apps/manager/src-tauri/Cargo.toml --all --check
cargo clippy --manifest-path apps/manager/src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path apps/manager/src-tauri/Cargo.toml --release
```

To try the app without touching real accounts, `node apps/manager/scripts/create-review-fixture.mjs` creates two fake accounts, two bindings and a session under the git-ignored `artifacts/review-home`. Start the built app with that folder as its home:

```sh
HOME="$PWD/artifacts/review-home" \
  "apps/manager/src-tauri/target/release/bundle/macos/Claude Accounts.app/Contents/MacOS/claude-acc-manager"
```

The Manager workflow has two jobs. `checks` runs on every pull request, docs-only ones included, so it can be a required check: the frontend's typecheck and tests, then the crate's fmt, clippy and tests. `bundle` builds the unsigned `.app` once `checks` passes: on a pull request only if `apps/manager/`, `src/` or the root manifests changed, and always after a merge to `master` or when started by hand. Nothing is published.

## How it fits together

- **`claude_acc::gui`** (`src/gui.rs`) is the one part of the library the manager uses: typed, secret-free results and machine error codes, with binding resolution, writes and the launch rule all going through the CLI's own code. Its wire shape is pinned by `src/gui.contract.json`. See [the contract](contract.md).
- **The shell** (`apps/manager/src-tauri`) owns the account snapshot. It builds one at a time, numbers each one, and sends it to both the menu bar and the window, so neither depends on the other being open. A refresh that queued behind another reuses that result instead of repeating the network calls. Reads, writes and launches never wait for a refresh.
- **The frontend** (`apps/manager/src`) reaches the shell only through `api.ts`, the one module that imports `@tauri-apps/*`, and a test enforces that. JavaScript has no filesystem, shell or HTTP access, and a content security policy limits it to bundled assets and IPC.
- **Launching:** Terminal runs a `.command` file in a fresh login shell that inherits nothing from the app. The launcher is constant code that reads one private request file (0600, in a 0700 directory) as data. It exports the app's `HOME`, so the CLI resolves the same accounts the window showed. It drops `CLAUDE_SECURESTORAGE_CONFIG_DIR`. It deletes its request on start, which is how the app knows it ran. Paths, account names and translated text never become shell or AppleScript source.

## Language

The manager's prose is in one place per side: `apps/manager/src/i18n.tsx` for the window and `apps/manager/src-tauri/src/copy.rs` for the menu bar, dialogs and the Terminal prompt. On both sides a language missing an entry fails the build: `tsc` checks the window's table, and `copy.rs` matches exhaustively, as the CLI's `i18n.rs` does. Neither checks that a value is translated, and nothing falls back from one language to another ([#166](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/166)). Today only English is filled in. The Russian table is being written by the maintainer, and how the manager picks its language is still open.

## Limits

- macOS only for now. The launcher is the only platform-specific code, and other platforms get an explicit "macOS only" error from it.
- Unsigned. macOS asks you to allow the first launch once; see [#165](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/165).
- Usage and identity come from the same undocumented APIs and Keychain lookups the CLI uses, with the same caveats. A local identity record does not prove a live sign-in.
- A symlink planted in account storage (an account directory, a credential or identity file, `config` or `links`) is refused. The storage itself may live behind a symlink, such as a home on another volume. A project folder reached through a symlink resolves to its physical path, as the CLI's working directory does. Paths containing `=` cannot be bound, because the `links` format cannot represent them.
- Writes to `config` and `links` take the same lock as the CLI and `claude-switch.sh` (the script falls back to an unlocked write if it cannot get the lock within five seconds).
- No account or session deletion, rotation, telemetry, auto-update or launch at login.
