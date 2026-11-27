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

# The account names `usage` prints: its rows are a name on one line and the
# figure or reason indented under it, so the four-space rows are the names.
used() {
    local -a rows
    rows=(${(f)"$(_claude_acc_usage 2>/dev/null |
        awk '/^ {4}[^ ]/ { sub(/^[[:space:]]*(★ )?/, ""); print $1 }')"})
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

# A symlink is not an account either, matching the Rust side: `list_accounts`
# never listed one, and `gui::account_dir` refuses storage reached through a
# symlink outright. Four shapes, because the hints make different claims
# about them: a link to a directory, a link to a file, a dangling link, and
# a link whose name no command would take.
mkdir -p "$scratch/elsewhere"
: > "$scratch/target.txt"
ln -s "$scratch/elsewhere" "$CLAUDE_SWITCH_ACCOUNTS_DIR/linked"
ln -s "$scratch/target.txt" "$CLAUDE_SWITCH_ACCOUNTS_DIR/tofile"
ln -s "$scratch/gone" "$CLAUDE_SWITCH_ACCOUNTS_DIR/dangling"
ln -s "$scratch/elsewhere" "$CLAUDE_SWITCH_ACCOUNTS_DIR/linked.2"
# A dotted symlink, to pin that the symlink glob skips dotfiles too.
ln -s "$scratch/elsewhere" "$CLAUDE_SWITCH_ACCOUNTS_DIR/.hidden-link"
check "a symlinked dir is not listed" "a_b,work,work-2" "$(listed)"

print -r -- ""
print -r -- "accounts audited by \`doctor\`:"

check "the stray directory is not audited" "a_b,work,work-2" "$(audited)"

print -r -- ""
print -r -- "accounts shown by \`usage\`:"

check "the stray directory has no usage line" "a_b,work,work-2" "$(used)"

print -r -- ""
print -r -- "what \`doctor\` says about the rest:"

# The names one hint lists, read out of the line itself: everything between
# the first colon (which follows the directory) and the sentence after the
# list. Compared as whole lists rather than greppped for, because `linked`
# is a substring of `linked.2` and a count would pass for the wrong reason.
named_in() {
    _claude_acc_doctor 2>/dev/null | grep -- "$1" |
        sed -E 's/^[^:]*: //; s/\. .*$//'
}

# Untouchable: a name no command takes, whatever the entry is — and a name
# they would take on something that is not a directory. `remove` refuses
# both, so the user's own `rm` is the only way out and they must know.
check "the entries nothing here can act on" \
    "linked.2, personal, work lock, work.lock, работа" \
    "$(named_in '^Not accounts, in ')"

# Half an account: `run` and `remove` follow a symlink, so saying nothing
# here can act on it would be false. Shapes that must all land here: a link
# to a directory, a link to a file, and a dangling link.
check "the symlinks under a usable name" "dangling, linked, tofile" \
    "$(named_in '^Symlinks, in ')"

# `.DS_Store` arrives whenever anyone opens the directory in Finder. A line
# naming it every run, about a file that comes straight back, is worse than
# silence — and the Rust side skips dotfiles for the same reason.
: > "$CLAUDE_SWITCH_ACCOUNTS_DIR/.DS_Store"
check "a dotfile is in neither line" "0" \
    "$(_claude_acc_doctor 2>/dev/null | grep -c 'DS_Store')"
check "nor is a dotted symlink" "0" \
    "$(_claude_acc_doctor 2>/dev/null | grep -c 'hidden-link')"

print -r -- "the ide/ symlink pass:"

# The one site that writes rather than prints: on master the stray directory
# got an `ide` symlink of its own, because the pass walked every
# subdirectory. That is how the one on this machine came to hold anything.
_claude_ensure_ide_symlinks
check "an account gets its ide symlink" "yes" \
    "$([[ -L "$CLAUDE_SWITCH_ACCOUNTS_DIR/work/ide" ]] && print -r -- yes || print -r -- no)"
check "the stray directory does not" "no" \
    "$([[ -e "$CLAUDE_SWITCH_ACCOUNTS_DIR/work.lock/ide" ]] && print -r -- yes || print -r -- no)"

print -r -- ""
if (( failures )); then
    print -r -- "$failures check(s) failed"
    exit 1
fi
print -r -- "all checks passed"
