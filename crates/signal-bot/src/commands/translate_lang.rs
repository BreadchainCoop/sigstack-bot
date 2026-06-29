//! Language codes, names, and flag emojis for translation commands.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Language {
    pub code: &'static str,
    pub name: &'static str,
    pub flag: &'static str,
}

/// Full supported language catalog (for `!translate-langs`).
pub const ALL_LANGUAGES: &[Language] = &[
    Language { code: "en", name: "English", flag: "🇺🇸" },
    Language { code: "es", name: "Spanish", flag: "🇪🇸" },
    Language { code: "fr", name: "French", flag: "🇫🇷" },
    Language { code: "de", name: "German", flag: "🇩🇪" },
    Language { code: "it", name: "Italian", flag: "🇮🇹" },
    Language { code: "pt", name: "Portuguese", flag: "🇵🇹" },
    Language { code: "ru", name: "Russian", flag: "🇷🇺" },
    Language { code: "zh", name: "Chinese", flag: "🇨🇳" },
    Language { code: "ja", name: "Japanese", flag: "🇯🇵" },
    Language { code: "ko", name: "Korean", flag: "🇰🇷" },
    Language { code: "ar", name: "Arabic", flag: "🇸🇦" },
    Language { code: "hi", name: "Hindi", flag: "🇮🇳" },
    Language { code: "bn", name: "Bengali", flag: "🇧🇩" },
    Language { code: "nl", name: "Dutch", flag: "🇳🇱" },
    Language { code: "pl", name: "Polish", flag: "🇵🇱" },
    Language { code: "tr", name: "Turkish", flag: "🇹🇷" },
    Language { code: "vi", name: "Vietnamese", flag: "🇻🇳" },
    Language { code: "th", name: "Thai", flag: "🇹🇭" },
    Language { code: "id", name: "Indonesian", flag: "🇮🇩" },
    Language { code: "uk", name: "Ukrainian", flag: "🇺🇦" },
    Language { code: "sv", name: "Swedish", flag: "🇸🇪" },
    Language { code: "cs", name: "Czech", flag: "🇨🇿" },
    Language { code: "el", name: "Greek", flag: "🇬🇷" },
    Language { code: "he", name: "Hebrew", flag: "🇮🇱" },
    Language { code: "ro", name: "Romanian", flag: "🇷🇴" },
    Language { code: "hu", name: "Hungarian", flag: "🇭🇺" },
    Language { code: "fi", name: "Finnish", flag: "🇫🇮" },
    Language { code: "da", name: "Danish", flag: "🇩🇰" },
    Language { code: "no", name: "Norwegian", flag: "🇳🇴" },
    Language { code: "fa", name: "Persian", flag: "🇮🇷" },
];

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
            _ => None,
        })
}

pub fn format_language_list(languages: &[Language]) -> String {
    let mut lines: Vec<String> = languages
        .iter()
        .map(|lang| format!("{} {} — {}", lang.flag, lang.code, lang.name))
        .collect();
    lines.sort_by(|a, b| a.cmp(b));
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
}
