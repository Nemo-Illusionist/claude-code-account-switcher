#!/usr/bin/env zsh
#
# Account names may not start with a hyphen, in the script as in the binary.
#
# The binary's argument parser reads `-work` as a flag, so a name the old
# rule accepted could be created but not addressed; and the script's own
# `add` and `remove` take `-s` and `-f` as flags. Both sides now refuse such
# a name.
#
# Run: zsh tests/shell/name_leading_dash.zsh

emulate -L zsh
set -u

typeset -i failures=0
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT

export HOME="$scratch/home"
mkdir -p "$HOME/.claude"
export CLAUDE_ACC_LANG=en
source "${0:A:h:h:h}/claude-switch.sh" >/dev/null 2>&1

check() {
    local name="$1" want="$2" got
    if _claude_validate_name "$name" >/dev/null 2>&1; then got=accepted; else got=refused; fi
    if [[ "$got" != "$want" ]]; then
        print -r -- "FAIL: '$name' was $got, expected $want"
        failures+=1
    fi
}

check -work refused
check - refused
check -- refused
check -s refused
check work accepted
check work- accepted
check _work accepted
check a accepted

# The refusal names the rule, rather than a generic error.
msg=$(_claude_validate_name -work 2>&1)
[[ "$msg" == *"must not start with a hyphen"* ]] || { print -r -- "FAIL: message was: $msg"; failures+=1; }

if (( failures )); then
    print -r -- "name_leading_dash: $failures failure(s)"
    exit 1
fi
print -r -- "name_leading_dash: ok"
