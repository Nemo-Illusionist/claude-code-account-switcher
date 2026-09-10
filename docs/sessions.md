# Sessions across accounts

[← README](../README.md) · [Русский](ru/sessions.md)

Claude Code stores a conversation as a transcript inside the config directory it was running under:

```
<CLAUDE_CONFIG_DIR>/projects/<slugified-cwd>/<session-id>.jsonl
```

Because every account here gets its own `CLAUDE_CONFIG_DIR`, every account also gets its own `projects/` tree. That has a consequence worth knowing: **`claude --resume <id>` only ever sees sessions that belong to the account it runs under.** Start a conversation on `work`, hit a limit, switch to `personal`, and `--resume` won't list it — the transcript is still there, just in the other account's directory.

`claude-acc sessions` shows the whole picture. By default it lists the current directory's sessions across every account; `--all` covers every project:

```
$ claude-acc sessions
Sessions for /Users/alice/Documents/my-repo:

  363edaeb-e81c-4021-94f4-7fe7d91815f4  work      just now     9.9 MB
  0266a566-0336-4055-8f05-c553d368528e  work      15h ago       60 KB  ← newest copy
  0266a566-0336-4055-8f05-c553d368528e  personal  6d ago        58 KB

The same session id appears in more than one account — those are separate
copies that have drifted apart. 'claude --resume' only ever sees the copy in
the account it runs under.

Resume one:  claude-acc run <account> --resume <id>
```

The same id can exist in more than one account once a transcript has been copied around. Those copies then drift independently, so the listing flags **which one was updated most recently** — that is usually the one you actually want to continue.

The transcript format itself carries no account identity — no email, no user id, no organization uuid (those live in `.claude.json`, which this command never reads or writes). That's why a transcript is portable between accounts at all.

### Moving a session to another account (`session copy`)

Hit a rate limit mid-task? Copy the conversation into a fresh account and carry on there:

```
$ claude-acc session copy 0266a566-0336-4055-8f05-c553d368528e --to personal

Note: the prompt cache is per-account, so the first message after resuming
under another account re-sends the whole transcript — slower and more expensive
than a normal turn.
Copy it from 'work' to 'personal'? [y/N] y
Copied session 0266a566-0336-4055-8f05-c553d368528e from 'work' to 'personal' (60 KB).
Also copied 3 subagent transcript(s).
Continue it:  claude-acc run personal --resume 0266a566-0336-4055-8f05-c553d368528e
```

It copies the transcript and, when there is one, the sidecar directory of subagent transcripts. `--to default` targets the standard `~/.claude`.

**This copies — it doesn't move.** The original stays where it is, so backing out costs nothing. From then on the two copies are independent: whichever account you actually continue the conversation under is the one whose copy grows.

Prompts you'll see, and how to skip them:

- **Which copy?** — if several accounts already hold this id, you get a numbered pick showing each copy's account, how long ago it was touched, and its size. `--from <account>` answers it up front. This is the one prompt `--force` can't skip: with copies that have drifted apart, guessing risks overwriting the version you wanted.
- **Overwrite?** — if the destination already holds a copy, both are shown side by side (marked `← copying this one` / `← will be replaced`) before you confirm.
- **The cost note** — the prompt cache is per-account, so the first turn after the move re-sends the whole transcript. On a large conversation that is slow and not cheap. Worth knowing before, not after.

`--force` skips the confirmations for scripting.

### `run --resume` checks for you

You don't have to remember any of this up front. When `claude-acc run <account> --resume <id>` names a session that account doesn't have, it says so before starting claude — which would otherwise just report an unknown session, with no hint that the transcript is sitting one account over:

```
$ claude-acc run work --resume 0266a566-0336-4055-8f05-c553d368528e

Session 0266a566-0336-4055-8f05-c553d368528e isn't in account 'work', but another account has it:
  default       15h ago       60 KB

Note: the prompt cache is per-account, so the first message after resuming
under another account re-sends the whole transcript — slower and more expensive
than a normal turn.
Copy it from 'default' into 'work' and resume? [y/N]
```

Answer `n` and claude starts anyway, exactly as before — it reports the unknown session itself.

When the id exists **both here and in another account**, those are two conversations that have drifted apart, and only you know which one you meant. So you get the copies as a numbered pick, with the current account's own copy among them:

```
Session 0266a566-0336-4055-8f05-c553d368528e exists in more than one account. Which copy do you want to resume?
  [1]  default       25m ago       60 KB
  [2]  work          15h ago       60 KB  ← this account, newest

Number (Enter to cancel):
```

Picking this account's copy (or pressing Enter) leaves everything alone. Picking another copies it in first.

Anything else is claude's ordinary behaviour, untouched: an id no other account has, and a bare `--resume` with no id — that opens claude's own session picker, and getting in front of it would only be in the way.

### …and by name, not only by id

`claude` gives every running session a name — derived from its directory, or set with `--name` — and `--resume` takes one in place of an id. Names work here too, across accounts, exactly the same way:

```
$ claude-acc run default --resume notes-api-3f

Session notes-api-3f isn't in account 'default', but another account has it:
  work          15m ago       5.1 MB

Note: the prompt cache is per-account, so the first message after resuming
under another account re-sends the whole transcript — slower and more expensive
than a normal turn.
Copy it from 'work' into 'default' and resume? [y/N]
```

`claude-acc session copy notes-api-3f --to default` takes a name too.

Two things worth knowing. A uuid always wins: if a live session took a name that happens to equal some transcript's id, the id is what resolves. And **a name only exists while its session runs** — claude keeps it in `<config-dir>/sessions/<pid>.json` and drops the entry when the process exits, recording it nowhere else. For a session that has already finished, use its id from `claude-acc sessions --all`.

### The same check for plain `claude --resume`

`claude` on your PATH is this tool's wrapper (see [IDE integration](ide.md)), so the check doesn't have to be limited to `claude-acc run`. With the hook on — the default — a plain `claude --resume <id>` gets exactly the prompts above:

```
claude-acc resume-hook          # show the current state
claude-acc resume-hook off      # plain `claude --resume` goes straight through
claude-acc resume-hook on
```

The setting lives in `~/.claude-switch/config`. `CLAUDE_ACC_NO_RESUME_HOOK=1` turns it off for a single shell without changing the stored value. `claude-acc run <account> --resume <id>` checks either way — the hook only governs the bare `claude` path.

The wrapper is careful about staying out of the way:

- it does nothing unless `--resume` is actually among the arguments, so an ordinary launch pays nothing;
- it does nothing without a terminal on both stdin and stdout, so scripts, pipes and CI are never prompted at;
- whatever happens, claude still starts — a failure in the check is never a failure to launch.

**macOS and Linux only.** The hook lives in the wrapper script, and there is no wrapper on Windows — PATH-based interception there would need a `.cmd`/`.exe` shim. `claude-acc run <account> --resume <id>` does the same check on every platform.

`claude-acc update` refreshes the wrapper for you; `claude-acc install` does too, if you ever need to force it.
