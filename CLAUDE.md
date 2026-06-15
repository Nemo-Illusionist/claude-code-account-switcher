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
workflow also runs `zsh -n` / `bash -n` syntax checks on them.
