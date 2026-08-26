---
paths:
  - "src/main.rs"
  - "src/commands/*.rs"
  - "claude-switch.sh"
---

# Keep the zsh script in step with the Rust CLI

`claude-switch.sh` (macOS / zsh, sourced from `~/.zshrc`) is a real, supported
way to use this tool, not a deprecated leftover. People run it today.

## Bug fixes: always, no test to apply

**A bug fix in behaviour the script also implements gets fixed in the script
too. Every time, same PR.** There is nothing to weigh here.

The bug exists in both. Fixing one side means shipping a changelog entry and a
release that say it's fixed while half the users still hit it — which is worse
than not having fixed it at all, because now nobody will look again.

Before you call a fix done, ask: does `claude-switch.sh` have this code path?
If yes, it has the bug, and it is part of the fix.

## Features: if it can be supported without trouble, it must be

Port it in the same PR as the Rust change. That is the whole test — not "is it
worth it", not "will anyone notice". If the script can do it without a fight,
it does it.

"Without trouble" means the script already has the machinery: it reads the same
`~/.claude-switch/` layout, calls the same `security` / `curl` / `jq`, and
prints the same kinds of messages. Most command and flag changes land here.

## When it genuinely can't

Only when porting would mean reimplementing a substantial Rust subsystem in
zsh, or the feature depends on machinery the script has no equivalent for.

Then say so **explicitly in the PR description, with the reason**. Skipping is
a decision that gets written down, never a silent omission.

## Where the two already differ

The script implements 18 commands. Absent for the reasons above:

| Command | Why it's Rust-only |
| --- | --- |
| `statusline` | renders from JSON on stdin |
| `sessions` | needs the cross-account `projects/` walk |
| `session copy` | same, plus the copy and sidecar machinery |
| `resume-hook` | the script generates its own `claude` wrapper |
| `install` | the script is sourced, never installed |

Adding a row is allowed. Adding one silently is not.

## Before pushing

If you touched `claude-switch.sh`, CI runs `zsh -n` on it. Run it locally too.
