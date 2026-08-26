# Commit messages set the version — and the PR title is the commit message

This repo squash-merges. The PR title becomes the single commit on `master`
verbatim, and `release-please` reads it to build the changelog and pick the
next version. **Nothing in CI validates it.** A wrong prefix is silent, and
once merged the tag and changelog are already generated from it.

Get it wrong and:

- `chore:` on a user-facing fix → absent from the changelog, no version bump;
  the fix ships invisibly, and nobody knows to upgrade
- `feat:` on a fix → a minor bump that promises a feature there isn't one
- a breaking change without `!` → ships as a minor, and upgrades break

So the PR title is not a summary you dash off. It is the release note.

## The prefixes this repo uses

Configured in `release-please-config.json`:

| Prefix | Bump | Changelog |
| --- | --- | --- |
| `feat:` | minor | **Features** |
| `fix:` | patch | **Bug Fixes** |
| `perf:` | patch | **Performance** |
| `docs:` | patch | **Documentation** |
| `refactor:` | patch | **Refactors** |
| `revert:` | patch | **Reverts** |
| `chore:` `ci:` `build:` `test:` `style:` | patch | hidden |
| any + `!`, or `BREAKING CHANGE:` in the body | **major** | highlighted |

Pick by what the change does **for a user**, not by how it felt to write. A
one-line change that fixes broken behaviour is `fix:`. A large refactor nobody
can observe is `refactor:`.

Write the subject in the imperative and lowercase after the prefix, as an
actual sentence about the change: `fix: refresh the claude wrapper on update`,
not `fix: wrapper bug`.

## One topic per PR

One problem, or one feature, per PR. Not two related ones, not "while I was in
there".

A PR that does two things cannot be reviewed as either, cannot be reverted
without taking the other with it, and gets one changelog line for two changes
— so one of them is invisible to whoever reads the release notes.

If a change turns out to need a second, unrelated fix to work, that is two
PRs, the second stacked on the first. Say so in the description.

Noticing something unrelated mid-change is normal. Note it, finish what you're
doing, do it separately.

## History

Add commits as work progresses. Never `--amend` anything already pushed, never
force-push a branch under review — the branch history is how a reviewer sees
what changed after their comments. Squash-merge collapses it at the end, so
extra commits cost nothing.

To bring a branch up to date, **merge** `master` into it. Never rebase.
