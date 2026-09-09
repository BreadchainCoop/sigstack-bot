//! Language codes, names, and flag emojis for translation commands.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Language {
    pub code: &'static str,
    pub name: &'static str,
    pub flag: &'static str,
}

/// Full supported language catalog (Language Threads, Bilingual Threads, manual `!translate`).
pub const ALL_LANGUAGES: &[Language] = &[
    Language {
        code: "en",
        name: "English",
        flag: "🇺🇸",
    },
    Language {
        code: "es",
        name: "Spanish",
        flag: "🇪🇸",
    },
    Language {
        code: "fr",
        name: "French",
        flag: "🇫🇷",
    },
    Language {
        code: "de",
        name: "German",
        flag: "🇩🇪",
    },
    Language {
        code: "it",
        name: "Italian",
        flag: "🇮🇹",
    },
    Language {
        code: "pt",
        name: "Portuguese",
        flag: "🇵🇹",
    },
    Language {
        code: "ru",
        name: "Russian",
        flag: "🇷🇺",
    },
    Language {
        code: "zh",
        name: "Chinese",
        flag: "🇨🇳",
    },
    Language {
        code: "ja",
        name: "Japanese",
        flag: "🇯🇵",
    },
    Language {
        code: "ko",
        name: "Korean",
        flag: "🇰🇷",
    },
    Language {
        code: "ar",
        name: "Arabic",
        flag: "🇸🇦",
    },
    Language {
        code: "hi",
        name: "Hindi",
        flag: "🇮🇳",
    },
    Language {
        code: "bn",
        name: "Bengali",
        flag: "🇧🇩",
    },
    Language {
        code: "nl",
        name: "Dutch",
        flag: "🇳🇱",
    },
    Language {
        code: "pl",
        name: "Polish",
        flag: "🇵🇱",
    },
    Language {
        code: "tr",
        name: "Turkish",
        flag: "🇹🇷",
    },
    Language {
        code: "vi",
        name: "Vietnamese",
        flag: "🇻🇳",
    },
    Language {
        code: "th",
        name: "Thai",
        flag: "🇹🇭",
    },
    Language {
        code: "id",
        name: "Indonesian",
        flag: "🇮🇩",
    },
    Language {
        code: "uk",
        name: "Ukrainian",
        flag: "🇺🇦",
    },
    Language {
        code: "sv",
        name: "Swedish",
        flag: "🇸🇪",
    },
    Language {
        code: "cs",
        name: "Czech",
        flag: "🇨🇿",
    },
    Language {
        code: "el",
        name: "Greek",
        flag: "🇬🇷",
    },
    Language {
        code: "he",
        name: "Hebrew",
        flag: "🇮🇱",
    },
    Language {
        code: "ro",
        name: "Romanian",
        flag: "🇷🇴",
    },
    Language {
        code: "hu",
        name: "Hungarian",
        flag: "🇭🇺",
    },
    Language {
        code: "fi",
        name: "Finnish",
        flag: "🇫🇮",
    },
    Language {
        code: "da",
        name: "Danish",
        flag: "🇩🇰",
    },
    Language {
        code: "no",
        name: "Norwegian",
        flag: "🇳🇴",
    },
    Language {
        code: "fa",
        name: "Persian",
        flag: "🇮🇷",
    },
    Language {
        code: "eu",
        name: "Basque",
        flag: "🇪🇸",
    },
    Language {
        code: "sw",
        name: "Swahili",
        flag: "🇰🇪",
    },
];

/// ISO codes excluded from in-chat auto-translate (`whatlang` does not detect them).
const IN_CHAT_AUTO_EXCLUDED: &[&str] = &["eu", "sw"];

/// Whether `code` is in the in-chat auto-translate catalog (`!translate-all-on` / `!translate-me-on`).
pub fn is_in_chat_auto_language(code: &str) -> bool {
    ALL_LANGUAGES.iter().any(|lang| lang.code == code) && !IN_CHAT_AUTO_EXCLUDED.contains(&code)
}

/// Languages supported for in-chat auto-translate (subset of [`ALL_LANGUAGES`]).
pub fn in_chat_auto_languages() -> impl Iterator<Item = &'static Language> {
    ALL_LANGUAGES
        .iter()
        .filter(|lang| is_in_chat_auto_language(lang.code))
}

/// Resolve a language token for in-chat auto-translate commands.
pub fn resolve_in_chat_auto_language(input: &str) -> Option<&'static Language> {
    resolve_language(input).filter(|lang| is_in_chat_auto_language(lang.code))
}

/// Resolve a user-provided language token (ISO code or common name).
pub fn resolve_language(input: &str) -> Option<&'static Language> {
    let normalized = input.trim().to_lowercase();
    if normalized.is_empty() {
        return None;
    }

    ALL_LANGUAGES
        .iter()
        .find(|lang| lang.code == normalized)
        .or_else(|| match normalized.as_str() {
            "english" => Some(&ALL_LANGUAGES[0]),
            "spanish" | "español" | "espanol" => Some(&ALL_LANGUAGES[1]),
            "french" | "français" | "francais" => Some(&ALL_LANGUAGES[2]),
            "german" | "deutsch" => Some(&ALL_LANGUAGES[3]),
            "italian" | "italiano" => Some(&ALL_LANGUAGES[4]),
            "portuguese" | "português" | "portugues" => Some(&ALL_LANGUAGES[5]),
            "russian" | "русский" => Some(&ALL_LANGUAGES[6]),
            "chinese" | "mandarin" => Some(&ALL_LANGUAGES[7]),
            "japanese" => Some(&ALL_LANGUAGES[8]),
            "korean" => Some(&ALL_LANGUAGES[9]),
            "arabic" => Some(&ALL_LANGUAGES[10]),
            "hindi" => Some(&ALL_LANGUAGES[11]),
            "bengali" => Some(&ALL_LANGUAGES[12]),
            "dutch" => Some(&ALL_LANGUAGES[13]),
            "polish" => Some(&ALL_LANGUAGES[14]),
            "turkish" => Some(&ALL_LANGUAGES[15]),
            "vietnamese" => Some(&ALL_LANGUAGES[16]),
            "thai" => Some(&ALL_LANGUAGES[17]),
            "indonesian" => Some(&ALL_LANGUAGES[18]),
            "ukrainian" => Some(&ALL_LANGUAGES[19]),
            "basque" | "euskara" => ALL_LANGUAGES.iter().find(|l| l.code == "eu"),
            "swahili" | "kiswahili" => ALL_LANGUAGES.iter().find(|l| l.code == "sw"),
            _ => None,
        })
}

/// User-facing error when a language is valid for threads but not in-chat auto.
pub fn in_chat_auto_reject_message(lang: &Language) -> String {
    format!(
        "{} ({}) is not supported for in-chat auto-translate — the bot must detect the message language locally.\n\
         Use Language Threads for {}, or quote-reply with !translate {} for one-off translation.\n\
         In-chat auto languages: !list-langs-in-chat",
        lang.name, lang.code, lang.code, lang.code
    )
}

pub fn format_language_list(languages: &[Language]) -> String {
    let mut lines: Vec<String> = languages
        .iter()
        .map(|lang| format!("{} {} — {}", lang.flag, lang.code, lang.name))
        .collect();
    lines.sort();
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_iso_code() {
        let lang = resolve_language("es").unwrap();
        assert_eq!(lang.code, "es");
        assert_eq!(lang.flag, "🇪🇸");
    }

    #[test]
    fn resolve_language_name() {
        assert_eq!(resolve_language("Spanish").unwrap().code, "es");
        assert_eq!(resolve_language("español").unwrap().code, "es");
    }

    #[test]
    fn unknown_language_returns_none() {
        assert!(resolve_language("klingon").is_none());
    }

    #[test]
    fn resolve_basque_and_swahili() {
        assert_eq!(resolve_language("eu").unwrap().name, "Basque");
        assert_eq!(resolve_language("euskara").unwrap().code, "eu");
        assert_eq!(resolve_language("sw").unwrap().name, "Swahili");
        assert_eq!(resolve_language("kiswahili").unwrap().code, "sw");
    }

    #[test]
    fn in_chat_auto_excludes_basque_and_swahili() {
        assert!(!is_in_chat_auto_language("eu"));
        assert!(!is_in_chat_auto_language("sw"));
        assert!(is_in_chat_auto_language("es"));
        assert_eq!(in_chat_auto_languages().count(), 30);
        assert_eq!(ALL_LANGUAGES.len(), 32);
        assert!(resolve_in_chat_auto_language("es").is_some());
        assert!(resolve_in_chat_auto_language("eu").is_none());
        assert!(resolve_in_chat_auto_language("basque").is_none());
    }
}
