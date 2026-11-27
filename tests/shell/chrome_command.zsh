#!/usr/bin/env zsh
#
# `claude-acc chrome [<account>] [--on|--off]`.
#
# Claude Code keeps the Claude in Chrome switch per config dir, so a new
# account has the browser tools missing with nothing on screen to say why.
# Setting it used to need an interactive session — `/chrome` inside that
# account — the only thing in this tool's reach that did.
#
# The write is Claude Code's own `edit-chrome-settings`, so these checks
# stand a stub in for `claude` and assert on what it was handed: the
# payload, and the `CLAUDE_CONFIG_DIR` it was handed it under. A stub is the
# point rather than a shortcut — the real subcommand refuses to run from
# inside a Claude Code session, which is where CI lives.
#
# Run: zsh tests/shell/chrome_command.zsh

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

work="$CLAUDE_SWITCH_ACCOUNTS_DIR/work"
other="$CLAUDE_SWITCH_ACCOUNTS_DIR/work-2"
mkdir -p "$work" "$other"

print -r -- "reading the state:"

# The three answers, and the one that matters: a config dir Claude Code has
# never written has *no* answer, which is not the same as "off" — reporting
# a brand-new account as switched off would be a finding about nothing.
print -r -- '{"claudeInChromeDefaultEnabled": true}' > "$work/.claude.json"
check "true reads as on" "  work  Claude in Chrome: on" "$(_claude_acc_chrome work)"
print -r -- '{"claudeInChromeDefaultEnabled": false}' > "$work/.claude.json"
check "false reads as off" "  work  Claude in Chrome: off" "$(_claude_acc_chrome work)"
print -r -- '{"claudeInChromeDefaultEnabled": null}' > "$work/.claude.json"
check "null reads as off, not unknown" "  work  Claude in Chrome: off" "$(_claude_acc_chrome work)"
rm -f "$work/.claude.json"
check "a dir Claude Code never wrote has no answer" "  work  Claude in Chrome: unknown" \
    "$(_claude_acc_chrome work)"

# Every account, with the standard one last. Unlike `list`, the standard
# account is shown whether or not it has a token: the question here is
# whether a config dir has the switch on, and `~/.claude` is a config dir
# either way.
print -r -- '{"claudeInChromeDefaultEnabled": true}' > "$work/.claude.json"
check "with no name, every account, standard last" "  work  Claude in Chrome: on
  work-2  Claude in Chrome: unknown
  ~/.claude/  Claude in Chrome: unknown" "$(_claude_acc_chrome)"

print -r -- ""
print -r -- "what it refuses:"

out=$(_claude_acc_chrome nope 2>&1)
check "an account that is not there" "1" "$(print -r -- "$out" | grep -c "nope")"
out=$(_claude_acc_chrome 'work.lock' 2>&1)
check "a name no command takes" "1" \
    "$(print -r -- "$out" | grep -c 'must contain only letters')"
out=$(_claude_acc_chrome work extra 2>&1)
check "two account names" "1" \
    "$(print -r -- "$out" | grep -c 'Usage: claude-acc chrome')"

print -r -- ""
print -r -- "writing, through Claude Code's own subcommand:"

# The stub records what it was called with, and answers as the real
# subcommand does.
bin="$scratch/bin"
mkdir -p "$bin"
cat > "$bin/claude" <<'STUB'
#!/bin/sh
printf '%s\n' "args: $*" >> "$CLAUDE_STUB_LOG"
printf '%s\n' "dir: $CLAUDE_CONFIG_DIR" >> "$CLAUDE_STUB_LOG"
printf 'stdin: ' >> "$CLAUDE_STUB_LOG"
cat >> "$CLAUDE_STUB_LOG"
printf '\n' >> "$CLAUDE_STUB_LOG"
[ -n "$CLAUDE_STUB_FAIL" ] && { printf '%s\n' "$CLAUDE_STUB_FAIL"; exit 1; }
printf '{"ok":true,"written":{"enabledByDefault":true}}\n'
STUB
chmod +x "$bin/claude"
export PATH="$bin:$PATH"
export CLAUDE_STUB_LOG="$scratch/stub.log"
export CLAUDE_STUB_FAIL=""

: > "$CLAUDE_STUB_LOG"
out=$(_claude_acc_chrome work --on 2>&1)
check "it reports what it set" "1" \
    "$(print -r -- "$out" | grep -c 'work  Claude in Chrome: on')"
check "and says the session has to restart" "1" \
    "$(print -r -- "$out" | grep -c 'restart')"
check "the subcommand is Claude Code's own" "args: edit-chrome-settings --json" \
    "$(grep '^args:' "$CLAUDE_STUB_LOG")"
check "aimed at that account's config dir" "dir: $work" \
    "$(grep '^dir:' "$CLAUDE_STUB_LOG")"
check "with the payload it documents" 'stdin: {"enabledByDefault":true}' \
    "$(grep '^stdin:' "$CLAUDE_STUB_LOG")"

: > "$CLAUDE_STUB_LOG"
_claude_acc_chrome work --off >/dev/null 2>&1
check "--off sends false, not true" 'stdin: {"enabledByDefault":false}' \
    "$(grep '^stdin:' "$CLAUDE_STUB_LOG")"

# The standard account's config record sits beside ~/.claude, not inside it,
# but the *config dir* handed to Claude Code is the directory itself.
mkdir -p "$HOME/.claude"
: > "$CLAUDE_STUB_LOG"
_claude_acc_chrome default --on >/dev/null 2>&1
check "default aims at ~/.claude, never accounts/default" "dir: $HOME/.claude" \
    "$(grep '^dir:' "$CLAUDE_STUB_LOG")"
check "and nothing was created at accounts/default" "no" \
    "$([[ -e "$CLAUDE_SWITCH_ACCOUNTS_DIR/default" ]] && print -r -- yes || print -r -- no)"

print -r -- ""
print -r -- "when Claude Code says no:"

# Its own guard against being run from inside a session. The refusal is
# passed through verbatim rather than interpreted — the subcommand is
# undocumented, so a future build may refuse for a reason we have not seen.
export CLAUDE_STUB_FAIL="This setting cannot be changed from an editor started inside a Claude Code session."
out=$(_claude_acc_chrome work --on 2>&1)
check "the refusal is passed through" "1" \
    "$(print -r -- "$out" | grep -c 'Claude Code refused: This setting cannot be changed')"
_claude_acc_chrome work --on >/dev/null 2>&1
check "and the command fails" "1" "$?"
export CLAUDE_STUB_FAIL=""

print -r -- ""
if (( failures )); then
    print -r -- "$failures check(s) failed"
    exit 1
fi
print -r -- "all checks passed"
