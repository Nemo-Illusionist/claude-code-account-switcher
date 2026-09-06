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

### If you touched anything `cfg`-gated, add the Windows cross-check

CI runs the four commands above on Linux, macOS **and Windows**. The four
above only cover the platform you are sitting on, so a `#[cfg(unix)]` test —
or the helper it leaves behind — can be clean locally and dead code there,
which `-D warnings` turns into a failed build:

```sh
rustup target add x86_64-pc-windows-msvc      # once
RUSTFLAGS="-D warnings" cargo clippy --all-targets --target x86_64-pc-windows-msvc
```

It is a type-check, not a run, so it takes about a second. Worth it any time
a `cfg` appears in the diff — see `.claude/rules/platform-gating.md`.

If you touched shell files (`claude-switch.sh`, shell templates), the **Shell**
workflow also syntax-checks them — `zsh -n` on `claude-switch.sh` and
`shell/init.zsh`, `bash -n` on `shell/init.bash` and both wrappers
(`shell/claude-wrapper.sh`, `shell/claude-vscode-wrapper.sh`), a `sh -n` pass
over the two wrappers since they declare `#!/bin/sh`, and a PowerShell parse
of `shell/init.ps1`. A new shell file is not covered until it is added there.

## Actions are pinned to a commit SHA

Every `uses:` in `.github/workflows/` names a commit, with the tag it came
from in a trailing comment:

```yaml
- uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4.4.0
```

A tag is mutable — whoever owns the action can move `v4` to a different
commit, and CI would run that code with the workflow's token. This repo
handles other people's credentials, so that is not a theoretical worry.
Dependabot bumps these weekly, SHA and comment together.

**Adding a new action means pinning it too.** Resolve the tag first:

```sh
gh api repos/<owner>/<repo>/git/ref/tags/<tag> --jq '.object.sha'
```

If the action selects its behaviour from the ref it is used at — as
`dtolnay/rust-toolchain@stable` did — say that explicitly in `with:`, because
the pin takes the ref away.

## Everything else lives in `.claude/rules/`

Project conventions beyond the build — commits and PR scope, tests, i18n,
shell parity, completions, README parity. Most load on their own when the
files they govern come into play, but a path-scoped rule can't fire before
you've opened the file. Look there before starting work.
