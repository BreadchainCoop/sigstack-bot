//! Shared translation helpers for `!translate` and `!translate-on`.

use crate::commands::translate_lang::Language;
use crate::group_preferences_store::GroupTranslateMode;
use near_ai_client::{Message, NearAiClient, NearAiError, Role};
use tracing::debug;
use whatlang::{Detector, Lang};

const MIN_DETECT_CONFIDENCE: f64 = 0.2;

/// Map a detected code into one side of the active pair when possible.
fn normalize_for_translate_all_pair(mode: &GroupTranslateMode, code: &str) -> Option<String> {
    let code = code.to_lowercase();
    if mode.target_for_source(&code).is_some() {
        return Some(code);
    }
    // Iberian romance often transcribed as Portuguese; treat as Spanish when es is in the pair.
    if matches!(code.as_str(), "pt" | "ca" | "gl")
        && (mode.lang_a == "es" || mode.lang_b == "es")
        && mode.target_for_source("es").is_some()
    {
        return Some("es".into());
    }
    None
}

fn iso_to_whatlang(code: &str) -> Option<Lang> {
    Some(match code {
        "en" => Lang::Eng,
        "es" => Lang::Spa,
        "zh" => Lang::Cmn,
        "hi" => Lang::Hin,
        "bn" => Lang::Ben,
        "fr" => Lang::Fra,
        "ar" => Lang::Ara,
        "pt" => Lang::Por,
        "ru" => Lang::Rus,
        "ja" => Lang::Jpn,
        "de" => Lang::Deu,
        "ko" => Lang::Kor,
        "it" => Lang::Ita,
        "nl" => Lang::Nld,
        "pl" => Lang::Pol,
        "tr" => Lang::Tur,
        "uk" => Lang::Ukr,
        "sv" => Lang::Swe,
        "cs" => Lang::Ces,
        "el" => Lang::Ell,
        "he" => Lang::Heb,
        "ro" => Lang::Ron,
        "hu" => Lang::Hun,
        "fi" => Lang::Fin,
        "da" => Lang::Dan,
        "no" => Lang::Nob,
        "fa" => Lang::Pes,
        "vi" => Lang::Vie,
        "th" => Lang::Tha,
        "id" => Lang::Ind,
        "ca" => Lang::Cat,
        _ => return None,
    })
}

/// Detect language restricted to the active bilingual pair.
///
/// Open-vocabulary whatlang often mislabels short English as Norwegian (etc.)
/// at tiny confidence; constraining to the pair recovers both directions.
///
/// Absolute confidence can stay very low even when the allowlist correctly
/// picks the winner (e.g. Italian vs Spanish on "ciao buongiorno" ~0.02).
/// Trust the allowlist result whenever it returns a language.
fn detect_text_language_in_pair(mode: &GroupTranslateMode, text: &str) -> Option<String> {
    let allowlist: Vec<Lang> = [mode.lang_a.as_str(), mode.lang_b.as_str()]
        .into_iter()
        .filter_map(iso_to_whatlang)
        .collect();
    if allowlist.len() < 2 {
        return None;
    }

    let info = Detector::with_allowlist(allowlist).detect(text)?;
    lang_to_iso639_1(info.lang()).map(str::to_string)
}

/// Like [`detect_text_language`] but tuned for short / casual messages.
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

fn text_language_candidates(mode: &GroupTranslateMode, text: &str) -> Vec<String> {
    let mut codes = Vec::new();
    let mut push = |code: &str| {
        if !codes.iter().any(|c| c == code) {
            codes.push(code.to_string());
        }
    };

    // Prefer pair-constrained whatlang so short messages aren't lost to open-vocab mislabels.
    if let Some(lang) = detect_text_language_in_pair(mode, text) {
        push(&lang);
    }
    if let Some(lang) = detect_text_language(text) {
        push(&lang);
    }
    if let Some(lang) = detect_text_language_voice(text) {
        push(&lang);
    }
    codes
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
        Lang::Cat => "ca",
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
        .chat_with_retry(
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
            Some(2),
        )
        .await
}

/// Text auto-translate reply: translation only (original visible in thread).
pub fn format_text_auto_translation(target: &Language, translation: &str) -> String {
    format!("{} {}", target.flag, translation.trim())
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
    for code in text_language_candidates(mode, text) {
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
    fn resolve_text_pair_short_english_both_directions() {
        let mode = GroupTranslateMode::new(
            resolve_language("en").unwrap(),
            resolve_language("es").unwrap(),
        );
        // Open whatlang labels this as Norwegian at ~3% confidence; pair allowlist recovers en→es.
        let pair = resolve_translate_all_text_pair(&mode, "i'm doing quite fine thanks")
            .expect("short English should translate to Spanish");
        assert_eq!(pair.0.code, "en");
        assert_eq!(pair.1.code, "es");

        let pair = resolve_translate_all_text_pair(&mode, "estoy bien!")
            .expect("short Spanish should translate to English");
        assert_eq!(pair.0.code, "es");
        assert_eq!(pair.1.code, "en");
    }

    #[test]
    fn resolve_text_pair_maps_portuguese_to_spanish_in_es_en_pair() {
        let mode = GroupTranslateMode::new(
            resolve_language("es").unwrap(),
            resolve_language("en").unwrap(),
        );
        let pair = resolve_translate_all_text_pair(&mode, "Como foi tu dia?");
        let (source, target) = pair.expect("pt-like text should map to es in es/en pair");
        assert_eq!(source.code, "es");
        assert_eq!(target.code, "en");
    }

    #[test]
    fn format_text_auto_includes_flag() {
        let es = resolve_language("es").unwrap();
        let out = format_text_auto_translation(es, " Buenos días ");
        assert_eq!(out, format!("{} Buenos días", es.flag));
    }

    #[test]
    fn resolve_text_pair_short_italian_in_it_es_pair() {
        let mode = GroupTranslateMode::new(
            resolve_language("it").unwrap(),
            resolve_language("es").unwrap(),
        );
        // Pair allowlist picks Italian at ~2% absolute confidence — still accept the winner.
        let pair = resolve_translate_all_text_pair(&mode, "ciao buongiorno")
            .expect("short Italian should translate to Spanish");
        assert_eq!(pair.0.code, "it");
        assert_eq!(pair.1.code, "es");

        let pair = resolve_translate_all_text_pair(&mode, "buenos dias")
            .expect("Spanish should translate to Italian");
        assert_eq!(pair.0.code, "es");
        assert_eq!(pair.1.code, "it");
    }
}
