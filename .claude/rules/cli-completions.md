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

Completion scripts have no unit tests here, and CI only checks that they parse
(`zsh -n`, `bash -n`, and a PowerShell parse of `init.ps1`). To check behaviour,
stub the shells' completion primitives and call the function directly:

- zsh — override `_describe` and `_files`, set `words` and `CURRENT`
- bash — set `COMP_WORDS` and `COMP_CWORD`, read back `COMPREPLY`

Walk every branch you added, including the ones that should offer nothing.
