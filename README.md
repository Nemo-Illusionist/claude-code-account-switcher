# Claude Code Account Switcher

[Русская версия](README.ru.md)

Bind different Claude Code accounts to different directories.
On `cd`, the correct account is activated automatically.

![claude-acc: the account follows the directory](assets/demo.gif)

Two distributions:

- **Rust CLI** (`claude-acc`) — cross-platform: macOS, Linux, Windows; zsh, bash, PowerShell. **Recommended.**
- **Shell script** (`claude-switch.sh`) — zsh-only, macOS-focused. Single file, no binary, no compilation.

Both share the same on-disk format (`~/.claude-switch/`) so you can switch between them freely.

## Directory-bound accounts, not a global switch

You don't switch accounts — you `cd`. `CLAUDE_CONFIG_DIR` is resolved per shell from the current directory, so work dirs use the work account and personal dirs use yours, in parallel terminals at the same time. There is no global "currently active account" to forget to switch back to.

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

## Comparison

Other tools solving nearby problems, and how they differ (summarised from their READMEs, August 2026):

| | **claude-acc** | [cswap](https://github.com/realiti4/claude-swap) | [aisw](https://github.com/burakdede/aisw) | direnv + `CLAUDE_CONFIG_DIR` |
|---|---|---|---|---|
| Model | account is a property of the directory | one globally active login (+ optional directory → account map) | one globally active profile per tool | account is a property of the directory |
| Plain `claude` picks the account by cwd | yes, on `cd` | via `cswap run` in a mapped directory | no — `aisw workspace guard` warns/blocks on mismatch | yes, where an `.envrc` exists |
| Directory inheritance and overrides | yes | yes (nearest mapped ancestor) | per-repo / git-remote binds | per-directory `.envrc` |
| Different accounts in parallel terminals | yes | yes (session mode) | no — switching is global | yes |
| Per-account `settings.json`, `CLAUDE.md`, agents, skills, MCP | yes — separate config dir | no — sessions reuse `~/.claude`, only history is separate | partial — isolated home per tool where the tool supports it | yes |
| Live identity audit of a config dir | `doctor` — OAuth profile API: email, plan, UUID | account emails from stored credentials | `doctor` / `verify` check config integrity | no |
| Rate-limit usage | `usage` — 5h / 7d per account | TUI dashboard, macOS menu bar, adaptive polling | no | no |
| Auto-rotation when a limit is hit | no (out of scope) | yes — strategies, cooldown, hysteresis | no | no |
| Status line with the active account | `statusline --install` | no | no | no |
| IDE launches (JetBrains, VS Code) | wrapper on `PATH` + `ide/` symlink | follow the global login | follow the global profile | no |
| Claude **desktop app** accounts | `desktop` — isolated profiles, open side by side | no | no | no |
| Adopt an existing config dir without re-login | `import` — re-keys the macOS Keychain entry | `add` / `import` of credential exports | capture the current login as a profile | n/a |
| Other coding CLIs (Codex, Gemini) | no | no | yes | n/a |
| Runtime | Rust binary (or a single zsh script) | Python (uv / pipx) | Rust | direnv |

Short version: **cswap** if you want one active account plus automatic rotation around rate limits; **aisw** if you juggle several coding CLIs; the **direnv** recipe if you already run direnv and want nothing else installed. `claude-acc` is for keeping accounts *separated* — work, personal, client — with the binding living in the directory tree, and an audit trail of which identity is actually behind each config dir.

## Commands

| Command | Description |
| --- | --- |
| `claude-acc` | Help |
| `claude-acc list` | List all accounts |
| `claude-acc add <name>` | Add account (runs `claude login`); add `-s` / `--seed` to seed from `~/.claude/` |
| `claude-acc clone-settings <name>` | Copy `settings.json` / `CLAUDE.md` / `agents/` / etc. from `~/.claude/` into an existing account |
| `claude-acc import <name> <path>` | Adopt an existing config dir as an account (no re-login); `--move` to relocate |
| `claude-acc login <name>` | Re-login to an account |
| `claude-acc remove <name>` | Remove account |
| `claude-acc default [name]` | Show/set default account |
| `claude-acc reset` | Reset default to `~/.claude/` |
| `claude-acc link <name>` | Link account to current directory |
| `claude-acc unlink` | Unlink current directory |
| `claude-acc links` | Show all directory links |
| `claude-acc status` | Show active account |
| `claude-acc usage` | Show 5h / 7d rate-limit usage for every account |
| `claude-acc sessions [--all]` | List Claude Code sessions across accounts (current directory by default) |
| `claude-acc session copy <id> --to <name>` | Copy a session into another account so `claude --resume` can see it |
| `claude-acc resume-hook [on\|off]` | Show/set whether plain `claude --resume <id>` gets the same check |
| `claude-acc desktop add\|list\|run\|remove [<name>]` | Claude Desktop profiles — separate app profiles that run side by side |
| `claude-acc desktop clone-config <name>` | Copy MCP servers and preferences into a desktop profile (`--from`, `--force`) |
| `claude-acc desktop clone-runtime <name>` | Clone the downloaded runtime into a profile, copy-on-write (macOS/APFS) |
| `claude-acc desktop usage` | Account, plan and 5h / 7d usage behind every desktop profile (macOS) |
| `claude-acc statusline [--install]` | Render (or install) a Claude Code status line with the active account |
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

## Shell completions

`claude-acc install` also wires up Tab completion for zsh, bash and PowerShell. It covers every command and its arguments — account names (with `default` where the command accepts it), `session copy` ids for the current directory, `desktop` profile names, `resume-hook on|off`, `import`'s path, and each command's flags:

```
$ claude-acc session copy <TAB>
363edaeb-e81c-4021-94f4-7fe7d91815f4  0266a566-0336-4055-8f05-c553d368528e

$ claude-acc session copy 0266a566-… --to <TAB>
default  personal  work
```

Session ids are scoped to the current directory on purpose: a full listing runs to hundreds of uuids across every project ever opened, which is not a menu anyone can pick from.

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

## Importing an existing config dir

Already running multiple accounts the manual way — separate `~/.claude-work` / `~/.claude-personal` directories driven by `CLAUDE_CONFIG_DIR` aliases? `import` adopts one of those into a managed account **without making you log in again**:

```bash
claude-acc import work ~/.claude-work          # copy the dir in
claude-acc import work ~/.claude-work --move    # …or move it
```

It copies (or moves) the directory into `~/.claude-switch/accounts/<name>/` and then verifies the identity, printing the email it resolved to.

The catch it handles for you: on macOS, Claude Code stores the OAuth token in the Keychain under a key derived from the **absolute config-dir path**, so a plain copy would orphan the token at the new location. `import` re-keys the Keychain entry to the new path, so auth keeps working — no `claude login` needed. (If the token lives in a plaintext `.credentials.json` instead, it just travels with the directory.) If neither is present, `import` still succeeds and tells you to run `claude-acc login <name>`.

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

## Sessions across accounts (`sessions`)

Claude Code stores a conversation as a transcript inside the config directory it was running under:

```
<CLAUDE_CONFIG_DIR>/projects/<slugified-cwd>/<session-id>.jsonl
```

Because every account here gets its own `CLAUDE_CONFIG_DIR`, every account also gets its own `projects/` tree. That has a consequence worth knowing: **`claude --resume <id>` only ever sees sessions that belong to the account it runs under.** Start a conversation on `work`, hit a limit, switch to `personal`, and `--resume` won't list it — the transcript is still there, just in the other account's directory.

`claude-acc sessions` shows the whole picture. By default it lists the current directory's sessions across every account; `--all` covers every project:

```
$ claude-acc sessions
Sessions for /Users/alice/Documents/my-repo:

  363edaeb-e81c-4021-94f4-7fe7d91815f4  work      just now     9.9 MB
  0266a566-0336-4055-8f05-c553d368528e  work      15h ago       60 KB  ← newest copy
  0266a566-0336-4055-8f05-c553d368528e  personal  6d ago        58 KB

The same session id appears in more than one account — those are separate
copies that have drifted apart. 'claude --resume' only ever sees the copy in
the account it runs under.

Resume one:  claude-acc run <account> --resume <id>
```

The same id can exist in more than one account once a transcript has been copied around. Those copies then drift independently, so the listing flags **which one was updated most recently** — that is usually the one you actually want to continue.

The transcript format itself carries no account identity — no email, no user id, no organization uuid (those live in `.claude.json`, which this command never reads or writes). That's why a transcript is portable between accounts at all.

### Moving a session to another account (`session copy`)

Hit a rate limit mid-task? Copy the conversation into a fresh account and carry on there:

```
$ claude-acc session copy 0266a566-0336-4055-8f05-c553d368528e --to personal

Note: the prompt cache is per-account, so the first message after resuming
under another account re-sends the whole transcript — slower and more expensive
than a normal turn.
Copy it from 'work' to 'personal'? [y/N] y
Copied session 0266a566-0336-4055-8f05-c553d368528e from 'work' to 'personal' (60 KB).
Also copied 3 subagent transcript(s).
Continue it:  claude-acc run personal --resume 0266a566-0336-4055-8f05-c553d368528e
```

It copies the transcript and, when there is one, the sidecar directory of subagent transcripts. `--to default` targets the standard `~/.claude`.

**This copies — it doesn't move.** The original stays where it is, so backing out costs nothing. From then on the two copies are independent: whichever account you actually continue the conversation under is the one whose copy grows.

Prompts you'll see, and how to skip them:

- **Which copy?** — if several accounts already hold this id, you get a numbered pick showing each copy's account, how long ago it was touched, and its size. `--from <account>` answers it up front. This is the one prompt `--force` can't skip: with copies that have drifted apart, guessing risks overwriting the version you wanted.
- **Overwrite?** — if the destination already holds a copy, both are shown side by side (marked `← copying this one` / `← will be replaced`) before you confirm.
- **The cost note** — the prompt cache is per-account, so the first turn after the move re-sends the whole transcript. On a large conversation that is slow and not cheap. Worth knowing before, not after.

`--force` skips the confirmations for scripting.

### `run --resume` checks for you

You don't have to remember any of this up front. When `claude-acc run <account> --resume <id>` names a session that account doesn't have, it says so before starting claude — which would otherwise just report an unknown session, with no hint that the transcript is sitting one account over:

```
$ claude-acc run work --resume 0266a566-0336-4055-8f05-c553d368528e

Session 0266a566-0336-4055-8f05-c553d368528e isn't in account 'work', but another account has it:
  default       15h ago       60 KB

Note: the prompt cache is per-account, so the first message after resuming
under another account re-sends the whole transcript — slower and more expensive
than a normal turn.
Copy it from 'default' into 'work' and resume? [y/N]
```

Answer `n` and claude starts anyway, exactly as before — it reports the unknown session itself.

When the id exists **both here and in another account**, those are two conversations that have drifted apart, and only you know which one you meant. So you get the copies as a numbered pick, with the current account's own copy among them:

```
Session 0266a566-0336-4055-8f05-c553d368528e exists in more than one account. Which copy do you want to resume?
  [1]  default       25m ago       60 KB
  [2]  work          15h ago       60 KB  ← this account, newest

Number (Enter to cancel):
```

Picking this account's copy (or pressing Enter) leaves everything alone. Picking another copies it in first.

Anything else is claude's ordinary behaviour, untouched: an id no other account has, and a bare `--resume` with no id — that opens claude's own session picker, and getting in front of it would only be in the way.

### The same check for plain `claude --resume`

`claude` on your PATH is this tool's wrapper (see [IDE integration](#ide-integration)), so the check doesn't have to be limited to `claude-acc run`. With the hook on — the default — a plain `claude --resume <id>` gets exactly the prompts above:

```
claude-acc resume-hook          # show the current state
claude-acc resume-hook off      # plain `claude --resume` goes straight through
claude-acc resume-hook on
```

The setting lives in `~/.claude-switch/config`. `CLAUDE_ACC_NO_RESUME_HOOK=1` turns it off for a single shell without changing the stored value. `claude-acc run <account> --resume <id>` checks either way — the hook only governs the bare `claude` path.

The wrapper is careful about staying out of the way:

- it does nothing unless `--resume` is actually among the arguments, so an ordinary launch pays nothing;
- it does nothing without a terminal on both stdin and stdout, so scripts, pipes and CI are never prompted at;
- whatever happens, claude still starts — a failure in the check is never a failure to launch.

**macOS and Linux only.** The hook lives in the wrapper script, and there is no wrapper on Windows — PATH-based interception there would need a `.cmd`/`.exe` shim. `claude-acc run <account> --resume <id>` does the same check on every platform.

`claude-acc update` refreshes the wrapper for you; `claude-acc install` does too, if you ever need to force it.

## Claude Desktop profiles (`desktop`)

Everything above is about the CLI. The **desktop app** has the same problem — one app, one signed-in account — and it turns out to have a clean answer.

The app is Electron, so it honours Chromium's `--user-data-dir`. Point it at a directory of our own and it gets a fully isolated profile: its own sign-in, its own settings, its own MCP servers. That is the same move this tool already makes for the CLI with `CLAUDE_CONFIG_DIR` — and it has one property the CLI accounts don't:

> **Profiles run side by side.** There is no "switch". Your work account and your personal account can both be open, in two windows, at the same time. The app takes no single-instance lock, and Chromium's lock lives inside each profile directory.

```bash
claude-acc desktop add work      # create the profile and open Claude on it to sign in
claude-acc desktop add work -s   # ...and seed its MCP servers from the app's own profile
claude-acc desktop list          # profiles, and which account each is signed in as
claude-acc desktop usage         # ...plus their 5h / 7d rate-limit usage, live
claude-acc desktop run work      # open Claude on that profile again
claude-acc desktop clone-config work   # copy MCP servers into an existing profile
claude-acc desktop clone-runtime work  # clone the ~10.5 GB runtime — free on APFS
claude-acc desktop remove work   # delete the profile and everything in it
```

`desktop add` creates `~/.claude-switch/desktop/<name>/` and opens the app on it. The window comes up signed out — sign in there with the account this profile is for:

```
$ claude-acc desktop add work
Desktop profile 'work' created. Opening Claude on it...
It opens signed out — sign in there with the account for this profile.
The profile is fully isolated, so the app re-downloads its sandbox images into it — expect several GB.

Open it again later:  claude-acc desktop run work
```

```
$ claude-acc desktop list
Claude Desktop profiles:
    work  (signed out)
    ~/Library/…/Claude/  (the app's own profile)
```

Once signed in, that row carries the account it belongs to — see [`desktop usage`](#which-account-each-profile-is-signed-in-as-desktop-usage).

The last row is the app's own profile — the one you get when you open Claude from the Dock. Nothing here reads or writes it; it is listed so the picture is complete.

Your main instance is never quit, never touched, and never has its signed-in state copied around. That is the whole reason this approach is worth having: the alternative — quitting the app and swapping profile data on disk — mixes authentication state and triggers server-side re-authentication, which is exactly what the Windows tools in this space keep running into.

**Trade-offs, stated plainly:**

- **Disk.** Isolation is total, so a profile would re-download its whole ~10.5 GB runtime. `clone-runtime` makes that free on APFS — see below. Caches (~1.5 GB) are still per-profile.
- **MCP servers are per-profile.** A new profile starts with none — `-s` or `clone-config` brings them over, see below.
- **`--user-data-dir` is a Chromium switch, not a documented Claude Desktop feature.** This is how VS Code and most Electron apps are routinely run, so the risk is small — but if the app ever pins its own data directory unconditionally, this stops working.
- **Sign in with only one Claude window open.** Signing in finishes through a `claude://` link, which the system hands to whichever window it feels like — do it with two open and both can end up on the same account. `desktop add` says so before it launches.

### Where it works

| | Launching profiles | Which account a profile is on (`desktop usage`) |
|---|---|---|
| **macOS** | yes — `/Applications/Claude.app` or `~/Applications/` | yes |
| **Windows**, installed from [claude.com/download](https://claude.com/download) | yes — `%LOCALAPPDATA%\AnthropicClaude\` | not yet — the key lives in DPAPI, not the Keychain |
| **Windows**, installed from the Microsoft Store | **no** — see below | no |
| **Linux** ([official package](https://code.claude.com/docs/en/desktop-linux), beta) | yes — `claude-desktop` on `PATH` | not yet — libsecret / kwallet |

If the app is somewhere else — an unofficial Linux build, a non-standard install — point at it: `CLAUDE_ACC_DESKTOP_APP=/path/to/the/app`.

**The Microsoft Store build can't do this, and won't pretend to.** Its executable lives under `WindowsApps` and starts only through the Store's own activation, which is no way to pass a command-line switch; the package also redirects file paths, so a switch that did arrive wouldn't point where it says. Rather than launch it and quietly open your real profile while claiming otherwise, `desktop` says what's wrong and stops. The installer from claude.com/download works.

> Reported but unverified by us: on Windows, Cowork resolves its VM image relative to `%APPDATA%`, so a profile kept elsewhere may fail to start one, and only one Cowork VM runs at a time regardless. Plain chat is unaffected.

### Bringing MCP servers along (`clone-config`)

A profile's MCP servers and app preferences live in `claude_desktop_config.json` **inside the profile directory**, so a new profile starts with neither. Re-adding a docker MCP server by hand in every profile gets old fast — this is the desktop analog of [`clone-settings`](#inheriting-claude-config) for CLI accounts:

```bash
claude-acc desktop add work -s               # seed at creation, from the app's own profile
claude-acc desktop clone-config work         # or seed an existing profile
claude-acc desktop clone-config work --from personal   # from another profile instead
```

```
$ claude-acc desktop clone-config work
MCP servers and preferences copied from ~/Library/…/Claude/.
Server definitions only — any that sign in separately will ask for that again in the new profile.
```

An existing config is **kept, not replaced** — it likely holds servers someone added by hand:

```
$ claude-acc desktop clone-config work
This profile already has a claude_desktop_config.json. Replace it with --force.
```

Two things worth knowing:

- **Definitions, not sessions.** An MCP server that authenticates on its own will ask for that again in the new profile — as it should, since the point of a separate profile is a separate identity.
- The file can hold server credentials, so it is copied with its mode intact (`0600` in the app's own profile) and via a staging file, so an interrupted copy can't leave half a config behind.

### Which account each profile is signed in as (`desktop usage`)

`desktop list` reads nothing but files, so it can only say whether a profile holds a credential. `desktop usage` goes further — it decrypts the profile's token and asks the API, giving you the email, the plan, and the same 5h / 7d bars [`usage`](#usage-tracking-usage) shows for CLI accounts:

```
$ claude-acc desktop usage
macOS will now ask for your login keychain password: reading a profile's account and usage means decrypting its token, and the key for that lives in the 'Claude Safe Storage' keychain entry. Declining only costs you this listing.

Claude Desktop usage:
    work  <work@company.com>  Max 20x
      5h  [██████░░░░░░░░░░░░░░]   32%  resets in 52m
      7d  [████████░░░░░░░░░░░░]   40%  resets in 5d 16h
```

It caches what it learns, so `desktop list` shows the email from then on without asking for anything:

```
$ claude-acc desktop list
Claude Desktop profiles:
    work  <work@company.com>  Max 20x  (signed in)
    ~/Library/…/Claude/  (the app's own profile)
```

**About that keychain prompt.** The desktop app stores its token the way every Chromium app does on macOS: encrypted with a key kept in the keychain entry `Claude Safe Storage`, whose access list names only the app itself. Reading it therefore asks you for your login keychain password — once, if you pick "Always Allow". That is a real thing to be asked for, so `desktop usage` says what it is about to do *before* the dialog appears rather than after, and nothing else in this tool ever touches that entry. Decline and you lose this one listing; everything else keeps working.

Without it, a profile still shows its account **uuid**, which sits in plaintext in the profile's own `config.json`:

```
$ claude-acc desktop list
Claude Desktop profiles:
    work  aa6c22d5…  (signed in)
```

Not an identity anyone recognises, but enough to see that two profiles are two different accounts.

> `desktop usage` is a Rust-CLI feature. Decrypting the token needs PBKDF2-HMAC-SHA1 and AES-128-CBC with an explicit key — stock macOS ships LibreSSL, whose `openssl` has no `kdf` subcommand, so the shell script would need Homebrew's OpenSSL or Python to do it. It shows the uuid instead.

### Not paying for the runtime twice (`clone-runtime`)

Most of a profile's weight is components the app downloads and then only reads:

| Size | |
|---|---|
| ~10 GB | `vm_bundles/claudevm.bundle/` — Cowork sandbox images |
| 250 MB | `claude-code-vm/<version>/` |
| 220 MB | `claude-code/<version>/` |

Identical in every profile, and a new profile fetches its own copy of all of it. On APFS it doesn't have to: `clone-runtime` clones them **copy-on-write**, so each profile gets fully independent files that share blocks with the original until one of them is written.

```
$ claude-acc desktop clone-runtime work
Cloned 13 runtime component(s), 10.5 GB logical, from ~/Library/…/Claude/.
Disk actually used: 12 KB.
```

Ten and a half gigabytes, in a third of a second, for twelve kilobytes of directory metadata. `--from <profile>` clones from another profile instead of the app's own; `--force` replaces a runtime the profile already has.

**Why clone rather than share.** The Windows tools in this space point every profile at one `vm_bundles/` directory. They can, because they quit the app before switching, so only one instance ever touches those files. Ours run at the same time — two live VMs writing to one image is corruption, not a saving. A clone has no shared writer at all: writing to one leaves the other byte-for-byte intact, and only the changed blocks get allocated.

**What is not cloned, deliberately.** The sandbox bundle mixes downloads with per-VM identity, and only the first kind may travel:

| Cloned | Left for the app |
|---|---|
| `rootfs.img`, `vmlinuz`, `initrd*` — the images | `machineIdentifier`, `macAddress`, `gvisorMacAddress`, `vmIP` — two live VMs sharing a MAC address is a collision, not a saving |
| `.*.origin` — which image set they came from, so the app doesn't refetch | `sessiondata.img`, `efivars.fd` — this profile's own state |
| `claude-code/<version>/`, `claude-code-vm/<version>/` — whole, they are a download and a `.verified` marker | `Cache/` (1.2 GB), `Code Cache/` (346 MB) — live Chromium caches, written continuously and refilled by the app |

Two caveats, both stated by the command itself:

- **Cross-filesystem clones are refused, not performed.** `cp -c` silently falls back to a real copy when it can't clone, which would spend 10 GB to save 10 GB. If the profile isn't on the same filesystem as the source, the command says so and does nothing.
- **Whether the app accepts a pre-seeded runtime is untested.** The filesystem mechanics are verified; the app's reaction is not — nobody has run Cowork in a profile seeded this way. If it misbehaves, delete that profile's `vm_bundles/`, `claude-code/` and `claude-code-vm/`, and the app fetches its own. Reports either way are welcome in [#92](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/92).

Attachments are unaffected by any of this: Cowork keeps user files at the path in `coworkUserFilesPath`, which lives **outside** the profile and travels with [`clone-config`](#bringing-mcp-servers-along-clone-config), so a new profile points at the files you already have rather than a copy of them.

## Status line

Claude Code can show a custom status bar at the bottom of the screen. `claude-acc statusline` renders one that leads with **the account this session is running under** — the one thing Claude Code itself can't show — followed by git branch, model, project, and a 5-hour rate-limit bar:

```
work │ ⎇ main │ Opus 4.8 (1M context) │ approvalmax-product-AM-37583 │ ▓▓▓░░░░░░░ 32%
```

Install it into the active account's `settings.json` with one command:

```bash
claude-acc statusline --install
```

Then restart Claude Code. The command reads Claude Code's session JSON on stdin, so the data is free — no API calls. The bar shows `rate_limits.five_hour.used_percentage` (the live subscription limit, provided by Claude Code for Pro/Max), colored green → yellow → red as you approach the wall; it's omitted early in a session before Claude Code populates it. The account badge comes from `CLAUDE_CONFIG_DIR`. Colors honor `NO_COLOR`.

Prefer to wire it up by hand? Point `statusLine` at the installed binary:

```json
{
  "statusLine": { "type": "command", "command": "~/.claude-switch/bin/claude-acc statusline" }
}
```

> Status line is a Rust-CLI feature — Claude Code's `statusLine` runs a binary/script path, which the shell-script distribution can't provide as a sourced function.

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
