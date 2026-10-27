#!/usr/bin/env zsh
#
# `import` must not take a source that holds the accounts directory, and
# `import --move` must not take one that holds ~/.claude.
#
# Regression: `import x ~` copied the home directory, accounts and all, into
# ~/.claude-switch/accounts/x, and `import x ~/.claude --move` moved the
# standard account away from Claude Code. Both are refused before anything is
# copied or moved; ordinary imports, including a copy of ~/.claude, still work.
#
# Run: zsh tests/shell/import_containment.zsh

emulate -L zsh
set -u

typeset -i failures=0
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT

# Sourcing the script writes into ~/.claude-switch/; keep the real one out.
export HOME="$scratch/home"
mkdir -p "$HOME/.claude" "$HOME/.claude-work"
export CLAUDE_ACC_LANG=en
source "${0:A:h:h:h}/claude-switch.sh" >/dev/null 2>&1
mkdir -p "$CLAUDE_SWITCH_ACCOUNTS_DIR"

# No Keychain and no network: neither is what this checks.
_claude_acc_rekey_keychain() { return 1 }
_claude_acc_token() { return 0 }

refused() {
    local want="$1" name="$2"; shift 2
    local out
    out=$(_claude_acc_import "$name" "$@" 2>&1)
    if [[ $? -eq 0 || "$out" != *"$want"* ]]; then
        print -r -- "FAIL: import $name $* was not refused with '$want': $out"
        failures+=1
    fi
    if [[ -e "$CLAUDE_SWITCH_ACCOUNTS_DIR/$name" ]]; then
        print -r -- "FAIL: import $name $* created the account"
        failures+=1
    fi
}

refused "cannot be imported into it" a "$HOME"
refused "cannot be imported into it" b "$CLAUDE_SWITCH_DIR"
refused "cannot be imported into it" c "$HOME/.claude-work/.."
refused "standard account" d "$HOME/.claude" --move
refused "cannot be imported into it" e "$HOME" --move
[[ -d "$HOME/.claude" ]] || { print -r -- "FAIL: ~/.claude was moved"; failures+=1; }

# What import is for still works.
_claude_acc_import copied "$HOME/.claude" >/dev/null 2>&1
[[ -d "$CLAUDE_SWITCH_ACCOUNTS_DIR/copied" && -d "$HOME/.claude" ]] ||
    { print -r -- "FAIL: copying ~/.claude did not import it"; failures+=1; }
_claude_acc_import moved "$HOME/.claude-work" --move >/dev/null 2>&1
[[ -d "$CLAUDE_SWITCH_ACCOUNTS_DIR/moved" && ! -e "$HOME/.claude-work" ]] ||
    { print -r -- "FAIL: moving a hand-rolled dir did not import it"; failures+=1; }

if (( failures )); then
    print -r -- "import_containment: $failures failure(s)"
    exit 1
fi
print -r -- "import_containment: ok"
