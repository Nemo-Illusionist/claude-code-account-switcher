# Comparison with other tools

[← README](../README.md) · [Русский](ru/comparison.md)

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
| IDE launches (JetBrains, VS Code terminal) | wrapper on `PATH` + `ide/` symlink | follow the global login | follow the global profile | no |
| VS Code **native UI** (the extension's own panel) | `vscode install` — `claudeCode.claudeProcessWrapper` | no | no | no |
| Claude **desktop app** accounts | `desktop` — isolated profiles, open side by side | no | no | no |
| Adopt an existing config dir without re-login | `import` — re-keys the macOS Keychain entry | `add` / `import` of credential exports | capture the current login as a profile | n/a |
| Other coding CLIs (Codex, Gemini) | no | no | yes | n/a |
| Runtime | Rust binary (or a single zsh script) | Python (uv / pipx) | Rust | direnv |

Short version: **cswap** if you want one active account plus automatic rotation around rate limits; **aisw** if you juggle several coding CLIs; the **direnv** recipe if you already run direnv and want nothing else installed. `claude-acc` is for keeping accounts *separated* — work, personal, client — with the binding living in the directory tree, and an audit trail of which identity is actually behind each config dir.
