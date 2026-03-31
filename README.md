# Claude Code Account Switcher

[Русская версия](README.ru.md)

Bind different Claude Code accounts to different directories.
On `cd`, the correct account is activated automatically.

Cross-platform: macOS, Linux, Windows. Supports zsh, bash, PowerShell.

## Install

### From binary

Download from [GitHub Releases](https://github.com/Nemo-Illusionist/claude-code-account-switcher/releases), then run:

```bash
claude-acc install
```

This will:
- Copy the binary to `~/.claude-switch/bin/claude-acc`
- Auto-detect your shell (zsh/bash/PowerShell)
- Add shell integration to your rc file

To update, download the new version and run `claude-acc install` again.

### From source

```bash
cargo install --path .
claude-acc install
```

### Legacy (zsh-only script)

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

## License

MIT
