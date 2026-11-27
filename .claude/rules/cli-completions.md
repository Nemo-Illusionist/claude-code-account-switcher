---
paths:
  - "src/main.rs"
  - "src/commands/completions.rs"
  - "shell/init.zsh"
  - "shell/init.bash"
  - "shell/init.ps1"
---

# A new command isn't done until it completes

Adding a command, subcommand, flag, or positional argument to the CLI means
touching all three completion scripts in the same commit — not just `main.rs`:

- `shell/init.zsh`
- `shell/init.bash`
- `shell/init.ps1`

All three, every time. A command that completes in zsh but not bash is worse
than one that completes nowhere, because the gap looks like a bug in the shell.

Cover everything a user can type after the command name: the name itself, its
positional arguments, and its flags.

## Values the tool already knows

If an argument takes a value that exists somewhere in the program — account
names, session ids — serve it from `src/commands/completions.rs`
(`claude-acc completions <what>`) instead of hardcoding a list in three
different shells.

Keep any such list scoped to something a human can pick from. Session ids are
deliberately limited to the current directory's project: the full set is
hundreds of uuids across every project ever opened.

## Also

Add the command to the table in **both** READMEs — see `readme-parity.md`.

## Verifying

CI checks that all three parse (`zsh -n`, `bash -n`, and a PowerShell parse of
`init.ps1`). Behaviour is another matter: `tests/shell/completion_nounset.zsh`
is the only test that sources one of them, and it covers `shell/init.zsh`
alone. `init.bash` and `init.ps1` have nothing. So a new branch is checked by
hand, which means stubbing the shells' completion primitives and calling the
function directly:

- zsh — override `_describe` and `_files`, set `words` and `CURRENT`
- bash — set `COMP_WORDS` and `COMP_CWORD`, read back `COMPREPLY`

`completion_nounset.zsh` is the worked example, with the
`__CLAUDE_ACC_BIN__` substitution, a stub binary, and a bash half driven as
a separate `set -u` process. Add a case there rather than writing a second
harness.

Two things it does **not** model, both learned the hard way:

- It calls the completion function directly, so it runs with whatever
  options the test set. A real zsh completion does not: `_main_complete`
  evals `_comp_setup`, which applies `${_comp_options[@]}` — a list that
  includes `unset`, `nullglob`, `extendedglob`, `bareglobqual` and
  `NO_errexit`. So `setopt nounset` is **off** inside any real completion,
  and a glob behaves differently there than in the test. A failure seen only
  through this harness may not be reachable at all; one of mine was not.
- bash gets no such relaxation. `set -u` is in force while `init.bash` is
  sourced, while the prompt hook runs, and while completing — which is where
  the real instances of that class were.

Walk every branch you added, including the ones that should offer nothing.
And capture stderr separately: a helper that swallows it will report a
completion that aborted as a completion that offered nothing in silence.
