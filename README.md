# Claude Code Account Switcher

[Русская версия](README.ru.md)

Bind different Claude Code accounts to different directories.
On `cd`, the correct account is activated automatically.

Two distributions:

- **Rust CLI** (`claude-acc`) — cross-platform: macOS, Linux, Windows; zsh, bash, PowerShell. **Recommended.**
- **Shell script** (`claude-switch.sh`) — zsh-only, macOS-focused. Single file, no binary, no compilation.

Both share the same on-disk format (`~/.claude-switch/`) so you can switch between them freely.

## Install

### Rust CLI (recommended)

Download from [GitHub Releases](https://github.com/Nemo-Illusionist/claude-code-account-switcher/releases), then run:

```bash
claude-acc install
```

This will:
- Copy the binary to `~/.claude-switch/bin/claude-acc` (`.exe` on Windows)
- Install the IDE wrapper at `~/.claude-switch/bin/claude` (see [IDE integration](#ide-integration))
- Auto-detect your shell (zsh/bash/PowerShell)
- Add shell integration to your rc file

To update later, just run `claude-acc update` — it downloads the latest release binary for your platform and swaps it in. (Or download a new binary manually and run `claude-acc install` again.)

#### From source

```bash
cargo install --path .
claude-acc install
```

#### Windows

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

### Shell script (zsh-only)

```bash
cp claude-switch.sh ~/.claude-switch.sh
echo 'source ~/.claude-switch.sh' >> ~/.zshrc
source ~/.zshrc
```

## Quick start

```bash
# 1. Add accounts (opens Claude login)
claude-acc add work

# 2. Link work account to a directory
cd ~/work
claude-acc link work

# Done! cd into ~/work or any subdirectory uses the work account.
# Everything else uses the standard ~/.claude/ config.
```

## Commands

| Command | Description |
| --- | --- |
| `claude-acc` | Help |
| `claude-acc list` | List all accounts |
| `claude-acc add <name>` | Add account (runs `claude login`); add `-s` / `--seed` to seed from `~/.claude/` |
| `claude-acc clone-settings <name>` | Copy `settings.json` / `CLAUDE.md` / `agents/` / etc. from `~/.claude/` into an existing account |
| `claude-acc login <name>` | Re-login to an account |
| `claude-acc remove <name>` | Remove account |
| `claude-acc default [name]` | Show/set default account |
| `claude-acc reset` | Reset default to `~/.claude/` |
| `claude-acc link <name>` | Link account to current directory |
| `claude-acc unlink` | Unlink current directory |
| `claude-acc links` | Show all directory links |
| `claude-acc status` | Show active account |
| `claude-acc usage` | Show 5h / 7d rate-limit usage for every account |
| `claude-acc run <name>` | Run claude under a specific account |
| `claude-acc whoami` | Print the email (or name) of the active account |
| `claude-acc doctor [--json]` | Audit each account's actual OAuth identity |
| `claude-acc install` | Install binary and shell integration |
| `claude-acc update [--check]` | Update the binary to the latest GitHub release |

## How it works

```
~/.claude-switch/
├── accounts/
│   └── work/        ← Claude config for work account
├── config           ← default=work (or empty for ~/.claude/)
└── links            ← bindings: path=account
```

On directory change:

1. Looks up the current directory in `~/.claude-switch/links`
2. If not found — walks up the directory tree
3. If no binding — uses the default account (or `~/.claude/` if none set)
4. Sets `CLAUDE_CONFIG_DIR`

## Directory inheritance

Linking a directory applies to **all subdirectories** automatically.
You don't need to link each project separately:

```
~/work                  → work      (linked explicitly)
~/work/project-a        → work      (inherited)
~/work/project-b        → work      (inherited)
~/work/project-b/src    → work      (inherited)
~/personal              → ~/.claude/ (default)
```

A more specific link always wins. This lets you set exceptions:

```
~/work                  → work      (linked)
~/work/project-a        → work      (inherited)
~/work/secret           → personal  (linked — overrides parent)
~/work/secret/src       → personal  (inherited from secret)
```

Use `default` as a reserved name to explicitly fall back to `~/.claude/`:

```
~/work                  → work      (linked)
~/work/project-a        → work      (inherited)
~/work/hobby            → ~/.claude/ (linked to default — overrides parent)
~/work/hobby/sub        → ~/.claude/ (inherited from hobby)
```

```bash
cd ~/work/hobby
claude-acc link default
# hobby → ~/.claude/ (default)
```

## IDE integration

JetBrains IDEs (PhpStorm, IntelliJ etc.) and VSCode launch the `claude` binary directly without sourcing your shell config, so `CLAUDE_CONFIG_DIR` would not be set and the wrong account would be used. To make IDE ↔ Claude Code handshake work for non-default accounts, `claude-acc install` sets up two things:

- A wrapper at `~/.claude-switch/bin/claude` that picks the account for the current working directory (via `claude-acc activate`) and `exec`s the real `claude` binary. `~/.claude-switch/bin` is prepended to `PATH` (by the shell init), so both terminals and IDEs pick up the wrapper transparently.
- A symlink `~/.claude-switch/accounts/<name>/ide → ~/.claude/ide` for every account. Claude Code writes IDE lock files to `$CLAUDE_CONFIG_DIR/ide/`, but IDE plugins always look in `~/.claude/ide/`. The symlink makes both sides agree.

No manual setup required — `claude-acc install` does both. New accounts created via `claude-acc add` get their `ide/` symlink automatically.

## What gets switched

`CLAUDE_CONFIG_DIR` relocates the entire `~/.claude/` directory, including ([docs](https://code.claude.com/docs/en/settings)):

| File | Description |
|---|---|
| `settings.json` | User-level settings |
| `CLAUDE.md` | Global memory / instructions |
| `agents/` | Subagents |
| `.credentials.json` | Auth credentials |
| `projects/` | Per-project global configs |
| sessions, history, etc. | Runtime data |

Each account gets its own copy of all these files in `~/.claude-switch/accounts/<name>/`.

### Per-account default model

Because every account has its own `settings.json`, you get a per-account default model for free — no extra flag or config. Set Claude Code's [`model`](https://code.claude.com/docs/en/settings) key in that account's settings file:

```bash
# e.g. Opus on work, a lighter model on personal
echo '{ "model": "opus" }' > ~/.claude-switch/accounts/work/settings.json
```

Now any `claude` started under the `work` account boots with that model. (Tools that symlink a single shared `settings.json` across accounts can't do this without a separate mechanism — here it's just the isolated config dir doing its job.)

## Inheriting `~/.claude/` config

A fresh `claude-acc add work` produces an empty config dir — no `settings.json`, no `CLAUDE.md`, no custom agents. If you want the new account to start with the same setup as your standard `~/.claude/`, use the `-s` / `--seed` flag, or run `clone-settings` retroactively:

```bash
claude-acc add -s work               # seed at creation time
claude-acc clone-settings work       # seed an existing account
```

Both copy a curated set of files from `~/.claude/`:

**Copied** (configuration / personalization):
- `settings.json` (env vars, permissions, hooks references, statusline, plugins, language)
- `CLAUDE.md` (global memory)
- `agents/`, `commands/`, `output-styles/`, `skills/` (custom assets)

**Not copied** (per-account state — would defeat the isolation):
- `.credentials.json` (auth token — re-acquired via `claude auth login`)
- `settings.local.json` (per-machine local overrides)
- `projects/`, `todos/`, `statsig/` (sessions, runtime state, telemetry)
- `hooks/`, `plugins/` (settings.json references these by absolute path; copying duplicates files for nothing)
- `.account-info.json` (the doctor cache)

Existing files in the target are skipped — `clone-settings` is a one-shot seed, not a sync.

## Per-project settings

Each account gets its own `~/.claude-switch/accounts/<name>/` directory, which acts as `CLAUDE_CONFIG_DIR`. This means each account has its own `settings.json`, credentials, and project history.

You can use this to have different settings for different projects — even under the same login. Just create multiple accounts and log in with the same credentials:

```bash
# Shared work account with default settings
claude-acc add work
cd ~/work
claude-acc link work

# Same login, but with its own settings for a specific project
claude-acc add work-ml
cd ~/work/ml-project
claude-acc link work-ml

# Now edit settings independently:
# ~/.claude-switch/accounts/work/settings.json       — for all work projects
# ~/.claude-switch/accounts/work-ml/settings.json     — only for ml-project
```

> Note: `claude-acc add` runs `claude login`, so you'll need to log in again (same account, just a new config directory).

## Auditing identities (`doctor`)

`claude-acc add` and `claude-acc login` both run `claude auth login` under a per-account `CLAUDE_CONFIG_DIR`. Whatever Anthropic account you sign in with becomes the identity for that directory — and there's no built-in surface to see which account is actually behind a given config dir. If you accidentally log in with the wrong identity (browser auto-fill, a stale tab), the switch is silent: rate limits, conversation history, and billing leak across what you thought were isolated accounts.

`claude-acc doctor` reads each account's OAuth token from the macOS Keychain (with a `.credentials.json` fallback for non-Keychain installs), calls `https://api.anthropic.com/api/oauth/profile`, and prints the live email, plan, and UUID:

```
$ claude-acc doctor
Auditing 2 account(s):
  ✓ work      alice@anthropic.com  Max 20x  uuid=aa6c22d5-…
  ? personal  no token (run: claude-acc login personal)

1 of 2 accounts healthy.
```

The plan label (`Max 20x` / `Max` / `Pro`) is derived from the profile's tier flags and `rate_limit_tier`; it's omitted for accounts with no recognizable subscription.

It's purely a read-only audit — nothing is intercepted, no `claude` invocation is gated. Run it whenever you want to confirm a config dir is bound to the identity you expect. Requires `security`, `curl`, `jq`, and `shasum` (all preinstalled on macOS); the Rust binary uses native `serde_json` and `sha2` instead and only shells out to `security` and `curl`.

`doctor` also caches the result (email, plan, UUID) to `~/.claude-switch/accounts/<name>/.account-info.json` so `list`, `usage`, `status`, and `default` can show the identity next to each account without re-hitting the API:

```
$ claude-acc list
Claude Code accounts:
  ★ work       (default)  alice@anthropic.com   Max 20x  3d ago
    personal              bob@anthropic.com     Pro      1h ago *
    ~/.claude/            charlie@personal.com  Max 5x   3d ago    (standard)

$ claude-acc status
Active account: work <alice@anthropic.com> (linked to my-project)

$ claude-acc default
Default: work <alice@anthropic.com>
```

`doctor` audits the standard `~/.claude/` config too (the unmanaged identity used when no link / configured default applies). Its cache lives at `~/.claude-switch/default.account-info.json`. The `~/.claude/` row appears in `list` only after you've actually logged into Claude Code with the standard config (or after `doctor` has cached an identity for it).

For scripting, `claude-acc doctor --json` emits the same audit information as a single JSON document — and `claude-acc whoami` prints just the email (or account name fallback) of the active account, suitable for shell prompts:

```bash
# Use in a prompt:
PS1='[$(claude-acc whoami)] \$ '

# Or in a script:
case "$(claude-acc whoami)" in
    alice@anthropic.com) echo "work" ;;
    *)                   echo "other" ;;
esac
```

The `*` after an email means the OAuth token has rotated since the cache was written. Most often this is a routine OAuth refresh (identity unchanged) — but if you ran `claude auth login` directly between `doctor` runs, this is your reminder to re-verify. Run `claude-acc doctor` to refresh the cache.

### One login, several setups

Linking two account dirs to the **same** Anthropic login is a perfectly valid setup — it lets you keep separate global configs (different `CLAUDE.md`, plugins, agents, MCP servers, output styles) under a single subscription, and switch between them per directory. When `doctor` sees accounts that resolve to the same identity it cross-references them with `↔` so the overlap is intentional and visible, not a surprise:

```
$ claude-acc doctor
Auditing 2 account(s):
  ✓ minimal  alice@anthropic.com  Max 20x  uuid=aa6c22d5-…  ↔ same identity as full
  ✓ full     alice@anthropic.com  Max 20x  uuid=aa6c22d5-…  ↔ same identity as minimal

All accounts healthy.
```

This is just a note, never an error — both accounts share the login (and therefore the same usage limits), only their local config differs.

> **macOS only for now.** The Keychain hashing scheme is reverse-engineered from Claude Code's internals, so non-macOS platforms (where Claude Code uses libsecret / Credential Manager) aren't covered yet.

## Usage tracking (`usage`)

`claude-acc usage` shows how much of each account's rate limit you've burned, so you can pick a fresh account before you hit a wall. For every account (and the standard `~/.claude/` if logged in) it reads the OAuth token, calls `https://api.anthropic.com/api/oauth/usage`, and renders the **5-hour** and **7-day** windows with a bar, a percentage, and the time until each resets:

```
$ claude-acc usage
Claude Code usage:
  ★ work  <alice@anthropic.com>  Max 20x
      5h  [████████░░░░░░░░░░░░]   42%  resets in 2h 14m
      7d  [██░░░░░░░░░░░░░░░░░░]   11%  resets in 5d 17h
    personal  <bob@anthropic.com>  Pro
      5h  [░░░░░░░░░░░░░░░░░░░░]    0%  available now
      7d  [░░░░░░░░░░░░░░░░░░░░]    0%  resets in 6d 3h
```

Unlike `doctor`, the usage figures are always a live fetch — usage is volatile, so nothing is cached. The email/plan next to each account come from `doctor`'s cache, so run `claude-acc doctor` once to populate them. Accounts with no token show `no token (run: claude-acc login <name>)`; an unreachable API shows `token present, but API unreachable`. Same dependencies and platform caveat as `doctor` (`security`, `curl`, `jq`, `shasum`; macOS only for now).

## Language

Auto-detected from `LANG`. Override with:

```bash
export CLAUDE_ACC_LANG=ru  # or en
```

## Example session

```bash
$ claude-acc status
Active account: ~/.claude/ (standard)

$ claude-acc add work
Account 'work' created. Starting login...

$ cd ~/work
$ claude-acc link work
work → account 'work'

$ cd ~/work/secret-project
$ claude-acc status
Active account: work (linked to work)

$ cd ~/hobby/my-bot
$ claude-acc status
Active account: ~/.claude/ (standard)
```

## Switching between Rust and shell

Both versions read and write the same files under `~/.claude-switch/`:

```
~/.claude-switch/
├── accounts/        ← per-account CLAUDE_CONFIG_DIR
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

## Updating

```bash
claude-acc update          # download + install the latest release
claude-acc update --check  # just report whether a newer version exists
```

For the **Rust CLI**, `update` queries the latest GitHub release, and if it's newer than the running binary, downloads the prebuilt asset for your OS/architecture and swaps it in over `~/.claude-switch/bin/claude-acc`. Needs `curl`; prebuilt assets exist for macOS (x86_64/arm64), Linux (x86_64/arm64), and Windows (x86_64). On other platforms, build from source with `cargo install --path .`.

For the **shell script**, `claude-acc update` re-fetches the latest `claude-switch.sh` from GitHub into the file you sourced it from; re-source it (or open a new shell) to pick up the changes.

## Releases

Releases are managed automatically by [release-please](https://github.com/googleapis/release-please). On every push to `master`, an action reads the [conventional-commit](https://www.conventionalcommits.org/) messages and keeps a rolling "Release PR" open with a version bump and changelog. Merging that PR creates a tag and triggers cross-platform binary builds (macOS x64/arm64, Linux x64/arm64, Windows x64) that are attached to the release.

Use these commit-message prefixes so the bump is correct:

| Prefix | Bump |
|---|---|
| `feat:` | minor (`0.1.0 → 0.2.0`) |
| `fix:` / `perf:` / `refactor:` / `docs:` | patch (`0.1.0 → 0.1.1`) |
| `feat!:` or any commit with `BREAKING CHANGE:` in the body | major (`0.1.0 → 1.0.0`) |
| `chore:` / `ci:` / `build:` / `style:` / `test:` | no release |

## License

MIT
