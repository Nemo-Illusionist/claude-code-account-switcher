# Status line

[← README](../README.md) · [Русский](ru/statusline.md)

Claude Code can show a custom status bar at the bottom of the screen. `claude-acc statusline` renders one that leads with **the account this session is running under** — the one thing Claude Code itself can't show — followed by git branch, model, project, and a context-window meter:

```
work │ ⎇ main │ Opus 4.8 (1M context) │ approvalmax-product-AM-37583 │ ▓▓▓░░░░░░░ 32%
```

Install it into the active account's `settings.json` with one command:

```bash
claude-acc statusline --install
```

Then restart Claude Code. The command reads Claude Code's session JSON on stdin, so the data is free — no API calls. The account badge comes from `CLAUDE_CONFIG_DIR`. Colors honor `NO_COLOR`.

### What the meter is, and is not

It is **the context window of the current session**, from `context_window.remaining_percentage`, colored green → yellow → red and capped with a skull once auto-compaction is imminent. It is omitted early on, before Claude Code populates the block.

It is **not** your subscription quota. That is a different number, and [`claude-acc usage`](identity.md#how-much-rate-limit-is-left-usage) is what shows it — the 5-hour and 7-day rate-limit windows. The two never agree, because they measure different things: one is how full this conversation is, the other is how much of your plan you have spent. Reading the meter as the quota is what [#140](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/140) was about, and these guides were the reason — they described the 5-hour reading long after the meter had stopped showing it.

The percentage is also scaled to the **usable** window rather than the raw one. Claude Code reserves a slice for auto-compaction — ~16.5% by default, or the token count in `CLAUDE_CODE_AUTO_COMPACT_WINDOW` — so a raw "84% free" already means the meter is full. Normalising makes 80% mean "compaction is near" instead of "there's still slack", which is also why this number differs from the one Claude Code shows itself.

Prefer to wire it up by hand? Point `statusLine` at the installed binary:

```json
{
  "statusLine": { "type": "command", "command": "~/.claude-switch/bin/claude-acc statusline" }
}
```

> Status line is a Rust-CLI feature — Claude Code's `statusLine` runs a binary/script path, which the shell-script distribution can't provide as a sourced function.
