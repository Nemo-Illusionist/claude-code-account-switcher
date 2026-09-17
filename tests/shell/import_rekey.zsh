#!/usr/bin/env zsh
#
# `import` has to carry the standard ~/.claude account's token to the new
# account directory.
#
# Claude Code keys the Keychain entry by the absolute config-dir path, so
# `_claude_acc_rekey_keychain` copies it to the imported location. It looked
# for the scoped name only — and the standard account, running with no
# CLAUDE_CONFIG_DIR, keeps its token under the bare "Claude Code-credentials"
# instead. Importing ~/.claude therefore produced an account whose token was
# left behind, and the user was told to log in again.
#
# Run: zsh tests/shell/import_rekey.zsh

emulate -L zsh
set -u

typeset -i failures=0
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT

# Sourcing the script writes into ~/.claude-switch/; keep the real one out.
export HOME="$scratch/home"
mkdir -p "$HOME/.claude"

source "${0:A:h:h:h}/claude-switch.sh" >/dev/null 2>&1

standard="$HOME/.claude"
managed="$scratch/hand-rolled-account"
target="$CLAUDE_SWITCH_ACCOUNTS_DIR/imported"
mkdir -p "$managed"

scoped() { print -r -- "Claude Code-credentials-$(printf '%s' "$1" | shasum -a 256 | cut -c1-8)" }

# The rekey is macOS-only; CI also runs this on Linux.
uname() { print -r -- 'Darwin' }

# Stand in for the Keychain. `security add-generic-password -U` writes,
# `find-generic-password -w` reads.
typeset -gA keychain
security() {
    local action="$1" service="" value="" want_write=0
    shift
    [[ "$action" == "add-generic-password" ]] && want_write=1
    while (( $# )); do
        case "$1" in
            -s) service="$2"; shift ;;
            -w) (( want_write )) && { value="$2"; shift } ;;
        esac
        shift
    done
    if (( want_write )); then
        keychain[$service]="$value"
        return 0
    fi
    [[ -n "${keychain[$service]:-}" ]] || return 1
    print -r -- "${keychain[$service]}"
}

check() {
    local name="$1" want="$2" got="$3"
    if [[ "$got" == "$want" ]]; then
        print -r -- "ok   $name"
    else
        print -r -- "FAIL $name: want '$want', got '$got'"
        (( failures++ ))
    fi
}

# 1. Importing ~/.claude, whose token sits under the bare service only.
keychain=( "Claude Code-credentials" 'blob-standard' )
_claude_acc_rekey_keychain "$standard" "$target"
check 'importing the standard account carries its token over' \
    'blob-standard' "${keychain[$(scoped "$target")]:-}"

# 2. A hand-rolled dir with no entry of its own must not pick up the bare one:
#    it belongs to whoever logged in last, and would import a stranger.
keychain=( "Claude Code-credentials" 'blob-standard' )
_claude_acc_rekey_keychain "$managed" "$target"
check 'a managed source never takes the bare entry' \
    '' "${keychain[$(scoped "$target")]:-}"

# 3. The ordinary case, unchanged: a source with its own scoped entry.
keychain=( "$(scoped "$managed")" 'blob-managed' )
_claude_acc_rekey_keychain "$managed" "$target"
check 'a scoped source entry is copied as before' \
    'blob-managed' "${keychain[$(scoped "$target")]:-}"

# 4. When the standard account has both, its own entry is the one that travels.
keychain=(
    "Claude Code-credentials" 'blob-standard'
    "$(scoped "$standard")"   'blob-scoped'
)
_claude_acc_rekey_keychain "$standard" "$target"
check 'the scoped entry wins over the bare one' \
    'blob-scoped' "${keychain[$(scoped "$target")]:-}"

# 5. Nothing anywhere: report failure so `import` can tell the user to log in.
keychain=()
if _claude_acc_rekey_keychain "$managed" "$target"; then
    print -r -- 'FAIL an empty keychain should report failure'
    (( failures++ ))
else
    print -r -- 'ok   an empty keychain reports failure'
fi

if (( failures )); then
    print -r -- "$failures test(s) failed"
    exit 1
fi
print -r -- 'all cases passed'
