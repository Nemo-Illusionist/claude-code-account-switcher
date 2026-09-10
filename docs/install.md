# Installation and platforms

[← README](../README.md) · [Русский](ru/install.md)

Everything past the three-command install in the [README](../README.md#install):
building from source, the extra steps Windows needs, the zsh script, shell
completions, and moving between the two distributions.

## From source

```bash
cargo install --path .
claude-acc install
```

## Windows

PowerShell on a fresh Windows install needs two extra steps before `claude-acc` works:

1. **Allow the profile to run.** The default execution policy blocks the PowerShell profile, so the shell-integration line we add to it never executes — and that line is what puts `~/.claude-switch/bin` on `PATH` for the session:
   ```powershell
   Set-ExecutionPolicy -Scope CurrentUser RemoteSigned
   ```
2. **Run `install` by full path the first time.** The bin directory isn't on `PATH` yet, so call the `.exe` you just downloaded directly:
   ```powershell
   & "$HOME\Downloads\claude-acc.exe" install
   ```
3. **Restart PowerShell.** The profile only runs at shell startup, so the new `PATH` (and `cd`-activation) take effect in newly-spawned shells. After that, plain `claude-acc add work` works from anywhere.

Affected by an older broken install (binary copied without `.exe`, or shell line written for bash)? Re-run `claude-acc install` — it auto-cleans the stale extension-less binary and rewrites the profile line for PowerShell.

**Logging in on Windows.** `claude-acc add <name>` and `claude-acc login <name>` both spawn `claude auth login` under the new `CLAUDE_CONFIG_DIR`. On Windows that subcommand falls back to plain-text mode (no TUI), and the OAuth localhost callback frequently races ahead — so the `Paste code here if prompted >` prompt is unreliable for entering the code by hand. Workaround: after `claude-acc add <name>` has created the account directory, drive the login through Claude Code's first-launch TUI instead:

```powershell
claude-acc run <name>
```

This invokes `claude` directly under the account's `CLAUDE_CONFIG_DIR`, which triggers Claude Code's standard welcome → `Select login method:` flow. The in-TUI login accepts your code reliably and writes credentials to `~/.claude-switch/accounts/<name>/`. Verify with `claude-acc doctor` — each account should show its own email and UUID.

## Shell script (zsh-only)

```bash
cp claude-switch.sh ~/.claude-switch.sh
echo 'source ~/.claude-switch.sh' >> ~/.zshrc
source ~/.zshrc
```

## Shell completions

`claude-acc install` also wires up Tab completion for zsh, bash and PowerShell. It covers every command and its arguments — account names (with `default` where the command accepts it), `session copy` names of live sessions and ids for the current directory, `desktop` profile names, `vscode install|uninstall|status`, `resume-hook on|off`, `import`'s path, and each command's flags:

```
$ claude-acc session copy <TAB>
notes-api-3f  363edaeb-e81c-4021-94f4-7fe7d91815f4  0266a566-0336-4055-8f05-c553d368528e

$ claude-acc session copy 0266a566-… --to <TAB>
default  personal  work
```

Session ids are scoped to the current directory on purpose: a full listing runs to hundreds of uuids across every project ever opened, which is not a menu anyone can pick from.

## Switching between Rust and shell

Both versions read and write the same files under `~/.claude-switch/`:

```
~/.claude-switch/
├── accounts/        ← per-account CLAUDE_CONFIG_DIR
├── desktop/         ← per-profile Claude Desktop user-data dirs
├── config           ← default account
└── links            ← directory ↔ account bindings
```

So you can move from one to the other without re-creating accounts or relinking directories. Steps:

**Shell → Rust:**
1. Install the Rust binary: download from [Releases](https://github.com/Nemo-Illusionist/claude-code-account-switcher/releases) and run `claude-acc install`. The Rust install command writes its own shell-init line.
2. Remove the `source ~/.claude-switch.sh` line from your `~/.zshrc` (the Rust init handles activation now).
3. Optionally `rm ~/.claude-switch.sh`.

**Rust → shell:**
1. `cp claude-switch.sh ~/.claude-switch.sh` and add `source ~/.claude-switch.sh` to `~/.zshrc`.
2. Remove the `eval "$(... claude-acc init zsh)"` line from `~/.zshrc`.
3. Optionally `rm ~/.claude-switch/bin/claude-acc ~/.claude-switch/bin/claude` (the wrapper). The shell version regenerates its own wrapper on `source`.

Account credentials, links, and the `default` setting carry over without any changes.
