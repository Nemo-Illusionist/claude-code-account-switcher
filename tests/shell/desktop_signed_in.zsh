#!/usr/bin/env zsh
#
# Behaviour tests for `_claude_acc_desktop_signed_in` in claude-switch.sh.
#
# `zsh -n` is the only thing CI ran over this file before, and syntax was
# never the problem: #118 was a pattern that matched nothing on any real
# profile, so `desktop list` called every signed-in profile signed out and
# `desktop run` refused to launch one while any Claude Desktop was open.
# The cases that bug turned on live here.
#
# Run: zsh tests/shell/desktop_signed_in.zsh

emulate -L zsh
set -u

typeset -i failures=0
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT

# Sourcing the script runs its init, which creates ~/.claude-switch/ and
# regenerates the `claude` wrapper inside it. On a machine running the Rust
# CLI that silently replaces its wrapper with the zsh one, so running the
# tests breaks the installation. A scratch HOME keeps the real one out of it.
export HOME="$scratch/home"
mkdir -p "$HOME"

source "${0:A:h:h:h}/claude-switch.sh" >/dev/null 2>&1

# check <case name> <config.json body, or the word ABSENT> <yes|no>
check() {
    local name="$1" body="$2" want="$3"
    local dir="$scratch/${name//[^a-zA-Z0-9]/_}"
    mkdir -p "$dir"
    [[ "$body" == ABSENT ]] || print -r -- "$body" >"$dir/config.json"

    local got=no
    _claude_acc_desktop_signed_in "$dir" && got=yes
    if [[ "$got" == "$want" ]]; then
        print -r -- "ok   $name"
    else
        print -r -- "FAIL $name — expected $want, got $got"
        (( failures++ ))
    fi
}

# The shape Claude Desktop actually writes: indented, with a space between
# the key and its value. This is #118 itself.
check 'pretty-printed V2 token' '{
    "locale": "en-US",
    "oauth:tokenCacheV2": "djEwkwgMDd4K"
}' yes

check 'compact V2 token' '{"oauth:tokenCacheV2":"djEwkwgMDd4K"}' yes

check 'pre-V2 token, pretty-printed' '{
    "oauth:tokenCache": "djEwkwgMDd4K"
}' yes

# The V1 entry survives the V2 migration as an empty string; the Rust side
# has the same case, and the two must not disagree about it.
check 'empty placeholder is not a token' '{
    "oauth:tokenCache": "",
    "locale": "en-US"
}' no

check 'null is not a token' '{"oauth:tokenCacheV2": null}' no
check 'no token key at all' '{"locale": "en-US"}' no
check 'no config.json' ABSENT no

if (( failures )); then
    print -r -- "$failures failing case(s)"
    exit 1
fi
print -r -- "all cases passed"
