#!/usr/bin/env zsh
#
# The zsh completion survives `setopt nounset`.
#
# `_claude_acc_accounts` took its "include default" switch as `$1`, and the
# two callers that want only managed accounts — `remove` and
# `clone-settings` — pass nothing. Under `setopt nounset`, which people do
# set in .zshrc, reading an unset `$1` aborts the function: completion offers
# nothing and `_claude_acc_accounts:5: 1: parameter not set` lands over the
# prompt.
#
# Nothing in this repo sourced `shell/init.zsh` before, so this file is also
# the harness for it. The approach is the one `.claude/rules/cli-completions.md`
# prescribes: stub zsh's completion primitives, set `words` and `CURRENT`,
# and call the function directly.
#
# Run: zsh tests/shell/completion_nounset.zsh

emulate -L zsh
set -u
setopt nounset

typeset -i failures=0
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT

# The completion asks the binary for account names. A stub answers, so the
# test needs neither a build nor a ~/.claude-switch.
stub="$scratch/claude-acc"
cat > "$stub" <<'STUB'
#!/bin/sh
# `completions accounts` is the only thing the completion calls.
[ "$1" = "completions" ] && [ "$2" = "accounts" ] && printf 'work\nwork-2\n'
STUB
chmod +x "$stub"

sed "s|__CLAUDE_ACC_BIN__|$stub|g" "${0:A:h:h:h}/shell/init.zsh" > "$scratch/init.zsh"

# zsh's completion system is not loaded here, so everything the script calls
# into has to stand in. `_describe` is where candidates come out.
typeset -ga OFFERED=()
_describe() { shift; local arr="$1"; OFFERED+=("${(@P)arr}") }
_files() { OFFERED+=("<files>") }
compdef() { : }
add-zsh-hook() { : }

source "$scratch/init.zsh" >/dev/null 2>&1

check() {
    local label="$1" expected="$2" actual="$3"
    if [[ "$expected" == "$actual" ]]; then
        print -r -- "  ok   $label"
    else
        print -r -- "  FAIL $label: expected '$expected', got '$actual'"
        (( failures++ ))
    fi
}

# The candidate names for one completion point, comma-joined. Descriptions
# are cut at the colon, so a `name:description` pair compares as the name.
offered() {
    OFFERED=()
    words=("$@")
    CURRENT=${#words}
    _claude_acc_completion 2>/dev/null
    print -r -- "${(j:,:)${(@)OFFERED%%:*}}"
}

print -r -- "under setopt nounset:"

# The two call sites that pass no argument. These are what aborted.
check "remove offers the managed accounts" "work,work-2" \
    "$(offered claude-acc remove '')"
check "clone-settings offers the managed accounts" "work,work-2" \
    "$(offered claude-acc clone-settings '')"

# And the ones that pass something, which were never affected — here so a
# fix that broke them could not pass.
check "login offers default too" "default,work,work-2" \
    "$(offered claude-acc login '')"
check "--to offers default too" "default,work,work-2" \
    "$(offered claude-acc session copy x --to '')"

# Nothing should be written to the terminal while completing. Not through
# `offered`, which swallows stderr — this check would then have passed on
# the broken version too, as it did in the first draft. (The `remove` and
# `clone-settings` checks fail there either way; it was this one that was
# empty.)
OFFERED=()
words=(claude-acc remove '')
CURRENT=3
noise=$(_claude_acc_completion 2>&1 >/dev/null)
check "and says nothing on stderr" "" "$noise"

print -r -- ""
if (( failures )); then
    print -r -- "$failures check(s) failed"
    exit 1
fi
print -r -- "all checks passed"
