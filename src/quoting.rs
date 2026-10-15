//! Putting a filesystem path inside a quoted string in generated code.
//!
//! This tool writes shell into three places someone else's shell then parses:
//! the `claude` wrappers, the `init` templates, and the `eval` line added to
//! an rc file. All of them wrap our own binary's path in single quotes, and
//! `/Users/O'Brien` is an ordinary home directory — so the path closes the
//! quoted string early and leaves a file the shell cannot parse (#101).
//!
//! Both failures are silent when they happen and loud much later, somewhere
//! that does not point back here: `install` reports success, and then every
//! IDE launch fails and every shell start greets the user with a syntax
//! error.
//!
//! The two rules are different and must not be mixed up:
//!
//! | | |
//! |---|---|
//! | POSIX `sh`/`bash`/`zsh` | nothing is special inside `'...'`, not even `\`, so a quote is escaped by *leaving* the string: `'\''` |
//! | PowerShell | a quote inside `'...'` is escaped by doubling it: `''` |

/// Body of a POSIX single-quoted string holding `raw`.
///
/// End the quoted run, emit an escaped quote, start a new one. The result is
/// meant to be dropped between the quotes the template already has.
pub fn sh_single_quoted(raw: &str) -> String {
    raw.replace('\'', "'\\''")
}

/// Body of a PowerShell single-quoted string holding `raw`.
///
/// PowerShell has no backslash escapes; a literal quote is written twice.
/// Using the POSIX form here would emit `'\''`, which ends the string and
/// leaves a stray backslash — broken in a different way rather than fixed.
pub fn pwsh_single_quoted(raw: &str) -> String {
    raw.replace('\'', "''")
}

/// Whether `raw` can be represented in a *line* of generated shell at all.
///
/// Quoting handles every character except a newline: the `eval` line appended
/// to an rc file is found and replaced by line, and `init`'s own templates are
/// line-oriented too. A path containing one would be split across lines and
/// no amount of escaping puts it back together, so the caller refuses rather
/// than writing something broken.
pub fn is_single_line(raw: &str) -> bool {
    !raw.contains('\n') && !raw.contains('\r')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_without_quotes_is_returned_unchanged() {
        for p in ["/plain/path", "", "/Users/x/.claude-switch/bin/claude-acc"] {
            assert_eq!(sh_single_quoted(p), p);
            assert_eq!(pwsh_single_quoted(p), p);
        }
    }

    // Regression (#101): `/Users/O'Brien` is an ordinary home directory, and
    // every generated file pasted it between quotes unescaped.
    #[test]
    fn an_apostrophe_is_escaped_by_each_shells_own_rule() {
        assert_eq!(
            sh_single_quoted("/Users/O'Brien/bin"),
            "/Users/O'\\''Brien/bin"
        );
        assert_eq!(
            pwsh_single_quoted("/Users/O'Brien/bin"),
            "/Users/O''Brien/bin"
        );
    }

    // The two rules produce different output for the same input, which is the
    // whole reason there are two functions. A single shared helper would be
    // wrong for one of the three templates.
    #[test]
    fn the_two_rules_do_not_agree_and_must_not_be_swapped() {
        let raw = "/Users/O'Brien";
        assert_ne!(sh_single_quoted(raw), pwsh_single_quoted(raw));
    }

    #[test]
    fn several_quotes_are_each_escaped() {
        assert_eq!(sh_single_quoted("a'b'c"), "a'\\''b'\\''c");
        assert_eq!(pwsh_single_quoted("a'b'c"), "a''b''c");
    }

    // A backslash is literal inside POSIX single quotes, so it must survive
    // untouched — "escaping" it would change the path.
    #[test]
    fn a_backslash_is_left_alone() {
        assert_eq!(sh_single_quoted(r"a\b"), r"a\b");
        assert_eq!(pwsh_single_quoted(r"a\b"), r"a\b");
    }

    // A quote adjacent to a backslash is the case where getting the POSIX
    // rule subtly wrong (e.g. `\'`) still parses but yields the wrong path.
    #[test]
    fn a_quote_next_to_a_backslash_closes_and_reopens_cleanly() {
        assert_eq!(sh_single_quoted(r"a\'b"), "a\\'\\''b");
    }

    #[test]
    fn a_newline_is_not_representable_in_a_generated_line() {
        assert!(is_single_line("/Users/x/bin"));
        assert!(is_single_line("/Users/O'Brien/bin"));
        assert!(!is_single_line("/Users/two\nlines/bin"));
        assert!(!is_single_line("/Users/carriage\rreturn/bin"));
    }
}
