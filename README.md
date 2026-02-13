# Claude Code Account Switcher (macOS)

[Русская версия](README.ru.md)

Bind different Claude Code accounts to different directories.
On `cd`, the correct account is activated automatically.

## Install

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
| `claude-acc remove <name>` | Remove account |
| `claude-acc default [name]` | Show/set default account |
| `claude-acc reset` | Reset default to `~/.claude/` |
| `claude-acc link <name>` | Link account to current directory |
| `claude-acc unlink` | Unlink current directory |
| `claude-acc status` | Show active account |

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

Linking `~/work` to account `work` means all subdirectories
(`~/work/project-a`, `~/work/project-b/src`) inherit that account.

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
