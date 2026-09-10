# Status line

[← README](../README.md) · [Русский](ru/statusline.md)

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
