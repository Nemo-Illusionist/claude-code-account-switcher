# Accounts and configuration

[← README](../README.md) · [Русский](ru/accounts.md)

What an account actually is on disk, what it inherits from `~/.claude/`, and
how to adopt a config directory you already have.

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
- `plugins/` — the installed plugins and the marketplaces they came from

**Not copied** (per-account state — would defeat the isolation):
- `.credentials.json` (auth token — re-acquired via `claude auth login`)
- `settings.local.json` (per-machine local overrides)
- `projects/`, `todos/`, `statsig/` (sessions, runtime state, telemetry)
- `hooks/` (settings.json references these by absolute path; copying duplicates files for nothing)
- `.account-info.json` (the doctor cache)

### What happens to plugins

Claude Code keeps its plugin registry **per config dir**, so a new account
starts with none — which is why they are seeded rather than left out.

They cannot simply be copied, though: the registry records absolute paths
into the config dir it was written for. A plain copy would leave the new
account loading the old one's plugin cache — working by accident, and broken
the moment that account is removed. So `installPath` and `installLocation`
are rewritten to point at the new account. A marketplace sourced from outside
the config dir — one checked into a project, say — keeps its path exactly as
it was, which is why the rewrite matches on those fields and that prefix
rather than substituting text.

An account that has already installed plugins of its own is left alone.
Seeding is not a merge, and what you installed there is yours.

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

## Importing an existing config dir

Already running multiple accounts the manual way — separate `~/.claude-work` / `~/.claude-personal` directories driven by `CLAUDE_CONFIG_DIR` aliases? `import` adopts one of those into a managed account **without making you log in again**:

```bash
claude-acc import work ~/.claude-work          # copy the dir in
claude-acc import work ~/.claude-work --move    # …or move it
```

It copies (or moves) the directory into `~/.claude-switch/accounts/<name>/` and then verifies the identity, printing the email it resolved to.

The catch it handles for you: on macOS, Claude Code stores the OAuth token in the Keychain under a key derived from the **absolute config-dir path**, so a plain copy would orphan the token at the new location. `import` re-keys the Keychain entry to the new path, so auth keeps working — no `claude login` needed. (If the token lives in a plaintext `.credentials.json` instead, it just travels with the directory.) If neither is present, `import` still succeeds and tells you to run `claude-acc login <name>`.
