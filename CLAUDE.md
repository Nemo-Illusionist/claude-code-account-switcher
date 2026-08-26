# Contributor / agent notes

## Before pushing — run the full CI check set locally

CI (`.github/workflows`, the **Rust** workflow) fails the build on any of these,
so run all four locally before committing/pushing. Use the exact same flags CI
uses — `cargo clippy` without `--all-targets` and `cargo test` without
`--release` can pass locally while CI still fails.

```sh
cargo fmt --all --check                       # formatting (CI: "cargo fmt --check")
cargo clippy --all-targets -- -D warnings     # lints, warnings are errors
cargo build --release                         # release build
cargo test --release                          # tests, release profile
```

To auto-fix formatting before the check: `cargo fmt --all`.

If you touched shell files (`claude-switch.sh`, shell templates), the **Shell**
workflow also syntax-checks them — `zsh -n` on `claude-switch.sh` and
`shell/init.zsh`, `bash -n` on `shell/init.bash` and `shell/claude-wrapper.sh`,
and a PowerShell parse of `shell/init.ps1`.

## Everything else lives in `.claude/rules/`

Most of these load on their own when the files they govern come into play. But
a path-scoped rule can't fire before you've opened the file — when you're
planning, or creating one — so this index is the backstop. **Read the rule
before starting work it covers.**

| Rule | Applies when | In one line |
| --- | --- | --- |
| `commits-and-prs.md` | always | The PR title becomes the commit and sets the release version. One topic per PR. Never amend or rebase. |
| `tests.md` | Rust changes | Almost every code change ships a test that fails without it. |
| `i18n.md` | Rust changes | Every string a person reads is a `Msg` with an arm per language. Never `println!` prose. |
| `shell-parity.md` | Rust or `claude-switch.sh` | Bug fixes land in the zsh script too, always. Features whenever it can. |
| `cli-completions.md` | new command/flag/argument | Complete it in all three shells, same commit. |
| `readme-parity.md` | any `README*.md` | Change one README, change all of them. Translated samples come from a real run. |
