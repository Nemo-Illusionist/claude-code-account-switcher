#!/usr/bin/env zsh
#
# Nothing in the script may reference a parameter it does not own without a
# default.
#
# `setopt nounset` is not unusual in a careful .zshrc, and under it an unset
# parameter is a fatal error rather than an empty string. `_claude_acc_lang`
# read $CLAUDE_ACC_LANG and $LANG bare, and it backs `_msg` — so every
# user-facing string the script printed failed for those users (#137). The
# same shape appears wherever a command reads "$1" before testing whether it
# was given one.
#
# Two things are checked, because the error is silent in different ways:
#
#   - no command prints "parameter not set", with CLAUDE_ACC_LANG and LANG
#     both unset and `set -u` on
#   - the usage messages are byte-for-byte what they were, so a guard that
#     "fixed" the error by changing what the command does would still fail
#
# Run: zsh tests/shell/nounset.zsh

set -u

typeset -i failures=0
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT

export HOME="$scratch/home"
mkdir -p "$HOME/.claude"

# Deliberately NOT exported: this test exists because they can be absent.
unset CLAUDE_ACC_LANG LANG 2>/dev/null

source "${0:A:h:h:h}/claude-switch.sh" >/dev/null 2>&1

# `update` is absent from every list below on purpose. It downloads master's
# copy of the script over the live one, so running it here overwrote the very
# fix this test was written to pin.
typeset -a no_arg_commands=(
    list login remove default reset lock link unlink links status usage
    run doctor whoami clone-settings import desktop help
)

# Flags with no operand after them: the parsing loop consumes the flag and
# then looks at "$1" again, which is where the parameter has gone.
typeset -a flag_only=(
    'remove -f'
    'remove --purge'
    'remove --purge -f'
    'doctor --json'
    'lock -f'
    'desktop clone-config --from'
    'desktop clone-runtime --from'
)

probe() {
    local out
    out=$(claude-acc "$@" 2>&1)
    if [[ "$out" == *"parameter not set"* ]]; then
        print -r -- "  FAIL claude-acc $*: ${out%%$'\n'*}"
        (( failures++ ))
    else
        print -r -- "  ok   claude-acc $*"
    fi
}

print -r -- "no command references an unset parameter:"
for c in $no_arg_commands; do
    probe "$c"
done
for args in $flag_only; do
    probe ${=args}
done

print -r -- ""
print -r -- "_claude_acc_lang with neither variable set:"

lang=$(_claude_acc_lang)
if [[ "$lang" == "en" ]]; then
    print -r -- "  ok   falls back to en"
else
    print -r -- "  FAIL falls back to en: got '$lang'"
    (( failures++ ))
fi

# The override must still win, and $LANG must still be read — a guard that
# defaulted the variables into never being consulted would pass the checks
# above and break both languages.
lang=$(CLAUDE_ACC_LANG=ru _claude_acc_lang)
if [[ "$lang" == "ru" ]]; then
    print -r -- "  ok   CLAUDE_ACC_LANG still overrides"
else
    print -r -- "  FAIL CLAUDE_ACC_LANG still overrides: got '$lang'"
    (( failures++ ))
fi

lang=$(LANG=ru_RU.UTF-8 _claude_acc_lang)
if [[ "$lang" == "ru" ]]; then
    print -r -- "  ok   LANG is still detected"
else
    print -r -- "  FAIL LANG is still detected: got '$lang'"
    (( failures++ ))
fi

print -r -- ""
print -r -- "a missing operand reaches the usage message:"

# `desktop clone-config --from` used to be worse than a nounset error. With
# the value missing, `shift 2` could not shift at all, so the surrounding
# `while (( $# > 0 ))` ran forever and the shell died of heap exhaustion —
# on master, with or without nounset. A guard alone would have turned the
# abort into that hang, so the two belong together.
export CLAUDE_ACC_LANG=en

check_usage() {
    local label="$1" expected="$2"; shift 2
    local actual
    actual=$(claude-acc "$@" 2>&1 | head -1)
    if [[ "$actual" == "$expected" ]]; then
        print -r -- "  ok   $label"
    else
        print -r -- "  FAIL $label: expected '$expected', got '$actual'"
        (( failures++ ))
    fi
}

check_usage "login"          "Usage: claude-acc login <name>" login
check_usage "remove"         "Usage: claude-acc remove [-f] [--purge] <name>" remove
check_usage "link"           "Usage: claude-acc link <name>" link
check_usage "run"            "Usage: claude-acc run <name> [args...]" run
check_usage "clone-settings" "Usage: claude-acc clone-settings <name>" clone-settings
check_usage "import"         "Usage: claude-acc import <name> <path> [--move]" import
check_usage "desktop" \
    "Usage: claude-acc desktop add|clone-config|clone-runtime|list|run|remove [<name>]" \
    desktop
check_usage "desktop clone-config --from" \
    "Usage: claude-acc desktop add|clone-config|clone-runtime|list|run|remove [<name>]" \
    desktop clone-config --from

print -r -- ""
if (( failures )); then
    print -r -- "$failures check(s) failed"
    exit 1
fi
print -r -- "all checks passed"
