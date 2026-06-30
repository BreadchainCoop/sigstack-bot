//! Shared translation helpers for `!translate` and `!translate-on`.

use crate::commands::translate_lang::Language;
use crate::group_preferences_store::GroupTranslateMode;
use near_ai_client::{Message, NearAiClient, NearAiError, Role};
use tracing::debug;
use whatlang::Lang;

const MIN_DETECT_CONFIDENCE: f64 = 0.2;

/// Resolve source/target languages for `!translate-on` on a voice note.
pub fn resolve_translate_all_voice_pair(
    mode: &GroupTranslateMode,
    whisper_lang: Option<&str>,
    transcript: &str,
) -> Option<(&'static Language, &'static Language)> {
    for code in voice_language_candidates(whisper_lang, transcript) {
        if let Some(normalized) = normalize_for_translate_all_pair(mode, &code) {
            if let (Some(target), Some(source)) = (
                mode.target_for_source(&normalized),
                mode.source_language(&normalized),
            ) {
                return Some((source, target));
            }
        }
    }
    None
}

/// Map a detected code into one side of the active pair when possible.
fn normalize_for_translate_all_pair(mode: &GroupTranslateMode, code: &str) -> Option<String> {
    let code = code.to_lowercase();
    if mode.target_for_source(&code).is_some() {
        return Some(code);
    }
    // Iberian romance often transcribed as Portuguese; treat as Spanish when es is in the pair.
    if matches!(code.as_str(), "pt" | "ca" | "gl") && (mode.lang_a == "es" || mode.lang_b == "es") {
        if mode.target_for_source("es").is_some() {
            return Some("es".into());
        }
    }
    None
}

fn voice_language_candidates(whisper_lang: Option<&str>, transcript: &str) -> Vec<String> {
    let mut codes = Vec::new();
    let mut push = |code: &str| {
        if !codes.iter().any(|c| c == code) {
            codes.push(code.to_string());
        }
    };

    if let Some(lang) = whisper_lang {
        push(lang);
    }
    if let Some(lang) = detect_text_language_voice(transcript) {
        push(&lang);
    }
    for hint in casual_language_hints(transcript) {
        push(hint);
    }
    codes
}

/// Like [`detect_text_language`] but tuned for short Whisper transcripts.
fn detect_text_language_voice(text: &str) -> Option<String> {
    const MIN_VOICE_CONFIDENCE: f64 = 0.08;

    let info = whatlang::detect(text)?;
    if info.confidence() < MIN_VOICE_CONFIDENCE {
        return None;
    }

    match info.lang() {
        Lang::Eng => Some("en".into()),
        Lang::Spa => Some("es".into()),
        Lang::Por => Some("pt".into()),
        Lang::Fra => Some("fr".into()),
        Lang::Deu => Some("de".into()),
        other => lang_to_iso639_1(other).map(str::to_string),
    }
}

fn text_language_candidates(text: &str) -> Vec<String> {
    let mut codes = Vec::new();
    let mut push = |code: &str| {
        if !codes.iter().any(|c| c == code) {
            codes.push(code.to_string());
        }
    };

    if let Some(lang) = detect_text_language(text) {
        push(&lang);
    }
    if let Some(lang) = detect_text_language_voice(text) {
        push(&lang);
    }
    for hint in casual_language_hints(text) {
        push(hint);
    }
    codes
}

fn casual_language_hints(text: &str) -> Vec<&'static str> {
    let lower = text.to_lowercase();
    let mut hints = Vec::new();

    let english_markers = [
        " the ", " i'm ", " how ", " your ", "hello", "english", " day?",
        "speaking in english", "how are", "doing?",
    ];
    if english_markers.iter().any(|m| lower.contains(m)) {
        hints.push("en");
    }

    let spanish_markers = [
        "¿", "cómo", "como ", "está", "está?", "hola", "gracias", "día", "hablo",
    ];
    if spanish_markers.iter().any(|m| lower.contains(m)) {
        hints.push("es");
    }

    hints
}

/// Detect ISO 639-1 language code from text (for `!translate-on` text messages).
pub fn detect_text_language(text: &str) -> Option<String> {
    let info = whatlang::detect(text)?;
    if info.confidence() < MIN_DETECT_CONFIDENCE {
        debug!(
            confidence = info.confidence(),
            "Text language detection below confidence threshold"
        );
        return None;
    }
    lang_to_iso639_1(info.lang()).map(str::to_string)
}

fn lang_to_iso639_1(lang: Lang) -> Option<&'static str> {
    Some(match lang {
        Lang::Eng => "en",
        Lang::Spa => "es",
        Lang::Cmn => "zh",
        Lang::Hin => "hi",
        Lang::Ben => "bn",
        Lang::Fra => "fr",
        Lang::Ara => "ar",
        Lang::Por => "pt",
        Lang::Rus => "ru",
        Lang::Jpn => "ja",
        Lang::Deu => "de",
        Lang::Kor => "ko",
        Lang::Ita => "it",
        Lang::Nld => "nl",
        Lang::Pol => "pl",
        Lang::Tur => "tr",
        Lang::Ukr => "uk",
        Lang::Swe => "sv",
        Lang::Ces => "cs",
        Lang::Ell => "el",
        Lang::Heb => "he",
        Lang::Ron => "ro",
        Lang::Hun => "hu",
        Lang::Fin => "fi",
        Lang::Dan => "da",
        Lang::Nob => "no",
        Lang::Pes => "fa",
        Lang::Vie => "vi",
        Lang::Tha => "th",
        Lang::Ind => "id",
        _ => return None,
    })
}

/// Translate text via NEAR AI.
pub async fn near_ai_translate(
    near_ai: &NearAiClient,
    source: &str,
    target: &Language,
) -> Result<String, NearAiError> {
    let prompt = format!(
        "Translate the following text to {}. Return only the translation, with no explanation or quotes.\n\n{}",
        target.name, source
    );

    near_ai
        .chat(
            vec![
                Message {
                    role: Role::System,
                    content: Some(
                        "You are a professional translator. Output only the translated text."
                            .into(),
                    ),
                    tool_calls: None,
                    tool_call_id: None,
                },
                Message {
                    role: Role::User,
                    content: Some(prompt),
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            Some(0.3),
            Some(1024),
        )
        .await
}

/// Text auto-translate reply: translation only (original visible in thread).
pub fn format_text_auto_translation(target: &Language, translation: &str) -> String {
    format!("{} {}", target.flag, translation.trim())
}

/// Voice auto-translate reply: transcript + translation in one quote-reply.
pub fn format_voice_auto_translation(
    source: &Language,
    transcript: &str,
    target: &Language,
    translation: &str,
) -> String {
    format!(
        "📝 ({}) {}\n{} ({}) {}",
        source.code,
        transcript.trim(),
        target.flag,
        target.code,
        translation.trim()
    )
}

/// Resolve target language for a text message in translate-all mode.
pub fn target_for_message_text(
    mode: &GroupTranslateMode,
    text: &str,
) -> Option<(&'static Language, &'static Language)> {
    resolve_translate_all_text_pair(mode, text)
}

/// Resolve source/target for group text auto-translate (with short-message fallbacks).
pub fn resolve_translate_all_text_pair(
    mode: &GroupTranslateMode,
    text: &str,
) -> Option<(&'static Language, &'static Language)> {
    for code in text_language_candidates(text) {
        if let Some(normalized) = normalize_for_translate_all_pair(mode, &code) {
            if let (Some(target), Some(source)) = (
                mode.target_for_source(&normalized),
                mode.source_language(&normalized),
            ) {
                return Some((source, target));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::translate_lang::resolve_language;

    #[test]
    fn detects_english_text() {
        assert_eq!(
            detect_text_language("Is anyone going to the meetup?").as_deref(),
            Some("en")
        );
    }

    #[test]
    fn resolve_text_pair_casual_english() {
        let mode = GroupTranslateMode::new(
            resolve_language("es").unwrap(),
            resolve_language("en").unwrap(),
        );
        let pair = resolve_translate_all_text_pair(&mode, "hello, how are you doing?")
            .expect("casual English should match en in es/en pair");
        assert_eq!(pair.0.code, "en");
        assert_eq!(pair.1.code, "es");
    }

    #[test]
    fn resolve_voice_pair_from_whisper_english() {
        let mode = GroupTranslateMode::new(
            resolve_language("es").unwrap(),
            resolve_language("en").unwrap(),
        );
        let pair = resolve_translate_all_voice_pair(
            &mode,
            Some("en"),
            "Hello, I'm speaking in English now. How was your day?",
        );
        let (source, target) = pair.expect("should resolve en -> es");
        assert_eq!(source.code, "en");
        assert_eq!(target.code, "es");
    }

    #[test]
    fn resolve_voice_pair_spanish_from_whisper_or_hints() {
        let mode = GroupTranslateMode::new(
            resolve_language("es").unwrap(),
            resolve_language("en").unwrap(),
        );
        let pair = resolve_translate_all_voice_pair(
            &mode,
            Some("es"),
            "Como está? Como foi tu dia oi?",
        )
        .or_else(|| {
            resolve_translate_all_voice_pair(&mode, None, "Como está? Como foi tu dia oi?")
        });
        let (source, target) = pair.expect("should resolve es -> en");
        assert_eq!(source.code, "es");
        assert_eq!(target.code, "en");
    }

    #[test]
    fn resolve_voice_pair_maps_portuguese_to_spanish_in_es_en_pair() {
        let mode = GroupTranslateMode::new(
            resolve_language("es").unwrap(),
            resolve_language("en").unwrap(),
        );
        let pair = resolve_translate_all_voice_pair(&mode, Some("pt"), "Como foi tu dia?");
        let (source, target) = pair.expect("pt should map to es in es/en pair");
        assert_eq!(source.code, "es");
        assert_eq!(target.code, "en");
    }

    #[test]
    fn format_voice_bilingual() {
        let es = resolve_language("es").unwrap();
        let en = resolve_language("en").unwrap();
        let out = format_voice_auto_translation(es, "Hola", en, "Hello");
        assert!(out.contains("📝 (es) Hola"));
        assert!(out.contains("🇺🇸 (en) Hello"));
    }
}
