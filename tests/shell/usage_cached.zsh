#!/usr/bin/env zsh
#
# The offline fallback for `usage`: Claude Code's own last reading.
#
# When the API cannot be reached, `usage` used to print "token present, but
# API unreachable" and nothing else — while Claude Code's own figures were
# sitting in `cachedUsageUtilization` in that config dir's .claude.json, one
# local JSON parse away.
#
# These pin `_claude_acc_usage_cached`, which decides whether that reading may
# be used at all. The guard is the point: Claude Code stamps the cache with
# `accountUuid`, and a config dir since logged in as someone else would
# otherwise report another identity's spend as its own.
#
# Run: zsh tests/shell/usage_cached.zsh

emulate -L zsh
set -u

typeset -i failures=0
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT

# Sourcing the script runs its init, which writes into ~/.claude-switch/.
export HOME="$scratch/home"
mkdir -p "$HOME/.claude"

# Pin the language so the rendered lines below are the same on any machine.
# It also keeps `set -u` above from tripping over `_claude_acc_lang`, which
# reads $CLAUDE_ACC_LANG unguarded — a separate bug, not this test's subject.
export CLAUDE_ACC_LANG=en

source "${0:A:h:h:h}/claude-switch.sh" >/dev/null 2>&1

f="$scratch/config.json"

check() {
    local label="$1" expected="$2" actual="$3"
    if [[ "$expected" == "$actual" ]]; then
        print -r -- "  ok   $label"
    else
        print -r -- "  FAIL $label: expected '$expected', got '$actual'"
        (( failures++ ))
    fi
}

# A reading taken $1 seconds ago, stamped $2, signed in as $3. Pass the empty
# string for either uuid to leave that side out entirely.
write_config() {
    local age="$1" stamped="$2" signed="$3"
    local ms=$(( ($(date +%s) - age) * 1000 ))
    local stamp="" account=""
    [[ -n "$stamped" ]] && stamp=", \"accountUuid\": \"$stamped\""
    [[ -n "$signed" ]] && account=", \"oauthAccount\": {\"accountUuid\": \"$signed\"}"
    cat > "$f" <<JSON
{
  "cachedUsageUtilization": {
    "fetchedAtMs": $ms$stamp,
    "utilization": {
      "five_hour": {"utilization": 32, "resets_at": "2099-01-01T00:00:00Z"},
      "seven_day": {"utilization": 40, "resets_at": "2099-01-01T00:00:00Z"}
    }
  }$account
}
JSON
}

# "ok:<age>:<window count>" or "refused", so a failure says what was meant.
outcome() {
    local out
    if ! out=$(_claude_acc_usage_cached "$f"); then
        print -r -- "refused"
        return
    fi
    local -a lines
    lines=("${(@f)out}")
    print -r -- "ok:${lines[1]}:$(( ${#lines} - 1 ))"
}

print -r -- "_claude_acc_usage_cached:"

write_config 600 "u-1" "u-1"
check "a stamped, matching reading is usable" "ok:600:2" "$(outcome)"

# The guard this exists for.
write_config 60 "u-old" "u-new"
check "a reading stamped for another account is refused" "refused" "$(outcome)"

write_config 60 "" "u-new"
check "an unstamped reading is refused once someone is signed in" "refused" "$(outcome)"

# Both sides absent still agree — a dir Claude Code never stamped, with
# nobody recorded as signed in, is not a mismatch.
write_config 60 "" ""
check "a reading is usable when neither side names an account" "ok:60:2" "$(outcome)"

# A clock that moved backwards since the stamp was written must read as
# fresh, never as a huge number.
write_config -3600 "u-1" "u-1"
check "a stamp from the future reads as fresh" "ok:0:2" "$(outcome)"

print -r -- '{"oauthAccount": {"accountUuid": "u-1"}}' > "$f"
check "a file with no reading at all is refused" "refused" "$(outcome)"

# All windows null carries no information; returning it would print a
# heading with nothing under it.
cat > "$f" <<'JSON'
{"cachedUsageUtilization": {"fetchedAtMs": 1, "utilization": {"five_hour": null, "seven_day": null}}}
JSON
check "a reading with no windows is refused" "refused" "$(outcome)"

# One window present and one missing is still worth showing.
cat > "$f" <<JSON
{"cachedUsageUtilization": {"fetchedAtMs": $(( $(date +%s) * 1000 )),
  "utilization": {"five_hour": {"utilization": 32, "resets_at": null}, "seven_day": null}}}
JSON
check "a reading with one window is usable" "ok:0:1" "$(outcome)"

print -r -- '{not json' > "$f"
check "an unparseable file is refused" "refused" "$(outcome)"

rm -f "$f"
check "a missing file is refused" "refused" "$(outcome)"

print -r -- ""
print -r -- "_claude_acc_usage_lines, stale rendering:"

# A window whose reset has gone by gets no bar when the reading is stale.
past=$(_claude_acc_usage_lines $'5h\t97\t-120' 1)
if [[ "$past" == *"$(_msg usage_window_has_reset)"* && "$past" != *"█"* ]]; then
    print -r -- "  ok   a reset window is described, not drawn"
else
    print -r -- "  FAIL a reset window is described, not drawn: got '$past'"
    (( failures++ ))
fi

# Live output is untouched by any of this: the same line with stale unset
# still draws its bar.
live=$(_claude_acc_usage_lines $'5h\t97\t-120' 0)
if [[ "$live" == *"█"* ]]; then
    print -r -- "  ok   live output still draws the bar"
else
    print -r -- "  FAIL live output still draws the bar: got '$live'"
    (( failures++ ))
fi

# Still running, stale reading: the bar is drawn, because the figure is old
# but not from a window that has ended.
running=$(_claude_acc_usage_lines $'7d\t40\t3600' 1)
if [[ "$running" == *"█"* ]]; then
    print -r -- "  ok   a stale but still-running window keeps its bar"
else
    print -r -- "  FAIL a stale but still-running window keeps its bar: got '$running'"
    (( failures++ ))
fi

print -r -- ""
if (( failures )); then
    print -r -- "$failures check(s) failed"
    exit 1
fi
print -r -- "all checks passed"
