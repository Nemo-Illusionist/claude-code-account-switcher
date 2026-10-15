#!/usr/bin/env zsh
#
# An apostrophe in the install path must not break the generated wrapper.
#
# `_claude_ensure_wrapper` writes the path to this script into a heredoc
# between single quotes. `/Users/O'Brien` is an ordinary home directory, and
# an apostrophe there closed the quoted string early — leaving a wrapper zsh
# refuses to parse, with nothing saying so until the next `claude` launch
# (#101). The Rust side had the same flaw in `init`, `install` and the IDE
# wrapper; see `src/quoting.rs`.
#
# The assertion is zsh's own parser, not a string comparison: the question is
# whether the file can be run, and only the shell can answer that.
#
# Run: zsh tests/shell/apostrophe_path.zsh

emulate -L zsh
set -u

typeset -i failures=0
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT

repo="${0:A:h:h:h}"

# A copy of the script under a directory whose name contains an apostrophe.
# The real installation is never touched.
srcdir="$scratch/O'Brien"
mkdir -p "$srcdir"
cp "$repo/claude-switch.sh" "$srcdir/claude-switch.sh"

print -r -- "_claude_acc_sh_quote:"

# Sourced from the apostrophe path so CLAUDE_SWITCH_SCRIPT carries one.
export HOME="$scratch/home"
mkdir -p "$HOME/.claude"
export CLAUDE_ACC_LANG=en
source "$srcdir/claude-switch.sh" >/dev/null 2>&1

check() {
    local label="$1" expected="$2" actual="$3"
    if [[ "$expected" == "$actual" ]]; then
        print -r -- "  ok   $label"
    else
        print -r -- "  FAIL $label: expected '$expected', got '$actual'"
        (( failures++ ))
    fi
}

check "a plain path is unchanged" \
    "/Users/x/bin" "$(_claude_acc_sh_quote /Users/x/bin)"
check "an apostrophe ends and reopens the quoted run" \
    "/Users/O'\\''Brien/x" "$(_claude_acc_sh_quote "/Users/O'Brien/x")"
check "every apostrophe is escaped" \
    "a'\\''b'\\''c" "$(_claude_acc_sh_quote "a'b'c")"
# A backslash is literal inside POSIX single quotes, so touching it would
# change the path rather than protect it.
check "a backslash is left alone" 'a\b' "$(_claude_acc_sh_quote 'a\b')"

# The escaped form has to survive a real round trip through the shell, not
# merely look right: embed it where the wrapper embeds it and read it back.
raw="/Users/O'Brien/x"
print -r -- "p='$(_claude_acc_sh_quote "$raw")'" > "$scratch/rt.zsh"
print -r -- 'print -r -- "$p"' >> "$scratch/rt.zsh"
if zsh -n "$scratch/rt.zsh" 2>/dev/null && [[ "$(zsh "$scratch/rt.zsh")" == "$raw" ]]; then
    print -r -- "  ok   the escaped form round-trips through zsh"
else
    print -r -- "  FAIL the escaped form round-trips through zsh"
    (( failures++ ))
fi

print -r -- ""
print -r -- "the generated wrapper:"

wrapper="$CLAUDE_SWITCH_BIN/claude"

if [[ -f "$wrapper" ]]; then
    print -r -- "  ok   a wrapper was generated"
else
    print -r -- "  FAIL no wrapper was generated at $wrapper"
    (( failures++ ))
fi

# Guard the premise: if the path did not reach the wrapper at all, the parse
# check below would pass while testing nothing.
if grep -q "O'" "$wrapper" 2>/dev/null; then
    print -r -- "  ok   the apostrophe path reached the wrapper"
else
    print -r -- "  FAIL the wrapper does not mention the apostrophe path"
    (( failures++ ))
fi

if zsh -n "$wrapper" 2>/dev/null; then
    print -r -- "  ok   zsh parses the generated wrapper"
else
    print -r -- "  FAIL zsh cannot parse the generated wrapper:"
    zsh -n "$wrapper" 2>&1 | head -2 | sed 's/^/       /'
    (( failures++ ))
fi

# Parsing is necessary but not sufficient: a wrongly escaped path can parse
# and still point somewhere else. So check that the path the wrapper will
# source is a file that exists — without sourcing it. The wrapper `exec`s
# `claude` at the end, so running it from a test hangs waiting on a session.
#
# The quoted literal is handed back to zsh to unquote rather than being
# un-escaped here: re-implementing the shell's own rule in the test is how a
# test ends up agreeing with a wrong implementation.
quoted=$(grep -o "source '.*claude-switch\.sh'" "$wrapper" 2>/dev/null | head -1)
print -r -- "p=${quoted#source }" > "$scratch/unquote.zsh"
print -r -- 'print -r -- "$p"' >> "$scratch/unquote.zsh"
sourced=$(zsh "$scratch/unquote.zsh" 2>/dev/null)

if [[ -n "$sourced" && -f "$sourced" ]]; then
    print -r -- "  ok   the path it sources resolves to a real file"
else
    print -r -- "  FAIL the sourced path does not resolve: '$sourced'"
    (( failures++ ))
fi

# `:A` on both sides: the script records a resolved path, and $TMPDIR on
# macOS is itself a symlink (/var → /private/var), so the raw strings differ
# for reasons that have nothing to do with quoting.
if [[ "${sourced:A}" == "${srcdir:A}/claude-switch.sh" ]]; then
    print -r -- "  ok   and it is the script we sourced, apostrophe intact"
else
    print -r -- "  FAIL wrong script: '${sourced:A}' != '${srcdir:A}/claude-switch.sh'"
    (( failures++ ))
fi

print -r -- ""
if (( failures )); then
    print -r -- "$failures check(s) failed"
    exit 1
fi
print -r -- "all checks passed"
