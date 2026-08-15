//! Command-head matching: trim, ASCII case-fold, `_` → `-` on the token only.

/// First whitespace-separated token after trim.
pub fn command_head(text: &str) -> &str {
    text.split_whitespace().next().unwrap_or("")
}

/// ASCII-lowercase and `_` → `-` for a single command token (not args).
pub fn normalize_token(token: &str) -> String {
    token.to_ascii_lowercase().replace('_', "-")
}

/// Normalize the command head only; args are not rewritten.
pub fn normalize_command_head(text: &str) -> String {
    normalize_token(command_head(text))
}

/// Full-string normalize for exact (no-arg) commands.
///
/// Trims, ASCII-lowercases, replaces `_` with `-`, and collapses internal
/// ASCII whitespace so `!help  thread` matches the `!help thread` alias.
pub fn normalize_exact(text: &str) -> String {
    let lowered = text.trim().to_ascii_lowercase().replace('_', "-");
    let mut out = String::with_capacity(lowered.len());
    let mut prev_space = false;
    for c in lowered.chars() {
        if c.is_ascii_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(c);
            prev_space = false;
        }
    }
    out
}

/// Exact command match after [`normalize_exact`] (avoids `!translation` matching `!translation-on`).
pub fn is_exact_command(text: &str, command: &str) -> bool {
    normalize_exact(text) == normalize_exact(command)
}

pub fn is_exact_command_any(text: &str, commands: &[&str]) -> bool {
    let n = normalize_exact(text);
    commands.iter().any(|c| normalize_exact(c) == n)
}

/// True when the command head equals `prefix` after normalize (args allowed).
pub fn starts_with_word(text: &str, prefix: &str) -> bool {
    normalize_command_head(text) == normalize_token(prefix.trim())
}

pub fn starts_with_word_any(text: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|p| starts_with_word(text, p))
}

/// Remainder after a matching command head, with original args (not rewritten).
pub fn strip_word_prefix<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    let t = text.trim();
    let head = command_head(t);
    if normalize_token(head) != normalize_token(prefix.trim()) {
        return None;
    }
    if t.len() == head.len() {
        return Some("");
    }
    Some(t[head.len()..].trim())
}

pub fn strip_prefix_list<'a>(text: &'a str, prefixes: &[&str]) -> Option<&'a str> {
    prefixes.iter().find_map(|p| strip_word_prefix(text, p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_threads_head_normalizes_case_and_underscore() {
        assert_eq!(normalize_command_head("!Help-Threads"), "!help-threads");
        assert_eq!(normalize_command_head("!help_thread"), "!help-thread");
        assert_eq!(normalize_command_head("  !HELP_THREADS  "), "!help-threads");
    }

    #[test]
    fn args_keep_underscores() {
        assert_eq!(
            normalize_command_head("!translate-me-on es_MX"),
            "!translate-me-on"
        );
        assert_eq!(
            strip_word_prefix("!translate-me-on es_MX", "!translate-me-on"),
            Some("es_MX")
        );
        assert_eq!(
            strip_word_prefix("!Translate_Me_On es_MX", "!translate-me-on"),
            Some("es_MX")
        );
    }

    #[test]
    fn translation_is_not_prefix_of_translation_on() {
        assert!(is_exact_command("!translation", "!translation"));
        assert!(!is_exact_command("!translation-on es en", "!translation"));
        assert!(!starts_with_word("!translation-on es en", "!translation"));
    }

    #[test]
    fn leading_trailing_whitespace_still_matches() {
        assert!(is_exact_command("  !help  ", "!help"));
        assert!(is_exact_command_any(
            "\t!help-thread\n",
            &["!help-threads", "!help-thread"]
        ));
        assert!(starts_with_word(
            "  !translate-me-on es en  ",
            "!translate-me-on"
        ));
    }

    #[test]
    fn space_alias_matches_after_collapse() {
        assert!(is_exact_command("!help thread", "!help thread"));
        assert!(is_exact_command("!help  thread", "!help thread"));
        assert!(is_exact_command("!Help Thread", "!help thread"));
        assert!(!is_exact_command("!help extra", "!help thread"));
        assert!(!is_exact_command("!help extra", "!help"));
    }

    #[test]
    fn starts_with_word_rejects_glued_suffix() {
        assert!(starts_with_word("!translate-me-on", "!translate-me-on"));
        assert!(starts_with_word(
            "!translate-me-on es en",
            "!translate-me-on"
        ));
        assert!(!starts_with_word("!translate-me-onx", "!translate-me-on"));
        assert!(!starts_with_word("!help-threads", "!help"));
    }

    #[test]
    fn strip_prefix_list_picks_matching_head() {
        let prefixes = ["!translate-me-thread", "!translate-me-threads"];
        assert_eq!(
            strip_prefix_list("!translate-me-threads es", &prefixes),
            Some("es")
        );
        assert_eq!(
            strip_prefix_list("!translate-me-thread", &prefixes),
            Some("")
        );
        assert!(strip_prefix_list("!translate-me-on es en", &prefixes).is_none());
    }
}
