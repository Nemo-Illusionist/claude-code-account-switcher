#!/usr/bin/env zsh
#
# The shell integration survives strict mode.
#
# `set -u` / `setopt nounset` in someone's rc file is the case here. Two
# halves, and they are not equally serious:
#
# **bash, and real.** `__claude_acc_prompt_hook` read `$__CLAUDE_ACC_PREV_DIR`
# before it was ever set, so the very first prompt in every new shell printed
# `__CLAUDE_ACC_PREV_DIR: unbound variable`; and `_claude_acc_complete` read
# `${COMP_WORDS[2]}`, which is out of range while completing the first word —
# `claude-acc <TAB>`, the most ordinary completion there is — so that died
# with `COMP_WORDS[2]: unbound variable`. Bash does not relax `set -u` for
# completions. Both reproduced before the fix, from a `set -u` shell.
#
# **zsh, and defensive.** `_claude_acc_accounts` read a bare `$1` while two
# call sites pass nothing. A real completion never runs under `nounset`:
# `_main_complete` evals `_comp_setup`, which sets the options in
# `_comp_options`, and that list contains `unset` — `nounset`'s inverse. So
# the zsh checks below drive the function directly, which is the only way to
# reach it, and they pin the guard rather than a bug anybody met. The first
# version of this file claimed otherwise; the claim did not survive being
# checked.
#
# Nothing in this repo sourced `shell/init.zsh` or `shell/init.bash` before,
# so this file is also the harness for both. The zsh half follows the recipe
# in `.claude/rules/cli-completions.md` — stub the completion primitives, set
# `words`/`CURRENT`, call the function — and does *not* eval `_comp_setup`,
# which is exactly why it can see the guard at all.
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
print -r -- "bash, under set -u:"

# Driven as a separate bash process: these are bash bugs, and `set -u` has
# to be in effect while the file is sourced and while the hook runs.
bash_init="$scratch/init.bash"
sed "s|__CLAUDE_ACC_BIN__|$stub|g" "${0:A:h:h:h}/shell/init.bash" > "$bash_init"

bash_probe() {
    # $1 is bash code run after sourcing the integration under `set -u`.
    bash --noprofile --norc -c "
        set -u
        source '$bash_init' 2>&1
        $1
    " 2>&1
}

out=$(bash_probe 'eval "$PROMPT_COMMAND"; echo RC=$?')
check "the first prompt hook says nothing" "0" \
    "$(print -r -- "$out" | grep -c 'unbound variable')"
check "and it succeeds" "1" "$(print -r -- "$out" | grep -c 'RC=0')"

out=$(bash_probe 'COMP_WORDS=(claude-acc ""); COMP_CWORD=1; COMPREPLY=()
    _claude_acc_complete; echo "CANDIDATES=${COMPREPLY[*]-}"')
check "completing the first word says nothing" "0" \
    "$(print -r -- "$out" | grep -c 'unbound variable')"
check "and offers the commands" "1" \
    "$(print -r -- "$out" | grep -c 'CANDIDATES=.*clone-settings')"

out=$(bash_probe 'COMP_WORDS=(claude-acc remove ""); COMP_CWORD=2; COMPREPLY=()
    _claude_acc_complete; echo "CANDIDATES=${COMPREPLY[*]-}"')
check "completing an account says nothing" "0" \
    "$(print -r -- "$out" | grep -c 'unbound variable')"
check "and offers the accounts" "CANDIDATES=work work-2" \
    "$(print -r -- "$out" | grep '^CANDIDATES=')"

print -r -- ""
if (( failures )); then
    print -r -- "$failures check(s) failed"
    exit 1
fi
print -r -- "all checks passed"
