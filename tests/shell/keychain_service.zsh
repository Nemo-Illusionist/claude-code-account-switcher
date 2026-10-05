#!/usr/bin/env zsh
#
# The Keychain service name has to be the one Claude Code actually writes.
#
# Claude Code derives it as
# "Claude Code-credentials-<sha256(nfc(dir))[0:8]>", and two rules in that
# expression were missing here (#147):
#
#   - CLAUDE_SECURESTORAGE_CONFIG_DIR relocates credential storage
#     independently of CLAUDE_CONFIG_DIR. When set, *it* is what gets hashed,
#     so ignoring it meant reading a service name nothing ever writes to.
#     Set to the empty string it drops the suffix altogether and credentials
#     go to the bare legacy name instead.
#   - the path is NFC-normalised first, so a decomposed and a composed
#     spelling of one directory gave two different names, only one of which
#     was Claude Code's.
#
# The expected hashes are from Node — the engine Claude Code runs on — not
# from this script, so an error in our own derivation cannot make its own
# test agree with it:
#
#   crypto.createHash("sha256").update(p).digest("hex").substring(0, 8)
#
# Run: zsh tests/shell/keychain_service.zsh

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
    local label="$1" expected="$2" actual="$3"
    if [[ "$expected" == "$actual" ]]; then
        print -r -- "  ok   $label"
    else
        print -r -- "  FAIL $label: expected '$expected', got '$actual'"
        (( failures++ ))
    fi
}

# Reference values, computed in Node (see the header).
ascii_dir="/Users/x/.claude-switch/accounts/work"
ascii_hash="6669c001"
nfc_dir=$'/Users/José/.claude'
nfd_dir=$'/Users/José/.claude'
accented_hash="aa0dcdd9"

print -r -- "_claude_acc_keychain_service, the ordinary case:"

check "an ASCII path hashes as Claude Code hashes it" \
    "Claude Code-credentials-$ascii_hash" \
    "$(_claude_acc_keychain_service "$ascii_dir")"

print -r -- ""
print -r -- "CLAUDE_SECURESTORAGE_CONFIG_DIR:"

# The override decides the hash, not the directory we were handed.
check "an override is what gets hashed" \
    "Claude Code-credentials-$ascii_hash" \
    "$(CLAUDE_SECURESTORAGE_CONFIG_DIR="$ascii_dir" \
        _claude_acc_keychain_service /somewhere/else)"

# Set-but-empty is a distinct case upstream, not a missing value.
check "an empty override asks for the unsuffixed name" \
    "Claude Code-credentials" \
    "$(CLAUDE_SECURESTORAGE_CONFIG_DIR= _claude_acc_keychain_service /anything)"

# An unset variable must change nothing — this is the path everyone is on.
check "an unset override leaves the derivation alone" \
    "$(_claude_acc_keychain_service "$ascii_dir")" \
    "Claude Code-credentials-$ascii_hash"

print -r -- ""
print -r -- "NFC normalisation:"

# Guard the premise: if these two were equal as byte strings the rest of this
# section would pass without proving anything.
if [[ "$nfc_dir" == "$nfd_dir" ]]; then
    print -r -- "  FAIL the two spellings are indistinguishable in this shell"
    (( failures++ ))
else
    print -r -- "  ok   the two spellings differ as byte strings"
fi

if perl -MUnicode::Normalize -e '' 2>/dev/null; then
    check "a composed path hashes as Claude Code hashes it" \
        "Claude Code-credentials-$accented_hash" \
        "$(_claude_acc_keychain_service "$nfc_dir")"

    check "a decomposed path gives the same name" \
        "Claude Code-credentials-$accented_hash" \
        "$(_claude_acc_keychain_service "$nfd_dir")"

    # Normalising by stripping the accent would collide with a genuinely
    # different directory.
    # Checked for a real name as well as a different one: comparing only on
    # inequality passes when the derivation returns nothing at all, which is
    # exactly what it does before this fix.
    plain="$(_claude_acc_keychain_service /Users/Jose/.claude)"
    if [[ "$plain" == "Claude Code-credentials-"?* \
       && "$plain" != "Claude Code-credentials-$accented_hash" ]]; then
        print -r -- "  ok   an unaccented path is still a different account"
    else
        print -r -- "  FAIL an unaccented path: got '$plain'"
        (( failures++ ))
    fi

    # The decode/encode round trip is the part that is easy to get wrong:
    # perl receives bytes, so without an explicit decode it treats UTF-8 as
    # Latin-1, NFC does nothing, and the output is double-encoded — giving a
    # hash that matches neither spelling.
    check "the helper round-trips UTF-8 rather than double-encoding it" \
        "$nfc_dir" "$(_claude_acc_nfc "$nfd_dir")"
else
    print -r -- "  skip perl/Unicode::Normalize unavailable; NFC cases not run"
fi

# ASCII must never need perl: it is already NFC, and starting a process for
# every token lookup would be a real cost on the common path.
check "ASCII is returned untouched" "$ascii_dir" "$(_claude_acc_nfc "$ascii_dir")"

print -r -- ""
if (( failures )); then
    print -r -- "$failures check(s) failed"
    exit 1
fi
print -r -- "all checks passed"
