#!/usr/bin/env zsh
#
# The generated `claude` wrapper must not overrule an account the caller
# already chose.
#
# The wrapper sources claude-switch.sh to reuse its directory→account
# lookup, and the script used to activate for $PWD as a side effect of
# being sourced. Inside a linked directory that overwrote the choice
# `claude-acc run` had just made: `run default` and `run <other>` both
# ended up on the directory's account, silently.
#
# Run: zsh tests/shell/run_account_wrapper.zsh

emulate -L zsh
set -u

typeset -i failures=0
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT

# Everything the script reads and writes hangs off $HOME, so a scratch HOME
# keeps the real ~/.claude-switch/ (wrapper included) untouched.
export HOME="$scratch/home"
mkdir -p "$HOME"

script="${0:A:h:h:h}/claude-switch.sh"
source "$script" >/dev/null 2>&1

mkdir -p "$CLAUDE_SWITCH_ACCOUNTS_DIR/work" "$CLAUDE_SWITCH_ACCOUNTS_DIR/personal"
linked="$scratch/linked"
mkdir -p "$linked"
print -r -- "$linked=work" > "$CLAUDE_SWITCH_LINKS"

# A stand-in for the real claude binary: reports the account it was handed.
fake_bin="$scratch/bin"
mkdir -p "$fake_bin"
cat > "$fake_bin/claude" <<'FAKE'
#!/bin/sh
printf '%s\n' "${CLAUDE_CONFIG_DIR-unset}"
FAKE
chmod +x "$fake_bin/claude"

# The wrapper searches PATH for the real binary, skipping its own directory.
export PATH="$CLAUDE_SWITCH_BIN:$fake_bin:$PATH"

# check <case name> <expected CLAUDE_CONFIG_DIR> <env assignment>...
check() {
    local name="$1" want="$2"
    shift 2
    local got
    got=$(cd "$linked" && env "$@" "$CLAUDE_SWITCH_BIN/claude" 2>/dev/null)
    if [[ "$got" == "$want" ]]; then
        print -r -- "ok   — $name"
    else
        print -r -- "FAIL — $name: want '$want', got '$got'"
        (( failures++ ))
    fi
}

# `claude-acc run default` in a linked directory: the standard ~/.claude/
# account, not the one the directory is linked to.
check 'run default keeps the standard account' \
    'unset' -u CLAUDE_CONFIG_DIR CLAUDE_ACC_RUN_DEFAULT=1

# `claude-acc run personal` in a directory linked to work.
check 'run <name> keeps the named account' \
    "$CLAUDE_SWITCH_ACCOUNTS_DIR/personal" \
    "CLAUDE_CONFIG_DIR=$CLAUDE_SWITCH_ACCOUNTS_DIR/personal"

# Plain `claude` with no account chosen still gets the directory's own.
check 'plain claude activates for the directory' \
    "$CLAUDE_SWITCH_ACCOUNTS_DIR/work" -u CLAUDE_CONFIG_DIR

# The marker is ours; claude must never see it.
cat > "$fake_bin/claude" <<'FAKE'
#!/bin/sh
printf '%s\n' "${CLAUDE_ACC_RUN_DEFAULT-unset}"
FAKE
chmod +x "$fake_bin/claude"
check 'the run-default marker does not leak into claude' \
    'unset' -u CLAUDE_CONFIG_DIR CLAUDE_ACC_RUN_DEFAULT=1

if (( failures )); then
    print -r -- "$failures test(s) failed"
    exit 1
fi
print -r -- 'all tests passed'
