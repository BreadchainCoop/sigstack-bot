//! Shared translation helpers for `!translate` and `!translate-all`.

use crate::commands::translate_lang::Language;
use crate::group_translate_store::GroupTranslateMode;
use near_ai_client::{Message, NearAiClient, NearAiError, Role};
use tracing::debug;
use whatlang::Lang;

const MIN_DETECT_CONFIDENCE: f64 = 0.2;

/// Detect ISO 639-1 language code from text (for `!translate-all` text messages).
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

/// Resolve target language for a message in translate-all mode.
pub fn target_for_message_text(
    mode: &GroupTranslateMode,
    text: &str,
) -> Option<(&'static Language, &'static Language)> {
    let detected = detect_text_language(text)?;
    let target = mode.target_for_source(&detected)?;
    let source = mode.source_language(&detected)?;
    Some((source, target))
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
    fn detects_spanish_text() {
        assert_eq!(
            detect_text_language("¿Alguien va al meetup?").as_deref(),
            Some("es")
        );
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
