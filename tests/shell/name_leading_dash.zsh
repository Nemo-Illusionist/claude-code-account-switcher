#!/usr/bin/env zsh
#
# Account names may not start with a hyphen, in the script as in the binary.
#
# The binary's argument parser reads `-work` as a flag, so a name the old
# rule accepted could be created but not addressed; and the script's own
# `add` and `remove` take `-s` and `-f` as flags. Both sides now refuse such
# a name for a new account, and keep one that already exists usable.
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

# An account made before the rule stays usable while its directory is there;
# a new one still cannot be given such a name.
fail() { print -r -- "FAIL: $1"; failures+=1 }
_claude_validate_existing_name -work >/dev/null 2>&1 && fail "-work accepted with no directory"
mkdir -p "$CLAUDE_SWITCH_ACCOUNTS_DIR/-work"
_claude_validate_existing_name -work >/dev/null 2>&1 || fail "existing -work refused"
_claude_validate_name -work >/dev/null 2>&1 && fail "-work accepted for a new account"
# `--` before the name, as the binary suggests: lock reaches the account
# (it has no identity yet) rather than reporting it missing.
out=$(_claude_acc_lock -- -work 2>&1)
[[ "$out" == *"no signed-in identity"* ]] || fail "lock -- -work said: $out"
_claude_acc_remove -f -work >/dev/null 2>&1
[[ ! -e "$CLAUDE_SWITCH_ACCOUNTS_DIR/-work" ]] || fail "remove -f -work left the account"

# Desktop profiles: the same rule, under their own directory. `keep` is the
# control.
mkdir -p "$CLAUDE_SWITCH_DESKTOP_DIR/-work" "$CLAUDE_SWITCH_DESKTOP_DIR/keep"
_claude_acc_desktop_name_ok -work >/dev/null 2>&1 || fail "existing desktop profile -work refused"
_claude_acc_desktop_name_ok -nope >/dev/null 2>&1 && fail "desktop -nope accepted with no directory"
_claude_acc_desktop_remove -f -work >/dev/null 2>&1
[[ ! -e "$CLAUDE_SWITCH_DESKTOP_DIR/-work" ]] || fail "desktop remove -f -work left the profile"
[[ -d "$CLAUDE_SWITCH_DESKTOP_DIR/keep" ]] || fail "desktop remove touched keep"

if (( failures )); then
    print -r -- "name_leading_dash: $failures failure(s)"
    exit 1
fi
print -r -- "name_leading_dash: ok"
