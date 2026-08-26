---
paths:
  - "src/*.rs"
  - "src/commands/*.rs"
---

# Almost every code change ships a test proving it works

The default is a test, in the same PR. Not a follow-up PR, not "this one is
too small". If you changed what the program does, there is a test showing it
now does it — one that fails without your change.

#61, #62, #64 and #65 all merged with no tests at all, and needed #66 to catch
up afterwards. By then the untested code had already been released.

## The exceptions are narrow, and this isn't one of them

A change may go without a test only when it provably cannot alter behaviour:

- comments, doc comments, README, PR text
- a rename or a move with no logic change
- formatting, or a mechanical `cargo clippy --fix`

That's the list. **"I couldn't think of a good test" is not on it** — see the
next section; it almost always means the decision needs separating from the
side effect, not that the change is untestable.

Reaching for an exception is worth a sentence in the PR description saying
which one and why.

## Make the decision testable

Most of this codebase's logic is reachable without touching a filesystem, a
network, or a prompt — because the decision was deliberately split out into a
pure function. Keep doing that. It is the difference between a testable change
and an untestable one:

- `build_command` / `build_login_command` return a configured `Command`, so
  the env vars can be asserted without ever spawning `claude`
- `pick_source`, `plan_resume`, `resume_id` decide *what to do*; the caller
  does the prompting and the I/O
- `project_slug`, `human_size`, `normalize_version`, `parse_state` are plain
  value-in, value-out

If a new behaviour is hard to test, that is usually the code telling you the
decision and the side effect are tangled together. Separate them rather than
skipping the test.

## Conventions here

- `#[cfg(test)] mod tests` at the bottom of the file being tested
- No test-only dependencies — plain `cargo test`
- Filesystem tests build a scratch dir under
  `std::env::temp_dir().join(format!("cc-<what>-{}", std::process::id()))`
  and remove it at the end
- Name tests after the behaviour, as a sentence:
  `equal_timestamps_are_not_treated_as_newer`, not `test_plan_2`
- When a test exists because of a specific bug, say so in a comment. Future
  readers can't otherwise tell a load-bearing assertion from an obvious one:
  `// Regression: set_default used to rewrite the whole file`

## What is worth a test

Not coverage for its own sake. Test the things that would actually go wrong:

- every branch of a decision function, including the "do nothing" one
- boundaries — empty input, one item, several, equal values
- the case the change exists to fix, written so it fails without the fix
- anything platform-conditional, on both sides (`cfg!(windows)`)

## Before pushing

`cargo test --release` — the release profile, matching CI. See `CLAUDE.md` for
the full check set.
