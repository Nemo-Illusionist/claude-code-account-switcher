---
paths:
  - "README.md"
  - "README.ru.md"
---

# Both READMEs, or neither

`README.md` (English) and `README.ru.md` (Russian) are peers, not an original
and a translation allowed to lag. Change one, change the other **in the same
commit** — same sections, same order, same placement, same command table rows.

A docs-only catch-up PR afterwards is not a fix: between the two, a released
version shipped with a README that lied to half its readers.

## Russian console examples must be real output

Never hand-translate a sample block from the English README. Run the command
with `CLAUDE_ACC_LANG=ru` and paste what it actually printed.

Translated-by-hand samples drift from `src/i18n.rs` the moment a string
changes, and nothing in CI will ever catch it.

## Checklist

- [ ] Same new/edited section exists in both files, in the same position
- [ ] Command table updated in both, if a command changed
- [ ] Every `ru` sample block came from a real `CLAUDE_ACC_LANG=ru` run
- [ ] Anchors used in links exist in that file's own language
      (`#ide-integration` vs `#интеграция-с-ide`)
