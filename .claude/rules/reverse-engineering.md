# Claims about Claude Code's internals get verified, every time

This tool exists by reading things Anthropic never documented: the Keychain
service name, the `claudeInChromeDefaultEnabled` flag, the statusline JSON,
`cachedUsageUtilization`, `CLAUDE_SECURESTORAGE_CONFIG_DIR`. None of it is a
contract. All of it is one build's behaviour, observed once.

That makes confident recall the main way this repo gets things wrong. Not
carelessness — the claims *sound* right, they were right about some build, and
nothing locally contradicts them. Every failure cited below shipped, or nearly
shipped, from exactly that.

## Re-read the build. Do not recall it

Before writing a claim about Claude Code into code, a comment, a doc, a PR
description or an issue, read it out of the binary now:

```sh
B=$(readlink -f ~/.local/bin/claude)   # the build actually in use
file "$B"                              # must say Mach-O / ELF
strings -a "$B" | tr ';' '\n' > /tmp/cc-dump.txt
grep -F 'CLAUDE_SECURESTORAGE_CONFIG_DIR' /tmp/cc-dump.txt
```

**Not `command -v claude`.** With this tool installed that resolves to *our*
wrapper, a POSIX shell script, which contains none of what you are looking
for. The `file` check is there to catch it: if the answer is not a Mach-O or
ELF binary, you are grepping the wrong thing.

**Resolve the symlink rather than typing a version.** Claude Code updates
itself every few days and prunes old builds — `versions/2.1.288`, which #150
was written against, was gone ten days later. A hardcoded path then matches
nothing, and that reads exactly like the behaviour having changed.

**Dump to a file first.** The binary is ~250 MB and holds single lines
hundreds of kilobytes long; splitting on `;` is what makes any of it legible,
and you will be searching it repeatedly.

**Say which version you read.** The `keychain_service` doc comment does
("Transcribed from Claude Code 2.1.288"), so the next reader can tell a stale
observation from a current one instead of re-deriving it. The module header
above it does not — it credits an internal `dV()` with no version attached,
which is now unresolvable. Don't add more of those.

What this buys: the statusline payload was twice described as having moved to
`limits[]` with a server-side `severity`, in a PR description and then again
after being challenged. The binary built `{five_hour, seven_day, spend_limit}`
the whole time; `limits[]` belongs to the Agent SDK, a different surface.

## A minified name is not an identifier — search for the shape

Names are useless as handles here, for two separate reasons.

They are **reused** across chunks. In 2.1.300 `ve` is the config-dir resolver
— and the name of 75 other things, with at least eight unrelated bodies
(`if(!Dr())return`, `G(!0)`, a PowerShell `Get-ItemProperty`, …). Grepping
`function ve(` and reading the first hit answers a question nobody asked.

They also **churn between builds**: the service-name function and the resolver
are `A$`/`we` in 2.1.288 and `l1`/`ve` in 2.1.300. So never record a minified
name in a comment — it is wrong by the next release.

Search instead for what does not change: an environment variable, a string
literal, a distinctive operator sequence.

```sh
grep -F 'CLAUDE_SECURESTORAGE_CONFIG_DIR' /tmp/cc-dump.txt | grep -F 'sha256'
```

In #150 getting this wrong was the difference between "the config dir is
hashed raw" and the truth, which is that the resolver normalises itself. The
`.normalize("NFC")` sitting next to the hash covers only the env-var branch,
so the closest evidence pointed the wrong way; the rest was one chunk away,
behind an `import`.

## Before concluding something is absent, prove the search works

An empty result is ambiguous between "not there" and "the search was broken",
and the second is more likely than it feels.

A pattern containing quotes is the usual culprit, and it belongs in a file
rather than inline: `grep -cF 'function l1(e=\"\")'` matches nothing, because
inside single quotes `\"` is a literal backslash-quote. That produced three
false negatives in a single sitting, each looking like the code had changed.

So grep for a string known to be present and confirm a non-zero count before
believing a zero one.

## Transcribed code carries its caller's context

When copying a derivation out of Claude Code, ask **who is running it**.
Claude Code reads `process.env.CLAUDE_CONFIG_DIR` because it *is* the account.
This tool asks what the name would be for a directory it was handed — a
different question with a different answer.

Transcribing that line literally, as #150's first draft did, would have sent
every scoped Keychain lookup to the legacy name whenever `claude-acc` ran from
a plain shell, which is the normal case. Where a transcription departs from
the original on purpose, say so in a comment at that line; otherwise someone
will "fix" it back.

## Do not relay a claim nobody has reproduced

A subagent's report, an issue comment, a changelog line and your own memory
are all second-hand. Reproduce the behaviour, or label it unverified — never
state it as fact.

A reported `PATH` shadowing (`~/.local/bin` taking precedence over our
wrapper) was passed on and then could not be reproduced in an interactive
shell or under `zsh -l`. Retracting cost more than checking would have.

## Push-back is a reason to verify, not to restate

When someone disagrees with a factual claim — the user, a reviewer, a
subagent with evidence — the next move is to go and look, not to repeat it
more firmly. Both wrong assertions about the statusline payload happened
because the second one skipped that step.

## Anchors and links resolve against the target

A heading's anchor follows rules that are easy to get wrong for anything
non-ASCII. `#сколько-осталось-от-лимита-usage` was constructed and plausible;
the real heading yields `#сколько-лимита-осталось-usage`.

Open the target file, read the heading, build the anchor from what is there —
and the same for every relative path in a doc change.

## The short version

Anything in this repo that touches Claude Code is an observation with a
version attached. Treat a remembered observation as a hypothesis, and the
binary in front of you as the only evidence.
