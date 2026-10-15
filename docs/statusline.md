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

It is **the context window of the current session**, from `context_window.remaining_percentage`, coloured green → yellow → orange → bold red as auto-compaction approaches. It is omitted early on, before Claude Code populates the block.

It is **not** your subscription quota. That is a different number, and [`claude-acc usage`](identity.md#how-much-rate-limit-is-left-usage) is what shows it — the 5-hour and 7-day rate-limit windows. The two never agree, because they measure different things: one is how full this conversation is, the other is how much of your plan you have spent. Reading the meter as the quota is what [#140](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/140) was about, and these guides were the reason — they described the 5-hour reading long after the meter had stopped showing it.

The number is the used share of the whole window, so it **agrees with what Claude Code reports** for the same session: 393.5k of 1M reads as 39% in both places.

The colour is the part that knows about auto-compaction. Claude Code reserves a slice of the window for it — ~16.5% by default, or the token count in `CLAUDE_CODE_AUTO_COMPACT_WINDOW` — so the wall arrives before the bar is full. Severity is measured against that usable range instead of the raw one, which keeps red meaning "compaction is about to kick in" rather than an arbitrary fraction:

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
