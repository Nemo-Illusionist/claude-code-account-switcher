#!/usr/bin/env zsh
#
# `clone-settings --from <account>` seeds one account from another.
#
# The source used to be `~/.claude/` and nothing else, so the obvious thing
# — "give this new account the setup of the account I already use" — could
# not be asked for. Doing it by hand means getting the plugin registry's
# rewritten paths right by hand too, which is the part that breaks quietly:
# a plain copy leaves the new account loading the other one's plugin cache,
# working by accident until that account is removed.
#
# Run: zsh tests/shell/clone_settings_from.zsh

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
target="$CLAUDE_SWITCH_ACCOUNTS_DIR/work-2"
mkdir -p "$work/agents" "$work/plugins/cache/local/helper/1.0.0" "$target"

print -r -- '{"theme":"dark"}' > "$work/settings.json"
print -r -- 'memory' > "$work/CLAUDE.md"
print -r -- 'agent' > "$work/agents/one.md"
print -r -- 'p' > "$work/plugins/cache/local/helper/1.0.0/plugin.md"
# A registry pointing into the source account, plus a marketplace sourced
# from outside any config dir — that one has to survive verbatim.
print -r -- "{\"plugins\":{\"helper@local\":[{\"scope\":\"user\",\"installPath\":\"$work/plugins/cache/local/helper/1.0.0\"}]}}" \
    > "$work/plugins/installed_plugins.json"
print -r -- "{\"local\":{\"installLocation\":\"$work/plugins/marketplaces/local\"},\"repo\":{\"installLocation\":\"/repo/.claude/plugins\"}}" \
    > "$work/plugins/known_marketplaces.json"

# The standard account holds something different, so a test that passed
# `--from` and still read `~/.claude/` would show it.
print -r -- 'standard memory' > "$HOME/.claude/CLAUDE.md"

print -r -- "clone-settings work-2 --from work:"

_claude_acc_clone_settings work-2 --from work >/dev/null 2>&1

check "settings.json came from the source account" '{"theme":"dark"}' \
    "$(cat "$target/settings.json" 2>/dev/null)"
check "CLAUDE.md came from the account, not from ~/.claude/" "memory" \
    "$(cat "$target/CLAUDE.md" 2>/dev/null)"
check "agents/ came over" "agent" "$(cat "$target/agents/one.md" 2>/dev/null)"
check "the plugin cache came over" "p" \
    "$(cat "$target/plugins/cache/local/helper/1.0.0/plugin.md" 2>/dev/null)"

# The rewrite: paths into the source account point at the target now, and
# the one outside any config dir is untouched. Expected values written out
# here rather than computed from the script.
check "installPath was repointed at the target" \
    "$target/plugins/cache/local/helper/1.0.0" \
    "$(jq -r '.plugins["helper@local"][0].installPath' "$target/plugins/installed_plugins.json" 2>/dev/null)"
check "the other fields are untouched" "user" \
    "$(jq -r '.plugins["helper@local"][0].scope' "$target/plugins/installed_plugins.json" 2>/dev/null)"
check "installLocation was repointed too" \
    "$target/plugins/marketplaces/local" \
    "$(jq -r '.local.installLocation' "$target/plugins/known_marketplaces.json" 2>/dev/null)"
check "a marketplace outside the config dir keeps its path" "/repo/.claude/plugins" \
    "$(jq -r '.repo.installLocation' "$target/plugins/known_marketplaces.json" 2>/dev/null)"

print -r -- ""
print -r -- "what it refuses:"

# Seeding an account from itself is a typo, not a no-op worth running.
out=$(_claude_acc_clone_settings work --from work 2>&1)
check "an account cannot be seeded from itself" "1" \
    "$(print -r -- "$out" | grep -c 'is the account being seeded')"
# A source that is not an account at all.
out=$(_claude_acc_clone_settings work-2 --from nope 2>&1)
check "a source that does not exist is refused" "1" \
    "$(print -r -- "$out" | grep -c 'nope')"
# A name no command here takes. Checked against the name-invalid text, not
# merely against the absence of another message: without the name check the
# run still fails, as "no such account", and the weaker assertion passed.
out=$(_claude_acc_clone_settings work-2 --from 'work.lock' 2>&1)
check "a name no command takes is refused as invalid" "1" \
    "$(print -r -- "$out" | grep -c 'must contain only letters')"

# A bare `--from`, which used to spin for ever: `shift 2` with one argument
# left shifts nothing, so the loop read `--from` again and again. Run under
# a watchdog, because the failure mode is a hang rather than a wrong answer.
( _claude_acc_clone_settings work-2 --from >/dev/null 2>&1 ) &
bare_pid=$!
bare_result="hung"
for _ in {1..50}; do
    if ! kill -0 $bare_pid 2>/dev/null; then bare_result="returned"; break; fi
    sleep 0.1
done
[[ "$bare_result" == "hung" ]] && kill -9 $bare_pid 2>/dev/null
wait $bare_pid 2>/dev/null
check "a bare --from returns instead of spinning" "returned" "$bare_result"

# Two names is a mistake, not a silent choice of the second.
out=$(_claude_acc_clone_settings work-2 extra 2>&1)
check "two account names are refused" "1" \
    "$(print -r -- "$out" | grep -c 'Usage: claude-acc clone-settings')"

# `--from=` with nothing after it is a mistake too, not the standard account.
out=$(_claude_acc_clone_settings work-2 --from= 2>&1)
check "an empty --from is refused rather than silently defaulted" "1" \
    "$(print -r -- "$out" | grep -c 'must contain only letters')"

print -r -- ""
print -r -- "without --from, and with default:"

fresh="$CLAUDE_SWITCH_ACCOUNTS_DIR/fresh"
mkdir -p "$fresh"
_claude_acc_clone_settings fresh >/dev/null 2>&1
check "the source is ~/.claude/ when --from is absent" "standard memory" \
    "$(cat "$fresh/CLAUDE.md" 2>/dev/null)"

fresh2="$CLAUDE_SWITCH_ACCOUNTS_DIR/fresh2"
mkdir -p "$fresh2"
_claude_acc_clone_settings fresh2 --from default >/dev/null 2>&1
check "\`default\` names the standard account, not accounts/default" "standard memory" \
    "$(cat "$fresh2/CLAUDE.md" 2>/dev/null)"
check "and nothing was created at accounts/default" "no" \
    "$([[ -e "$CLAUDE_SWITCH_ACCOUNTS_DIR/default" ]] && print -r -- yes || print -r -- no)"

print -r -- ""
print -r -- "the nothing-to-copy message:"

# It used to say `~/.claude/` whatever the source was.
empty_src="$CLAUDE_SWITCH_ACCOUNTS_DIR/bare"
empty_dst="$CLAUDE_SWITCH_ACCOUNTS_DIR/bare-2"
mkdir -p "$empty_src" "$empty_dst"
out=$(_claude_acc_clone_settings bare-2 --from bare 2>&1)
check "it names the account it was pointed at" "1" \
    "$(print -r -- "$out" | grep -c "nothing to copy from ~/.claude-switch/accounts/bare$")"
check "and not ~/.claude/" "0" \
    "$(print -r -- "$out" | grep -c 'nothing to copy from ~/.claude/')"

print -r -- ""
if (( failures )); then
    print -r -- "$failures check(s) failed"
    exit 1
fi
print -r -- "all checks passed"
