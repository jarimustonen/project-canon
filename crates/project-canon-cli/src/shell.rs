//! Shell-safety helpers shared by the verbs that **print** commands for a human to run
//! (`new`'s bootstrap hook plan, `review`'s staged `issuectl` commands). These verbs never
//! execute the commands they render — but the rendered text must still be safe to paste into a
//! shell, so every interpolated value is single-quoted here.

/// POSIX single-quote a string for safe interpolation into a printed shell command. Wrapping in
/// single quotes disables every shell expansion; an embedded single quote is closed, escaped, and
/// reopened (`'\''`). Rust's `{:?}` is NOT a shell escaper (it double-quotes, leaving `$`/backtick
/// active, and renders non-ASCII as `\u{…}`), so it must never be used to build a command string.
pub fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_in_single_quotes() {
        assert_eq!(shell_quote("plain"), "'plain'");
        assert_eq!(shell_quote("/abs/path"), "'/abs/path'");
    }

    #[test]
    fn neutralizes_shell_metacharacters() {
        // `$`, backtick, `;`, `&&`, and spaces are all inert inside single quotes.
        assert_eq!(
            shell_quote("a b; rm -rf $HOME `id`"),
            "'a b; rm -rf $HOME `id`'"
        );
    }

    #[test]
    fn closes_and_reopens_for_an_embedded_single_quote() {
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
        // Two adjacent quotes each get the escape treatment.
        assert_eq!(shell_quote("''"), "''\\'''\\'''");
    }
}
