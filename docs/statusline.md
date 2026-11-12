# Status line

[← README](../README.md) · [Русский](ru/statusline.md)

Claude Code can show a custom status bar at the bottom of the screen. `claude-acc statusline` renders one that leads with **the account this session is running under** — the one thing Claude Code itself can't show — followed by git branch, model, project, and a context-window meter:

```
work │ ⎇ main │ Opus 4.8 (1M context) │ my-repo │ ▓▓▓░░░░░░░  39%
```

Install it into the active account's `settings.json` with one command:

```bash
claude-acc statusline --install
```

Then restart Claude Code. The command reads Claude Code's session JSON on stdin, so the data is free — no API calls. The account badge comes from `CLAUDE_CONFIG_DIR`. Colors honor `NO_COLOR`.

### What the meter is, and is not

It is **the context window of the current session**, from `context_window.remaining_percentage` against the window the session is held to, coloured green → yellow → orange → bold red as auto-compaction approaches. It is omitted early on, before Claude Code populates the block.

It is **not** your subscription quota. That is a different number, and [`claude-acc usage`](identity.md#how-much-rate-limit-is-left-usage) is what shows it — the 5-hour and 7-day rate-limit windows. The two never agree, because they measure different things: one is how full this conversation is, the other is how much of your plan you have spent. Reading the meter as the quota is what [#140](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/140) was about, and these guides were the reason — they described the 5-hour reading long after the meter had stopped showing it.

The number is the used share of the window **this session is actually held to**, counted from the same token sum Claude Code counts, so it reads as Claude Code reads it: 393.5k of 1M is 39% in both places. (Claude Code's own footer additionally keeps an output reserve of up to 20k tokens out of the window it divides by, so once the window is small its figure runs a few points ahead of this one.)

Hold a session to a smaller window and that is what the meter counts against. The same 393.5k under a 400k window reads as 98%, not 39% — and 400k is the window Claude Code is counting down to. The status line payload carries only the *model's* window, so the smaller one is resolved here, the way Claude Code resolves it: `CLAUDE_CODE_AUTO_COMPACT_WINDOW` first, then the highest-precedence settings file that names one, and never larger than the model's own window.

| Where | Key |
|---|---|
| environment | `CLAUDE_CODE_AUTO_COMPACT_WINDOW`, a window in tokens — `400k` and `0.5m` are accepted, a value outside the bounds is clamped into them, and it takes precedence over every setting |
| `<project>/.claude/settings.local.json`, then `.claude/settings.json`, then the account's `settings.json` | `modelSettings.<model>.autoCompactWindow` for one model, else the top-level `autoCompactWindow` |

A *setting* outside the 100000–1000000 Claude Code accepts is ignored here as it is there — only the environment variable is clamped, which is Claude Code's own asymmetry — and `"auto"` means the model keeps its own window. Two sources are **not** read: an organisation's managed settings, and the server-side and experiment defaults Claude Code consults below the settings files. A window lowered through one of those leaves the meter reading low.

The colour is the part that knows about auto-compaction: a slice of that window is kept back for the compaction summary, so the wall arrives before the bar is full. Severity is measured against the window less a 16.5% margin rather than against the whole of it, which keeps red meaning "compaction is about to kick in" rather than an arbitrary fraction:

| | |
|---|---|
| green | under half the usable window |
| yellow → orange | past half, then two thirds |
| bold red | compaction is imminent |

So the digits answer *how full is this conversation*, and the colour answers *how close is the wall*. Earlier versions scaled the digits too, which made the same session read 47% here and 39% in the application — two numbers for one quantity, with nothing on screen to say which was which.

Prefer to wire it up by hand? Point `statusLine` at the installed binary:

```json
{
  "statusLine": { "type": "command", "command": "~/.claude-switch/bin/claude-acc statusline" }
}
```

> Status line is a Rust-CLI feature — Claude Code's `statusLine` runs a binary/script path, which the shell-script distribution can't provide as a sourced function.
