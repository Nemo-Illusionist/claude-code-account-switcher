#!/usr/bin/env zsh
#
# Claude in Chrome is switched on per config dir, and nothing said so.
#
# Claude Code wires the claude-in-chrome MCP server only when
# `claudeInChromeDefaultEnabled` is `true` in that config dir's own
# .claude.json. A new account starts without it, so the browser tools are
# simply absent there — which reads as "this machine has no extension"
# rather than "this account never said yes". Worse, a value that is present
# but not `true` (Claude Code leaves it at `null`) also takes the account off
# the auto-enable path, so it never offers to turn it on by itself.
#
# These pin the two readers `doctor` reports from.
#
# Run: zsh tests/shell/chrome_flag.zsh

emulate -L zsh
set -u

typeset -i failures=0
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT

# Sourcing the script runs its init, which writes into ~/.claude-switch/.
# A scratch HOME keeps the real installation out of it — and doubles as the
# standard account's home, whose .claude.json sits beside ~/.claude/.
export HOME="$scratch/home"
mkdir -p "$HOME/.claude"

source "${0:A:h:h:h}/claude-switch.sh" >/dev/null 2>&1

managed="$CLAUDE_SWITCH_ACCOUNTS_DIR/work"
mkdir -p "$managed"

check() {
    local label="$1" expected="$2" actual="$3"
    if [[ "$expected" == "$actual" ]]; then
        print -r -- "  ok   $label"
    else
        print -r -- "  FAIL $label: expected '$expected', got '$actual'"
        (( failures++ ))
    fi
}

# Run the reader and turn its exit status into a word, so a failure message
# says what was meant rather than a bare number.
enabled_state() {
    _claude_acc_chrome_enabled "$1"
    case $? in
        0) print -r -- "on" ;;
        1) print -r -- "off" ;;
        *) print -r -- "unknown" ;;
    esac
}

seen_state() {
    if _claude_acc_chrome_seen "$1"; then print -r -- "seen"; else print -r -- "unseen"; fi
}

print -r -- "_claude_acc_chrome_enabled:"

print -r -- '{"claudeInChromeDefaultEnabled": true}' > "$managed/.claude.json"
check "true reads as on" "on" "$(enabled_state "$managed/.claude.json")"

print -r -- '{"claudeInChromeDefaultEnabled": false}' > "$managed/.claude.json"
check "false reads as off" "off" "$(enabled_state "$managed/.claude.json")"

# The case this exists for: Claude Code leaves the key at `null` rather than
# removing it. `null` is not `true`, so the server is never wired — and not
# absent either, so auto-enable never fires.
print -r -- '{"claudeInChromeDefaultEnabled": null}' > "$managed/.claude.json"
check "null reads as off, not unknown" "off" "$(enabled_state "$managed/.claude.json")"

print -r -- '{"oauthAccount": {}}' > "$managed/.claude.json"
check "an absent key reads as off" "off" "$(enabled_state "$managed/.claude.json")"

# No file and unparseable file are both "we don't know", never "off" —
# reporting a brand-new account dir as switched off is a finding about
# nothing.
check "no file at all has no answer" "unknown" "$(enabled_state "$managed/nope.json")"

print -r -- '{not json' > "$managed/.claude.json"
check "an unparseable file has no answer" "unknown" "$(enabled_state "$managed/.claude.json")"

print -r -- ""
print -r -- "_claude_acc_chrome_seen:"

print -r -- '{"cachedChromeExtensionInstalled": true}' > "$managed/.claude.json"
check "a cached install counts as seen" "seen" "$(seen_state "$managed/.claude.json")"

print -r -- '{"chromeExtension": {"pairedDeviceId": "abc"}}' > "$managed/.claude.json"
check "a paired device counts as seen" "seen" "$(seen_state "$managed/.claude.json")"

# `chromeExtension: null` is what Claude Code writes when nothing has paired.
print -r -- '{"chromeExtension": null, "cachedChromeExtensionInstalled": false}' \
    > "$managed/.claude.json"
check "an unpaired dir is not seen" "unseen" "$(seen_state "$managed/.claude.json")"

print -r -- '{"oauthAccount": {}}' > "$managed/.claude.json"
check "a dir that recorded nothing is not seen" "unseen" "$(seen_state "$managed/.claude.json")"

print -r -- ""
print -r -- "_claude_acc_config_json:"

# The standard account is the odd one out: Claude Code writes its record at
# `CLAUDE_CONFIG_DIR ?? $HOME`, so for `default` the file sits beside
# ~/.claude/ rather than inside it. Reading the wrong path would report the
# standard account as "no answer" forever.
check "default points beside ~/.claude/" \
    "$HOME/.claude.json" "$(_claude_acc_config_json default)"
check "a managed account points inside its dir" \
    "$managed/.claude.json" "$(_claude_acc_config_json work)"

print -r -- ""
if (( failures )); then
    print -r -- "$failures check(s) failed"
    exit 1
fi
print -r -- "all checks passed"
