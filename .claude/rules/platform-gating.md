---
paths:
  - "src/*.rs"
  - "src/commands/*.rs"
---

# `cfg`-gate the module, not each item inside it

CI builds on Linux, macOS and Windows with `-D warnings`, so on every
platform *unused* is an error. That makes platform gating asymmetric in a way
that is easy to get wrong: gating what obviously needs gating, and leaving
behind something that only that code used.

The shape that bit us (#103, fixed in #104): two tests that create symlinks,
correctly marked `#[cfg(unix)]` — and a `scratch()` helper and a
`use super::*` left outside the gate. On Windows the module compiled to
nothing but an unused import and an unused function. Every other job passed;
Windows failed on dead code.

## The rule

**If every test in a module is gated, gate the module:**

```rust
// Every test here creates symlinks, so the whole module is unix-only.
#[cfg(all(test, unix))]
mod tests {
```

Not `#[cfg(test)] mod tests` with a `#[cfg(unix)]` on each `#[test]` — that
leaves helpers and imports stranded.

**If only some tests are gated**, then the helpers they use must be gated the
same way, and so must any import only they need. Gating a whole module is
simpler and states the reason once, so prefer restructuring until that is
possible.

The same applies outside tests: a `#[cfg(not(windows))]` function whose only
caller is also `#[cfg(not(windows))]` is fine, but a constant, an import, or
a helper used *only* from gated code needs the same gate.

## Before pushing

Type-check the platform you are not on. It is about a second, and it is the
only local check that sees this class at all:

```sh
RUSTFLAGS="-D warnings" cargo clippy --all-targets --target x86_64-pc-windows-msvc
```

`rustup target add x86_64-pc-windows-msvc` once, first. On a Windows machine,
cross-check `x86_64-unknown-linux-gnu` instead — the direction that matters
is "the platform whose `cfg` branches your local build never compiled".

## Merging

Green CI is a precondition for merge, not a formality. Where the ruleset lets
an admin override it, the override exists to get past the *review*
requirement on a solo repo — never past a red build. Read the check results
before merging; do not chain a merge onto the command that watches them.
