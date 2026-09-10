# Claude Desktop profiles

[← README](../README.md) · [Русский](ru/desktop.md)

Everything above is about the CLI. The **desktop app** has the same problem — one app, one signed-in account — and it turns out to have a clean answer.

The app is Electron, so it honours Chromium's `--user-data-dir`. Point it at a directory of our own and it gets a fully isolated profile: its own sign-in, its own settings, its own MCP servers. That is the same move this tool already makes for the CLI with `CLAUDE_CONFIG_DIR` — and it has one property the CLI accounts don't:

> **Profiles run side by side.** There is no "switch". Your work account and your personal account can both be open, in two windows, at the same time. The app takes no single-instance lock, and Chromium's lock lives inside each profile directory.

**Signing in has to happen with Claude closed.** Not a nicety — signing in finishes through a `claude://` link, and the system hands that to whichever instance is registered for the scheme, which is not the one that started the login. With another window open, the new profile simply never receives it. So:

1. Quit Claude.
2. `claude-acc desktop add <name>`, and sign in in the window that opens.
3. From then on, open as many profiles as you like — they run side by side.

`desktop add` refuses while Claude is running and names what's open, rather than launching a window that can't finish signing in. The same check applies to `desktop run` on a profile that isn't signed in yet; a profile that already is opens freely. `--force` overrides it.

```bash
claude-acc desktop add work      # create the profile and open Claude on it to sign in
claude-acc desktop add work -s   # ...and seed its MCP servers from the app's own profile
claude-acc desktop list          # profiles, and which account each is signed in as
claude-acc desktop usage         # ...plus their 5h / 7d rate-limit usage, live
claude-acc desktop run work      # open Claude on that profile again
claude-acc desktop clone-config work   # copy MCP servers into an existing profile
claude-acc desktop clone-runtime work  # clone the ~10.5 GB runtime — free on APFS
claude-acc desktop remove work   # delete the profile and everything in it
```

`desktop add` creates `~/.claude-switch/desktop/<name>/` and opens the app on it. The window comes up signed out — sign in there with the account this profile is for:

```
$ claude-acc desktop add work
Desktop profile 'work' created. Opening Claude on it...
It opens signed out — sign in there with the account for this profile.
The profile is fully isolated, so the app re-downloads its sandbox images into it — expect several GB.

Open it again later:  claude-acc desktop run work
```

```
$ claude-acc desktop list
Claude Desktop profiles:
    work  (signed out)
    ~/Library/…/Claude/  (the app's own profile)
```

Once signed in, that row carries the account it belongs to — see [`desktop usage`](#which-account-each-profile-is-signed-in-as-desktop-usage).

The last row is the app's own profile — the one you get when you open Claude from the Dock. Nothing here reads or writes it; it is listed so the picture is complete.

Your main instance is never quit, never touched, and never has its signed-in state copied around. That is the whole reason this approach is worth having: the alternative — quitting the app and swapping profile data on disk — mixes authentication state and triggers server-side re-authentication, which is exactly what the Windows tools in this space keep running into.

**Trade-offs, stated plainly:**

- **Disk.** Isolation is total, so a profile would re-download its whole ~10.5 GB runtime. `clone-runtime` makes that free on APFS — see below. Caches (~1.5 GB) are still per-profile.
- **MCP servers are per-profile.** A new profile starts with none — `-s` or `clone-config` brings them over, see below.
- **`--user-data-dir` is a Chromium switch, not a documented Claude Desktop feature.** This is how VS Code and most Electron apps are routinely run, so the risk is small — but if the app ever pins its own data directory unconditionally, this stops working.

### Where it works

| | Launching profiles | Which account a profile is on (`desktop usage`) |
|---|---|---|
| **macOS** | yes — `/Applications/Claude.app` or `~/Applications/` | yes |
| **Windows**, installed from [claude.com/download](https://claude.com/download) | yes — `%LOCALAPPDATA%\AnthropicClaude\` | not yet — the key lives in DPAPI, not the Keychain |
| **Windows**, installed from the Microsoft Store | **no** — see below | no |
| **Linux** ([official package](https://code.claude.com/docs/en/desktop-linux), beta) | yes — `claude-desktop` on `PATH` | not yet — libsecret / kwallet |

If the app is somewhere else — an unofficial Linux build, a non-standard install — point at it: `CLAUDE_ACC_DESKTOP_APP=/path/to/the/app`.

**The Microsoft Store build can't do this, and won't pretend to.** Its executable lives under `WindowsApps` and starts only through the Store's own activation, which is no way to pass a command-line switch; the package also redirects file paths, so a switch that did arrive wouldn't point where it says. Rather than launch it and quietly open your real profile while claiming otherwise, `desktop` says what's wrong and stops. The installer from claude.com/download works.

> Reported but unverified by us: on Windows, Cowork resolves its VM image relative to `%APPDATA%`, so a profile kept elsewhere may fail to start one, and only one Cowork VM runs at a time regardless. Plain chat is unaffected.

### Bringing MCP servers along (`clone-config`)

A profile's MCP servers and app preferences live in `claude_desktop_config.json` **inside the profile directory**, so a new profile starts with neither. Re-adding a docker MCP server by hand in every profile gets old fast — this is the desktop analog of [`clone-settings`](accounts.md#inheriting-claude-config) for CLI accounts:

```bash
claude-acc desktop add work -s               # seed at creation, from the app's own profile
claude-acc desktop clone-config work         # or seed an existing profile
claude-acc desktop clone-config work --from personal   # from another profile instead
```

```
$ claude-acc desktop clone-config work
MCP servers and preferences copied from ~/Library/…/Claude/.
Server definitions only — any that sign in separately will ask for that again in the new profile.
```

An existing config is **kept, not replaced** — it likely holds servers someone added by hand:

```
$ claude-acc desktop clone-config work
This profile already has a claude_desktop_config.json. Replace it with --force.
```

Two things worth knowing:

- **Definitions, not sessions.** An MCP server that authenticates on its own will ask for that again in the new profile — as it should, since the point of a separate profile is a separate identity.
- The file can hold server credentials, so it is copied with its mode intact (`0600` in the app's own profile) and via a staging file, so an interrupted copy can't leave half a config behind.

### Which account each profile is signed in as (`desktop usage`)

`desktop list` reads nothing but files, so it can only say whether a profile holds a credential. `desktop usage` goes further — it decrypts the profile's token and asks the API, giving you the email, the plan, and the same 5h / 7d bars [`usage`](identity.md#usage-tracking-usage) shows for CLI accounts:

```
$ claude-acc desktop usage
macOS will now ask for your login keychain password: reading a profile's account and usage means decrypting its token, and the key for that lives in the 'Claude Safe Storage' keychain entry. Declining only costs you this listing.

Claude Desktop usage:
    work  <work@company.com>  Max 20x
      5h  [██████░░░░░░░░░░░░░░]   32%  resets in 52m
      7d  [████████░░░░░░░░░░░░]   40%  resets in 5d 16h
```

It caches what it learns, so `desktop list` shows the email from then on without asking for anything:

```
$ claude-acc desktop list
Claude Desktop profiles:
    work  <work@company.com>  Max 20x  (signed in)
    ~/Library/…/Claude/  (the app's own profile)
```

**About that keychain prompt.** The desktop app stores its token the way every Chromium app does on macOS: encrypted with a key kept in the keychain entry `Claude Safe Storage`, whose access list names only the app itself. Reading it therefore asks you for your login keychain password — once, if you pick "Always Allow". That is a real thing to be asked for, so `desktop usage` says what it is about to do *before* the dialog appears rather than after, and nothing else in this tool ever touches that entry. Decline and you lose this one listing; everything else keeps working.

**Often you will not need it.** When the account a profile is signed into is also one of the accounts this tool manages, `desktop list` names it from that account's own `.claude.json` — a uuid match between two local files, no keychain prompt and no network:

```
$ claude-acc desktop list
Claude Desktop profiles:
    work  <work@company.com>  (signed in)
    ~/Library/…/Claude/  (the app's own profile)
```

No plan beside the email there: the plan comes from the profile API, and this row was answered without asking anyone anything.

When neither applies — nothing cached, and no account here signed in as that uuid — a profile shows the **uuid** itself, which sits in plaintext in the profile's own `config.json`:

```
$ claude-acc desktop list
Claude Desktop profiles:
    work  aa6c22d5…  (signed in)
```

Not an identity anyone recognises, but enough to see that two profiles are two different accounts.

> `desktop usage` is a Rust-CLI feature. Decrypting the token needs PBKDF2-HMAC-SHA1 and AES-128-CBC with an explicit key — stock macOS ships LibreSSL, whose `openssl` has no `kdf` subcommand, so the shell script would need Homebrew's OpenSSL or Python to do it. It shows the uuid instead.

### Not paying for the runtime twice (`clone-runtime`)

Most of a profile's weight is components the app downloads and then only reads:

| Size | |
|---|---|
| ~10 GB | `vm_bundles/claudevm.bundle/` — Cowork sandbox images |
| 250 MB | `claude-code-vm/<version>/` |
| 220 MB | `claude-code/<version>/` |

Identical in every profile, and a new profile fetches its own copy of all of it. On APFS it doesn't have to: `clone-runtime` clones them **copy-on-write**, so each profile gets fully independent files that share blocks with the original until one of them is written.

```
$ claude-acc desktop clone-runtime work
Cloned 13 runtime component(s), 10.5 GB logical, from ~/Library/…/Claude/.
Disk actually used: 12 KB.
```

Ten and a half gigabytes, in a third of a second, for twelve kilobytes of directory metadata. `--from <profile>` clones from another profile instead of the app's own; `--force` replaces a runtime the profile already has.

**Why clone rather than share.** The Windows tools in this space point every profile at one `vm_bundles/` directory. They can, because they quit the app before switching, so only one instance ever touches those files. Ours run at the same time — two live VMs writing to one image is corruption, not a saving. A clone has no shared writer at all: writing to one leaves the other byte-for-byte intact, and only the changed blocks get allocated.

**What is not cloned, deliberately.** The sandbox bundle mixes downloads with per-VM identity, and only the first kind may travel:

| Cloned | Left for the app |
|---|---|
| `rootfs.img`, `vmlinuz`, `initrd*` — the images | `machineIdentifier`, `macAddress`, `gvisorMacAddress`, `vmIP` — two live VMs sharing a MAC address is a collision, not a saving |
| `.*.origin` — which image set they came from, so the app doesn't refetch | `sessiondata.img`, `efivars.fd` — this profile's own state |
| `claude-code/<version>/`, `claude-code-vm/<version>/` — whole, they are a download and a `.verified` marker | `Cache/` (1.2 GB), `Code Cache/` (346 MB) — live Chromium caches, written continuously and refilled by the app |

Two caveats, both stated by the command itself:

- **Cross-filesystem clones are refused, not performed.** `cp -c` silently falls back to a real copy when it can't clone, which would spend 10 GB to save 10 GB. If the profile isn't on the same filesystem as the source, the command says so and does nothing.
- **Whether the app accepts a pre-seeded runtime is untested.** The filesystem mechanics are verified; the app's reaction is not — nobody has run Cowork in a profile seeded this way. If it misbehaves, delete that profile's `vm_bundles/`, `claude-code/` and `claude-code-vm/`, and the app fetches its own. Reports either way are welcome in [#92](https://github.com/Nemo-Illusionist/claude-code-account-switcher/issues/92).

Attachments are unaffected by any of this: Cowork keeps user files at the path in `coworkUserFilesPath`, which lives **outside** the profile and travels with [`clone-config`](#bringing-mcp-servers-along-clone-config), so a new profile points at the files you already have rather than a copy of them.
