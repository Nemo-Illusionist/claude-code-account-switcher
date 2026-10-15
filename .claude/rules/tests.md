---
paths:
  - "src/*.rs"
  - "src/commands/*.rs"
  - "claude-switch.sh"
  - "tests/shell/*.zsh"
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

## A test that can pass for the wrong reason is not a test

Three ways a green test has lied here. All three are cheap to rule out.

**The expected value must come from somewhere other than the code under
test.** Compute it by hand, or with a different tool, and paste it in as a
literal. Deriving it from our own implementation means a wrong implementation
agrees with its own test forever.

In #150 the Keychain hashes were computed in Node — the engine Claude Code
runs on — rather than by this code. That is the only reason a double-encoding
bug surfaced: the shell helper fed perl raw bytes, so NFC did nothing and the
output was re-encoded, producing a hash matching *neither* spelling of the
path. A self-derived expectation would have matched it happily.

**Assert the value, not merely that it differs.** `!=` is satisfied by
nothing at all. A `keychain_service.zsh` check passed on `master` because the
function did not exist, the comparison returned empty, and empty is `!=` the
expected name. Require a well-formed answer *and* the right one.

The same trap in a different shape: checking "does any emitted SGR code equal
`5`" to assert blink is gone false-positives on `38;5;208`, where the `5`
selects 256-colour mode. Assert over the exact set of codes.

**Run the new test against the base branch and read the failure.** Not just
that it fails — *why*. "33 checks failed" is the signal; "command not found"
where you expected a wrong value means you are pinning a new helper rather
than the bug. A throwaway worktree is the cheap way:

```sh
wt=$(mktemp -d)/wt && git worktree add -q "$wt" master
cp tests/shell/<new>.zsh "$wt/tests/shell/" && (cd "$wt" && zsh tests/shell/<new>.zsh)
git worktree remove --force "$wt"
```

## No test may depend on process-wide mutable state

`cargo test` runs tests in threads of one process. A test that sets or clears
an environment variable passes alone and then fails inside the full suite,
depending on scheduling — and the failure looks like a flake rather than the
design error it is.

Extract the decision as a function taking the value as a parameter:
`used_of_usable(remaining, total, acw)`, `severity_code(pct)`,
`service_name_for(dir, securestorage)`. The thin wrapper that reads the
environment needs no test; the logic gets all of them.

## Exploratory sweeps are read-only

When looping over every command to see which ones misbehave, leave out the
ones that write. `claude-acc update` downloads `master`'s copy of
`claude-switch.sh` over the live file — run inside a sweep from a working
copy, it silently reverted the fix being tested. `tests/shell/nounset.zsh`
excludes it, with a comment saying why.

The same applies to a probe that polls: distinguish "the answer is no" from
"the command failed". A `gh pr diff` returning nothing because of a transient
error reads identically to a changelog entry being absent, and reported the
opposite of the truth.

## Before pushing

`cargo test --release` — the release profile, matching CI. See `CLAUDE.md` for
the full check set.
