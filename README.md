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
- Copy the binary to `~/.claude-switch/bin/claude-acc`
- Install the IDE wrapper at `~/.claude-switch/bin/claude` (see [IDE integration](#ide-integration))
- Auto-detect your shell (zsh/bash/PowerShell)
- Add shell integration to your rc file

To update, download the new version and run `claude-acc install` again.

#### From source

```bash
cargo install --path .
claude-acc install
```

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
| `claude-acc add <name>` | Add account (runs `claude login`) |
| `claude-acc login <name>` | Re-login to an account |
| `claude-acc remove <name>` | Remove account |
| `claude-acc default [name]` | Show/set default account |
| `claude-acc reset` | Reset default to `~/.claude/` |
| `claude-acc link <name>` | Link account to current directory |
| `claude-acc unlink` | Unlink current directory |
| `claude-acc links` | Show all directory links |
| `claude-acc status` | Show active account |
| `claude-acc run <name>` | Run claude under a specific account |
| `claude-acc doctor` | Audit each account's actual OAuth identity |
| `claude-acc install` | Install binary and shell integration |

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

`claude-acc doctor` reads each account's OAuth token from the macOS Keychain (with a `.credentials.json` fallback for non-Keychain installs), calls `https://api.anthropic.com/api/oauth/profile`, and prints the live email + UUID:

```
$ claude-acc doctor
Auditing 2 account(s):
  ✓ work      alice@anthropic.com  uuid=aa6c22d5-…
  ? personal  no token (run: claude-acc login personal)

1 of 2 accounts healthy.
```

It's purely a read-only audit — nothing is intercepted, no `claude` invocation is gated. Run it whenever you want to confirm a config dir is bound to the identity you expect. Requires `security`, `curl`, `jq`, and `shasum` (all preinstalled on macOS); the Rust binary uses native `serde_json` and `sha2` instead and only shells out to `security` and `curl`.

`doctor` also caches the result to `~/.claude-switch/accounts/<name>/.account-info.json` so `list`, `status`, and `default` can show the email next to each account without re-hitting the API:

```
$ claude-acc list
Claude Code accounts:
  ★ work       (default)  alice@anthropic.com   3d ago
    personal              bob@anthropic.com     1h ago *
    ~/.claude/            charlie@personal.com  3d ago    (standard)

$ claude-acc status
Active account: work <alice@anthropic.com> (linked to my-project)

$ claude-acc default
Default: work <alice@anthropic.com>
```

`doctor` audits the standard `~/.claude/` config too (the unmanaged identity used when no link / configured default applies). Its cache lives at `~/.claude-switch/default.account-info.json`. The `~/.claude/` row appears in `list` only after you've actually logged into Claude Code with the standard config (or after `doctor` has cached an identity for it).

The `*` after an email means the OAuth token has rotated since the cache was written. Most often this is a routine OAuth refresh (identity unchanged) — but if you ran `claude auth login` directly between `doctor` runs, this is your reminder to re-verify. Run `claude-acc doctor` to refresh the cache.

> **macOS only for now.** The Keychain hashing scheme is reverse-engineered from Claude Code's internals, so non-macOS platforms (where Claude Code uses libsecret / Credential Manager) aren't covered yet.

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
