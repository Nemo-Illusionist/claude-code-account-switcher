#!/usr/bin/env zsh
#
# A directory under accounts/ is not an account just because it is a
# directory.
#
# `accounts/work.lock` turned up beside a real account — left by something
# writing a sibling path, not by this tool: `add`, `login` and `import` all
# refuse a name with a dot in it. Enumeration took every subdirectory
# regardless, so the stray one became a row of `list`, an entry in
# completion, and a `doctor` finding that advised `claude-acc login
# work.lock` — a command that rejects its own argument.
#
# The assertions drive `list`, the user-visible side, rather than the
# enumerator: a test that called the new helper would fail on the base
# branch with "command not found" and pin nothing about the bug. The
# fixture's names are checked against the rule by hand, not by calling it.
#
# Run: zsh tests/shell/account_names.zsh

emulate -L zsh
set -u

typeset -i failures=0
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT

export HOME="$scratch/home"
mkdir -p "$HOME/.claude"

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

# The account column of `list`, comma-joined: the first word of every row
# that names a managed account. The header, the `★` of the default one and
# the `~/.claude/` row are dropped, so what is left is exactly the set of
# directories the command took for accounts.
listed() {
    local -a rows
    rows=(${(f)"$(_claude_acc_list 2>/dev/null |
        awk '/^( {4}|  ★ )/ { sub(/^[[:space:]]*(★ )?/, ""); print $1 }')"})
    # The standard account's row, however its suffix is spelled. Quoted, so
    # the leading tilde is a character and not this test's own home.
    rows=("${(@)rows:#'~'/.claude/*}")
    print -r -- "${(j:,:)rows}"
}

# The account column of `doctor`, same shape. `doctor` is where the bug was
# loudest — it audited the stray directory and printed advice that cannot be
# followed — and it reaches the accounts through a different call site than
# `list`, so reverting either one alone has to fail something here.
audited() {
    local -a rows
    rows=(${(f)"$(_claude_acc_doctor 2>/dev/null |
        awk '/^  [?✓✗] / { print $2 }')"})
    rows=("${(@)rows:#'~'/.claude/*}")
    print -r -- "${(j:,:)rows}"
}

print -r -- "accounts listed by \`list\`:"

# No two names here differ only by case: on a case-insensitive filesystem
# they would fold onto one directory and the fixture would not be what it
# reads as.
for n in work work-2 a_b; do mkdir -p "$CLAUDE_SWITCH_ACCOUNTS_DIR/$n"; done
check "every name the tool would accept is listed" "a_b,work,work-2" "$(listed)"

# The case this exists for.
mkdir -p "$CLAUDE_SWITCH_ACCOUNTS_DIR/work.lock"
check "a dot-suffixed sibling is not listed" "a_b,work,work-2" "$(listed)"

mkdir -p "$CLAUDE_SWITCH_ACCOUNTS_DIR/work lock" "$CLAUDE_SWITCH_ACCOUNTS_DIR/.hidden" \
    "$CLAUDE_SWITCH_ACCOUNTS_DIR/работа"
check "whitespace, dotfiles and unicode are not listed" "a_b,work,work-2" "$(listed)"

# A file is not an account either, however it is named.
print -r -- "notes" > "$CLAUDE_SWITCH_ACCOUNTS_DIR/personal"
check "a plain file with a valid name is not listed" "a_b,work,work-2" "$(listed)"

# A directory reached through a symlink is one, though: the globs this
# replaced matched any entry, so these were listed before.
mkdir -p "$scratch/elsewhere"
ln -s "$scratch/elsewhere" "$CLAUDE_SWITCH_ACCOUNTS_DIR/linked"
check "a symlinked account dir is still listed" "a_b,linked,work,work-2" "$(listed)"

print -r -- ""
print -r -- "accounts audited by \`doctor\`:"

check "the stray directory is not audited" "a_b,linked,work,work-2" "$(audited)"

print -r -- ""
if (( failures )); then
    print -r -- "$failures check(s) failed"
    exit 1
fi
print -r -- "all checks passed"
