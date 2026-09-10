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
- Install the IDE wrapper at `~/.claude-switch/bin/claude` (see [IDE integration](docs/ide.md))
- Auto-detect your shell (zsh/bash/PowerShell)
- Add shell integration to your rc file

To update later, just run `claude-acc update` — it downloads the latest release binary for your platform and swaps it in. (Or download a new binary manually and run `claude-acc install` again.)

Building from source, the extra steps Windows needs, and the zsh script are in
[Installation and platforms](docs/install.md).

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

## What you get

- **The account follows the directory.** `cd` into a linked tree and plain `claude` is on that account — in every terminal, at the same time, with no global state to switch back. See [Directory inheritance](#directory-inheritance).
- **Each account is a whole config dir** — its own `settings.json`, `CLAUDE.md`, agents, skills, plugins, MCP servers and history, not just a login. See [Accounts and configuration](docs/accounts.md).
- **IDEs follow too.** JetBrains and the VS Code terminal go through a wrapper on `PATH`; the VS Code extension's native UI takes one more command. See [IDE integration](docs/ide.md).
- **You can see who each config dir is signed in as** — live, from the OAuth profile API — and pin it, so a re-login with the wrong browser session is reported instead of silently rebinding. See [Identities](docs/identity.md).
- **Rate limits per account**, 5h and 7d, so you can pick a fresh one before you hit a wall. See [`usage`](docs/identity.md#how-much-rate-limit-is-left-usage).
- **Conversations can move between accounts.** Hit a limit mid-task and carry on elsewhere — `--resume` offers it for you. See [Sessions across accounts](docs/sessions.md).
- **The desktop app too.** Isolated profiles that run side by side, in two windows, on two accounts. See [Claude Desktop profiles](docs/desktop.md).

## Commands

| Command | Description |
| --- | --- |
| `claude-acc` | Help |
| `claude-acc list` | List all accounts |
| `claude-acc add <name>` | Add account (runs `claude login`); add `-s` / `--seed` to seed from `~/.claude/` |
| `claude-acc clone-settings <name>` | Copy `settings.json` / `CLAUDE.md` / `agents/` / `plugins/` / etc. from `~/.claude/` into an existing account |
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
| `claude-acc session copy <id\|name> --to <name>` | Copy a session into another account so `claude --resume` can see it |
| `claude-acc resume-hook [on\|off]` | Show/set whether plain `claude --resume <id>` gets the same check |
| `claude-acc desktop add\|list\|run\|remove [<name>]` | Claude Desktop profiles — separate app profiles that run side by side |
| `claude-acc desktop clone-config <name>` | Copy MCP servers and preferences into a desktop profile (`--from`, `--force`) |
| `claude-acc desktop clone-runtime <name>` | Clone the downloaded runtime into a profile, copy-on-write (macOS/APFS) |
| `claude-acc desktop usage` | Account, plan and 5h / 7d usage behind every desktop profile (macOS) |
| `claude-acc vscode install\|uninstall\|status` | Wire the VS Code extension's native UI up to directory-bound accounts |
| `claude-acc statusline [--install]` | Render (or install) a Claude Code status line with the active account |
| `claude-acc run <name>` | Run claude under a specific account |
| `claude-acc whoami` | Print the email (or name) of the active account |
| `claude-acc lock <name>` | Pin an account to the identity it is signed in as (`--force` to re-pin) |
| `claude-acc doctor [--json]` | Audit each account's actual OAuth identity, and report drift from the pin |
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

## Guides

The README is the overview. Each topic has its own page:

| Want to | Read |
| --- | --- |
| Install on Windows, build from source, use the zsh script, move between the two | [Installation and platforms](docs/install.md) |
| See what an account holds, inherit your `~/.claude/` setup, adopt an existing config dir | [Accounts and configuration](docs/accounts.md) |
| Make JetBrains IDEs and VS Code follow the directory's account | [IDE integration](docs/ide.md) |
| Continue a conversation under another account after hitting a limit | [Sessions across accounts](docs/sessions.md) |
| Check which Anthropic account a config dir is really signed in as, and how much limit is left | [Identities: `doctor`, `lock`, `usage`](docs/identity.md) |
| Run several Claude Desktop accounts side by side | [Claude Desktop profiles](docs/desktop.md) |
| Show the active account in Claude Code's status bar | [Status line](docs/statusline.md) |
| See how this compares to cswap, aisw and the direnv recipe | [Comparison with other tools](docs/comparison.md) |

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

## Updating

```bash
claude-acc update          # download + install the latest release
claude-acc update --check  # just report whether a newer version exists
```

For the **Rust CLI**, `update` queries the latest GitHub release, and if it's newer than the running binary, downloads the prebuilt asset for your OS/architecture and swaps it in over `~/.claude-switch/bin/claude-acc`. Needs `curl`; prebuilt assets exist for macOS (x86_64/arm64), Linux (x86_64/arm64), and Windows (x86_64). On other platforms, build from source with `cargo install --path .`.

For the **shell script**, `claude-acc update` re-fetches the latest `claude-switch.sh` from GitHub into the file you sourced it from; re-source it (or open a new shell) to pick up the changes.

## License

MIT
