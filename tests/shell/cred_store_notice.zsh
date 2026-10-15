#!/usr/bin/env zsh
#
# `doctor` has to say when CLAUDE_SECURESTORAGE_CONFIG_DIR has collapsed every
# account into one credential entry.
#
# Since #150 both the binary and this script follow that variable, so they
# agree with Claude Code again. What neither could do is say what it costs:
# with one shared entry every account reads the same blob and reports the same
# identity, which looks exactly like a genuinely shared login (#151).
#
# Mirrors `shared_credential_store` in src/commands/doctor.rs. Set-but-empty
# is a separate outcome upstream rather than a missing value — the
# per-directory suffix is dropped entirely — so it gets its own message.
#
# Run: zsh tests/shell/cred_store_notice.zsh

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

print -r -- "_claude_acc_cred_store_notice:"

# Unset: silence. This is the path everyone is on, so a notice here would be
# noise in every single `doctor` run.
unset CLAUDE_SECURESTORAGE_CONFIG_DIR 2>/dev/null
check "an unset variable says nothing" "" "$(_claude_acc_cred_store_notice)"

check "a path override reports the path" \
    $'shared\t/Users/x/.claude' \
    "$(CLAUDE_SECURESTORAGE_CONFIG_DIR=/Users/x/.claude _claude_acc_cred_store_notice)"

check "an empty override is the legacy case" \
    "legacy" \
    "$(CLAUDE_SECURESTORAGE_CONFIG_DIR= _claude_acc_cred_store_notice)"

# Whitespace is a path as far as Claude Code is concerned — only an exactly
# empty value takes the unsuffixed branch there, so trimming would report the
# wrong outcome.
check "whitespace is a path, not an empty value" \
    $'shared\t ' \
    "$(CLAUDE_SECURESTORAGE_CONFIG_DIR=' ' _claude_acc_cred_store_notice)"

print -r -- ""
print -r -- "doctor output and exit code:"

# One real account so `doctor` has a row to print.
mkdir -p "$CLAUDE_SWITCH_ACCOUNTS_DIR/work"

out=$(CLAUDE_SECURESTORAGE_CONFIG_DIR=/Users/x/.claude claude-acc doctor 2>&1)
rc_shared=$?
if [[ "$out" == *"CLAUDE_SECURESTORAGE_CONFIG_DIR is set to /Users/x/.claude"* ]]; then
    print -r -- "  ok   the path case names the variable and the path"
else
    print -r -- "  FAIL the path case: got '${out}'"
    (( failures++ ))
fi

out_legacy=$(CLAUDE_SECURESTORAGE_CONFIG_DIR= claude-acc doctor 2>&1)
if [[ "$out_legacy" == *"set but empty"* ]]; then
    print -r -- "  ok   the empty case says it is empty"
else
    print -r -- "  FAIL the empty case: got '${out_legacy}'"
    (( failures++ ))
fi
# The two messages must not be the same text — that is the whole reason there
# are two of them.
if [[ "$out" != "$out_legacy" ]]; then
    print -r -- "  ok   the two cases read differently"
else
    print -r -- "  FAIL both cases printed the same thing"
    (( failures++ ))
fi

out_quiet=$(claude-acc doctor 2>&1)
if [[ "$out_quiet" != *"CLAUDE_SECURESTORAGE_CONFIG_DIR"* ]]; then
    print -r -- "  ok   nothing is said when the variable is unset"
else
    print -r -- "  FAIL a notice appeared with the variable unset"
    (( failures++ ))
fi

# Regression: adding this notice re-declared a name the function already had
# as a local, and zsh *prints* `name=value` for a bare `local` on an existing
# local — so a stray `kind=managed` line appeared in every doctor run. No
# line of doctor's output should look like a leaked variable.
if [[ "$out_quiet" == *$'\nkind='* || "$out_quiet" == *$'\nnotice='* \
   || "$out_quiet" == *$'\ndir='* || "$out_quiet" == *$'\ncred_notice='* ]]; then
    print -r -- "  FAIL a variable leaked into the output: '$out_quiet'"
    (( failures++ ))
else
    print -r -- "  ok   no declaration leaks into the output"
fi

# The notice is informational. `doctor`'s exit code means "wrong identity" and
# a script gating on it must not start failing over a configuration choice.
rc_quiet=0
claude-acc doctor >/dev/null 2>&1 || rc_quiet=$?
rc_set=0
CLAUDE_SECURESTORAGE_CONFIG_DIR=/Users/x/.claude claude-acc doctor >/dev/null 2>&1 || rc_set=$?
check "the notice does not change the exit code" "$rc_quiet" "$rc_set"

print -r -- ""
print -r -- "Russian:"

out_ru=$(CLAUDE_ACC_LANG=ru CLAUDE_SECURESTORAGE_CONFIG_DIR= claude-acc doctor 2>&1)
if [[ "$out_ru" == *"выставлена, но пуста"* ]]; then
    print -r -- "  ok   the empty case is translated"
else
    print -r -- "  FAIL the empty case is not translated: got '${out_ru}'"
    (( failures++ ))
fi

print -r -- ""
if (( failures )); then
    print -r -- "$failures check(s) failed"
    exit 1
fi
print -r -- "all checks passed"
