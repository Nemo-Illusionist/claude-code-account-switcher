#!/usr/bin/env zsh
#
# `desktop run` must consult the sign-in guard only for a profile that is
# *not* signed in — README: "a profile that already is opens freely".
#
# This is the half of #118 that hurt: the predicate read every signed-in
# profile as signed out, so with any Claude Desktop open `desktop run`
# entered the guard and refused to launch. Testing the predicate alone would
# not have caught that the two are wired together, so this drives the
# command and watches which branch it takes.
#
# Run: zsh tests/shell/desktop_run_guard.zsh

emulate -L zsh
set -u

source "${0:A:h:h:h}/claude-switch.sh" >/dev/null 2>&1

typeset -i failures=0
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT

# Point the script at a scratch profile tree, and replace everything past
# the decision: the guard reports that it ran and refuses, the launcher
# reports that it launched. Neither touches the real machine.
CLAUDE_SWITCH_DESKTOP_DIR="$scratch/desktop"
_claude_acc_desktop_signin_possible() { print -r -- 'GUARD-RAN'; return 1 }
_claude_acc_desktop_app() { print -r -- "$scratch/Claude.app" }
_claude_acc_desktop_launch() { print -r -- 'LAUNCHED' }

profile() {
    local name="$1" body="$2"
    mkdir -p "$CLAUDE_SWITCH_DESKTOP_DIR/$name"
    [[ "$body" == ABSENT ]] ||
        print -r -- "$body" >"$CLAUDE_SWITCH_DESKTOP_DIR/$name/config.json"
}

# check <case name> <profile name> <LAUNCHED|GUARD-RAN>
check() {
    local name="$1" prof="$2" want="$3" out
    out=$(_claude_acc_desktop_run "$prof" 2>&1)
    if [[ "$out" == *"$want"* ]]; then
        print -r -- "ok   $name"
    else
        print -r -- "FAIL $name — expected $want in output, got:"
        print -r -- "${out//$'\n'/\\n}"
        (( failures++ ))
    fi
}

# Pretty-printed exactly as Claude Desktop writes it — the shape #118 read
# as signed out.
profile signed-in '{
    "oauth:tokenCacheV2": "djEwkwgMDd4K"
}'
profile signed-out '{"locale": "en-US"}'

check 'a signed-in profile opens without the sign-in guard' signed-in LAUNCHED
check 'a signed-out profile still meets the guard' signed-out GUARD-RAN

if (( failures )); then
    print -r -- "$failures failing case(s)"
    exit 1
fi
print -r -- "all cases passed"
