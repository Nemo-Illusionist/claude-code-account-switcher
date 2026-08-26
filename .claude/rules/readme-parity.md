---
paths:
  - "README*.md"
---

# Every README, or none

The `README*.md` files are peers — no original, no translation allowed to lag.
Change one, change **all of them, in the same commit**: same sections, same
order, same placement, same command table rows.

Today that is `README.md` (English) and `README.ru.md` (Russian). Check what
is actually on disk rather than trusting that list — this rule applies to
whatever `README*.md` matches, including a language added after it was
written.

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

- [ ] The new or edited section exists in **every** `README*.md`, same position
- [ ] Command table updated in each, if a command changed
- [ ] Every translated sample block came from a real run in that language
- [ ] Anchors in links exist in that file's own language
      (`#ide-integration` vs `#интеграция-с-ide`)
