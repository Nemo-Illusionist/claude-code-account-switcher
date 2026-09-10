# Identities: `doctor`, `lock`, `usage`

[← README](../README.md) · [Русский](ru/identity.md)

Which Anthropic account is actually behind each config dir, how to pin it,
and how much of its rate limit is gone.

## Which account a config dir is signed in as (`doctor`)

`claude-acc add` and `claude-acc login` both run `claude auth login` under a per-account `CLAUDE_CONFIG_DIR`. Whatever Anthropic account you sign in with becomes the identity for that directory — and there's no built-in surface to see which account is actually behind a given config dir. If you accidentally log in with the wrong identity (browser auto-fill, a stale tab), the switch is silent: rate limits, conversation history, and billing leak across what you thought were isolated accounts.

`claude-acc doctor` reads each account's OAuth token from the macOS Keychain (with a `.credentials.json` fallback for non-Keychain installs), calls `https://api.anthropic.com/api/oauth/profile`, and prints the live email, plan, and UUID:

```
$ claude-acc doctor
Auditing 2 account(s):
  ✓ work      alice@anthropic.com  Max 20x  uuid=aa6c22d5-…
  ? personal  no token (run: claude-acc login personal)

1 of 2 accounts healthy.
```

The plan label (`Max 20x` / `Max` / `Pro`) is derived from the profile's tier flags and `rate_limit_tier`; it's omitted for accounts with no recognizable subscription.

It's purely a read-only audit — nothing is intercepted, no `claude` invocation is gated. Run it whenever you want to confirm a config dir is bound to the identity you expect. Requires `security`, `curl`, `jq`, and `shasum` (all preinstalled on macOS); the Rust binary uses native `serde_json` and `sha2` instead and only shells out to `security` and `curl`.

`doctor` also caches the result (email, plan, UUID) to `~/.claude-switch/accounts/<name>/.account-info.json` so `list`, `usage`, `status`, and `default` can show the identity next to each account without re-hitting the API:

```
$ claude-acc list
Claude Code accounts:
  ★ work       (default)  alice@anthropic.com   Max 20x  3d ago
    personal              bob@anthropic.com     Pro      1h ago *
    ~/.claude/            charlie@personal.com  Max 5x   3d ago    (standard)

$ claude-acc status
Active account: work <alice@anthropic.com> (linked to my-project)

$ claude-acc default
Default: work <alice@anthropic.com>
```

`doctor` audits the standard `~/.claude/` config too (the unmanaged identity used when no link / configured default applies). Its cache lives at `~/.claude-switch/default.account-info.json`. The `~/.claude/` row appears in `list` only after you've actually logged into Claude Code with the standard config (or after `doctor` has cached an identity for it).

For scripting, `claude-acc doctor --json` emits the same audit information as a single JSON document — and `claude-acc whoami` prints just the email (or account name fallback) of the active account, suitable for shell prompts:

```bash
# Use in a prompt:
PS1='[$(claude-acc whoami)] \$ '

# Or in a script:
case "$(claude-acc whoami)" in
    alice@anthropic.com) echo "work" ;;
    *)                   echo "other" ;;
esac
```

The `*` after an email means the OAuth token has rotated since the cache was written. Most often this is a routine OAuth refresh (identity unchanged) — but if you ran `claude auth login` directly between `doctor` runs, this is your reminder to re-verify. Run `claude-acc doctor` to refresh the cache.

### One login, several setups

Linking two account dirs to the **same** Anthropic login is a perfectly valid setup — it lets you keep separate global configs (different `CLAUDE.md`, plugins, agents, MCP servers, output styles) under a single subscription, and switch between them per directory. When `doctor` sees accounts that resolve to the same identity it cross-references them with `↔` so the overlap is intentional and visible, not a surprise:

```
$ claude-acc doctor
Auditing 2 account(s):
  ✓ minimal  alice@anthropic.com  Max 20x  uuid=aa6c22d5-…  ↔ same identity as full
  ✓ full     alice@anthropic.com  Max 20x  uuid=aa6c22d5-…  ↔ same identity as minimal

All accounts healthy.
```

This is just a note, never an error — both accounts share the login (and therefore the same usage limits), only their local config differs.

> **macOS only for now.** The Keychain hashing scheme is reverse-engineered from Claude Code's internals, so non-macOS platforms (where Claude Code uses libsecret / Credential Manager) aren't covered yet.

## Pinning an account to an identity (`lock`)

`doctor` tells you which account a directory is signed in as. It cannot tell
you whether that is the account it *should* be — you have to remember. A
re-login is where that goes wrong: the OAuth flow never asks which account
you meant, so signing in with the wrong browser session quietly rebinds a
config dir, and every session after that goes to the wrong place.

`lock` records the answer:

```bash
claude-acc lock work            # pin work to whoever it is signed in as now
claude-acc lock work --force    # accept a new identity on purpose
claude-acc lock default         # the standard ~/.claude account too
```

`add` and `login` pin automatically the first time an account signs in, so
this is for accounts created before the pin existed, and for the deliberate
re-pin. **Re-logging in never moves an existing pin** — that swap is exactly
what the pin exists to report.

`doctor` then compares the two, and says nothing at all while they agree:

```
$ claude-acc doctor
Auditing 2 account(s):
  ✓ work        alice@corp.com  uuid=a72fe3df-3623-46b0-89ab-85770432d3fd  ⚠ DRIFT: pinned to bob@personal.com (aa6c22d5-f7d1-4ac1-bb29-22abc90481c1), signed in as alice@corp.com (a72fe3df-3623-46b0-89ab-85770432d3fd)
  ✓ ~/.claude/  bob@personal.com  Max 20x  uuid=aa6c22d5-f7d1-4ac1-bb29-22abc90481c1  (standard)

Drift means this directory is signed in as an account it was not pinned to — work done here would go to the wrong one. Put it back with `claude-acc login <name>`, or accept the new identity with `claude-acc lock <name> --force`.
```

Drift makes `doctor` exit non-zero, so a shell prompt or a CI step can gate
on it.

**The comparison costs nothing.** It reads Claude Code's own record of the
signed-in account — a small JSON file beside each config dir — so there is no
keychain prompt and no network call, unlike the profile lookup the rest of
`doctor` does. That is what makes it cheap enough to run often.

**Only the UUID decides.** An email can change on one account, and two
accounts can share a display name; comparing on anything softer would report
drift that isn't there and miss drift that is.

**What it does not do:** nothing is blocked. `lock` and `doctor` report; they
never stand between you and `claude`. Whether a wrong identity should refuse
to launch at all is [#16](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/16), still open.

The pin lives in `~/.claude-switch/accounts/<name>/.identity-lock.json` —
except the standard account's, which goes to
`~/.claude-switch/default.identity-lock.json` rather than inside `~/.claude/`,
which belongs to Claude Code.

## How much rate limit is left (`usage`)

`claude-acc usage` shows how much of each account's rate limit you've burned, so you can pick a fresh account before you hit a wall. For every account (and the standard `~/.claude/` if logged in) it reads the OAuth token, calls `https://api.anthropic.com/api/oauth/usage`, and renders the **5-hour** and **7-day** windows with a bar, a percentage, and the time until each resets:

```
$ claude-acc usage
Claude Code usage:
  ★ work  <alice@anthropic.com>  Max 20x
      5h  [████████░░░░░░░░░░░░]   42%  resets in 2h 14m
      7d  [██░░░░░░░░░░░░░░░░░░]   11%  resets in 5d 17h
    personal  <bob@anthropic.com>  Pro
      5h  [░░░░░░░░░░░░░░░░░░░░]    0%  available now
      7d  [░░░░░░░░░░░░░░░░░░░░]    0%  resets in 6d 3h
```

Unlike `doctor`, the usage figures are always a live fetch — usage is volatile, so nothing is cached. The email/plan next to each account come from `doctor`'s cache, so run `claude-acc doctor` once to populate them. Accounts with no token show `no token (run: claude-acc login <name>)`; an unreachable API shows `token present, but API unreachable`. Same dependencies and platform caveat as `doctor` (`security`, `curl`, `jq`, `shasum`; macOS only for now).
