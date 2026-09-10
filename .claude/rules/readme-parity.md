---
paths:
  - "README*.md"
  - "docs/**/*.md"
---

# Every language, or none

The `README*.md` files and the guides under `docs/` are peers — no original,
no translation allowed to lag. Change one, change **all of them, in the same
commit**: same sections, same order, same placement, same command table rows.

Today that is `README.md` + `docs/*.md` (English) and `README.ru.md` +
`docs/ru/*.md` (Russian). Check what is actually on disk rather than trusting
that list — this rule applies to whatever matches, including a language added
after it was written.

## The two trees mirror each other file for file

`docs/<topic>.md` has a counterpart at `docs/ru/<topic>.md`, with the same
sections in the same order. A new guide means both files, and both index rows
in the "Guides" table of each README.

Links between guides are relative and stay inside their own language:
`docs/desktop.md` links to `accounts.md`, `docs/ru/desktop.md` links to the
`accounts.md` beside it. A cross-language link belongs only in the header
line each file starts with.

A docs-only catch-up PR afterwards is not a fix: between the two, a released
version shipped with a README that lied to some of its readers.

## Translated console examples must be real output

Never hand-translate a sample block from the English README. Run the command
under that language and paste what it actually printed:

```sh
CLAUDE_ACC_LANG=ru claude-acc sessions
```

`CLAUDE_ACC_LANG` takes the same codes as `src/i18n.rs` knows. Hand-translated
samples drift from the real strings the moment one changes, and nothing in CI
will ever catch it.

## Checklist

- [ ] The new or edited section exists in **every** language, same position —
      `README.md` / `README.ru.md`, or `docs/<topic>.md` / `docs/ru/<topic>.md`
- [ ] A new guide is linked from the "Guides" table in both READMEs
- [ ] Command table updated in each README, if a command changed
- [ ] Every translated sample block came from a real run in that language
- [ ] Anchors in links exist in that file's own language
      (`#ide-integration` vs `#интеграция-с-ide`), and every relative link
      resolves — the guides are separate files now, so a moved section
      breaks a link instead of just scrolling to the wrong place
