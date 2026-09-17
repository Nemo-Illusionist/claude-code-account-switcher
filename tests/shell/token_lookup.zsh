#!/usr/bin/env zsh
#
# `_claude_acc_token` has to find the standard ~/.claude account's token.
#
# Claude Code scopes the Keychain entry per config dir — "Claude Code-
# credentials-<sha256(dir)[0:8]>" — but the standard account runs with no
# CLAUDE_CONFIG_DIR and keeps the bare, unscoped "Claude Code-credentials"
# instead. The script only ever looked for the scoped name, so `doctor`,
# `list`, `usage` and `status` reported one account fewer than the machine
# had (#114), while the Rust binary listed it.
#
# Run: zsh tests/shell/token_lookup.zsh

emulate -L zsh
set -u

typeset -i failures=0
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT

# Sourcing the script runs its init, which writes into ~/.claude-switch/.
# A scratch HOME keeps the real installation out of it — and doubles as the
# standard account's directory for these cases.
export HOME="$scratch/home"
mkdir -p "$HOME/.claude"

source "${0:A:h:h:h}/claude-switch.sh" >/dev/null 2>&1

standard="$HOME/.claude"
managed="$CLAUDE_SWITCH_ACCOUNTS_DIR/work"
mkdir -p "$managed"

blob() { print -r -- "{\"claudeAiOauth\":{\"accessToken\":\"$1\"}}" }
scoped_service() {
    print -r -- "Claude Code-credentials-$(printf '%s' "$1" | shasum -a 256 | cut -c1-8)"
}

# Stand in for the Keychain: $keychain maps service name → stored blob.
typeset -gA keychain
security() {
    local service=""
    while (( $# )); do
        [[ "$1" == "-s" ]] && { service="$2"; shift }
        shift
    done
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

# 1. The standard account, stored only under the bare legacy service.
# The bare service name is Claude Code's, not ours — spelled out here so
# this test fails on the behaviour if the script stops looking for it.
keychain=( "Claude Code-credentials" "$(blob tok-legacy)" )
check 'standard account falls back to the unscoped service' \
    'tok-legacy' "$(_claude_acc_token "$standard")"

# 2. A managed account must NOT: the bare entry belongs to whoever logged in
#    last, so attributing it here would report another identity as this one.
check 'managed account does not touch the unscoped service' \
    '' "$(_claude_acc_token "$managed")"

# 3. Its own scoped entry still wins for the standard account.
keychain=(
    "Claude Code-credentials"       "$(blob tok-legacy)"
    "$(scoped_service "$standard")"       "$(blob tok-scoped)"
)
check 'a scoped entry takes precedence over the legacy one' \
    'tok-scoped' "$(_claude_acc_token "$standard")"

# 4. And the managed account reads its own scoped entry, as before.
keychain=( "$(scoped_service "$managed")" "$(blob tok-work)" )
check 'managed account reads its own scoped entry' \
    'tok-work' "$(_claude_acc_token "$managed")"

# 5. With nothing in the Keychain, the plaintext file is still the fallback.
keychain=()
blob tok-plaintext > "$managed/.credentials.json"
check 'plaintext .credentials.json remains the last resort' \
    'tok-plaintext' "$(_claude_acc_token "$managed")"

if (( failures )); then
    print -r -- "$failures test(s) failed"
    exit 1
fi
print -r -- 'all cases passed'
