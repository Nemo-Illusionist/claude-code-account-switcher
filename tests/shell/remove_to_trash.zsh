#!/usr/bin/env zsh
#
# `remove` puts the account directory in the Trash instead of unlinking it.
#
# An account dir holds transcripts, settings and plugins that exist nowhere
# else, and `rm -rf` left no way back from a mistyped name. Getting it wrong
# should cost a drag out of the Trash, not a restore from backup.
#
# Run: zsh tests/shell/remove_to_trash.zsh

emulate -L zsh
set -u

typeset -i failures=0
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT

# A scratch HOME keeps the real installation — and the real ~/.Trash — out
# of this entirely.
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

ok() {
    local label="$1"
    if eval "$2"; then
        print -r -- "  ok   $label"
    else
        print -r -- "  FAIL $label"
        (( failures++ ))
    fi
}

print -r -- "_claude_acc_to_trash:"

src="$scratch/work"
mkdir -p "$src"
print -r -- "contents" > "$src/marker"

dest=$(_claude_acc_to_trash "$src")
check "lands at ~/.Trash/<name>" "$HOME/.Trash/work" "$dest"
ok "the original is gone" '[[ ! -e "$src" ]]'
ok "the contents came along" '[[ "$(cat "$HOME/.Trash/work/marker")" == contents ]]'

# Removing an account called `work` twice is the ordinary case: the Trash is
# flat and shared with everything else thrown away.
mkdir -p "$src"
print -r -- "second" > "$src/marker"
dest=$(_claude_acc_to_trash "$src")
check "a second entry is numbered, not merged" "$HOME/.Trash/work 2" "$dest"
ok "the first entry is untouched" '[[ "$(cat "$HOME/.Trash/work/marker")" == contents ]]'
ok "the second entry has its own contents" '[[ "$(cat "$HOME/.Trash/work 2/marker")" == second ]]'

mkdir -p "$src"
dest=$(_claude_acc_to_trash "$src")
check "and a third keeps counting" "$HOME/.Trash/work 3" "$dest"

# A source that is not there at all must report failure, so the caller falls
# back rather than printing "moved to the Trash" about nothing.
if _claude_acc_to_trash "$scratch/not-here" >/dev/null 2>&1; then
    print -r -- "  FAIL a missing source reports failure"
    (( failures++ ))
else
    print -r -- "  ok   a missing source reports failure"
fi

print -r -- ""
print -r -- "remove, end to end:"

acc="$CLAUDE_SWITCH_ACCOUNTS_DIR/personal"
mkdir -p "$acc/projects"
print -r -- "transcript" > "$acc/projects/session.jsonl"

out=$(_claude_acc_remove -f personal 2>&1)

ok "the account dir is out of accounts/" '[[ ! -e "$acc" ]]'
ok "it is in the Trash" '[[ -d "$HOME/.Trash/personal" ]]'
ok "with its transcripts" \
    '[[ "$(cat "$HOME/.Trash/personal/projects/session.jsonl")" == transcript ]]'
ok "the message names the Trash" '[[ "$out" == *"moved to the Trash"* ]]'
ok "the message says it is undoable" '[[ "$out" == *"drag it back out"* ]]'
ok "and does not claim a deletion" '[[ "$out" != *"deleted."* ]]'

print -r -- ""
print -r -- "remove --purge:"

# `--purge` is the explicit "actually gone" path: the Trash keeps holding
# the disk space, so someone who means it needs a way to say so.
acc="$CLAUDE_SWITCH_ACCOUNTS_DIR/throwaway"
mkdir -p "$acc"
print -r -- "secret" > "$acc/marker"

out=$(_claude_acc_remove --purge -f throwaway 2>&1)

ok "the account dir is gone" '[[ ! -e "$acc" ]]'
ok "and did NOT land in the Trash" '[[ ! -e "$HOME/.Trash/throwaway" ]]'
ok "the message says deleted" '[[ "$out" == *"deleted."* ]]'
ok "and never mentions the Trash" '[[ "$out" != *"Trash"* ]]'

# Flag order must not matter — `-f --purge` and `--purge -f` are the same
# request, and getting it wrong would silently fall back to the Trash.
acc="$CLAUDE_SWITCH_ACCOUNTS_DIR/throwaway2"
mkdir -p "$acc"
out=$(_claude_acc_remove -f --purge throwaway2 2>&1)
ok "flag order does not matter" \
    '[[ ! -e "$acc" && ! -e "$HOME/.Trash/throwaway2" && "$out" == *"deleted."* ]]'

# Without --purge the prompt has to promise the Trash, because that promise
# is what makes a hasty `y` safe.
acc="$CLAUDE_SWITCH_ACCOUNTS_DIR/prompted"
mkdir -p "$acc"
out=$(print -r -- "n" | _claude_acc_remove prompted 2>&1)
ok "the plain prompt promises the Trash" '[[ "$out" == *"goes to the Trash"* ]]'
ok "and answering n keeps the account" '[[ -d "$acc" ]]'

out=$(print -r -- "n" | _claude_acc_remove --purge prompted 2>&1)
ok "the purge prompt warns it cannot be undone" '[[ "$out" == *"cannot be undone"* ]]'
ok "and answering n still keeps the account" '[[ -d "$acc" ]]'

print -r -- ""
if (( failures )); then
    print -r -- "$failures check(s) failed"
    exit 1
fi
print -r -- "all checks passed"
