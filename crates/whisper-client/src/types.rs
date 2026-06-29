//! Whisper API types.

use serde::Deserialize;

/// Response from `GET /health`.
#[derive(Debug, Clone, Deserialize)]
pub struct HealthResponse {
    pub status: String,
}

/// JSON response from `POST /inference` with `response_format=verbose_json`.
#[derive(Debug, Clone, Deserialize)]
pub struct InferenceResponse {
    pub text: String,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub detected_language: Option<String>,
}

/// Parsed transcription result.
#[derive(Debug, Clone)]
pub struct TranscriptionResult {
    pub text: String,
    /// Detected spoken language from Whisper (ISO 639-1 when mappable).
    pub language: Option<String>,
}

impl TranscriptionResult {
    pub fn trimmed_text(&self) -> &str {
        self.text.trim()
    }
}

/// Map whisper.cpp language name or ISO code to ISO 639-1.
pub fn whisper_language_to_iso(lang: &str) -> Option<&'static str> {
    let normalized = lang.trim().to_lowercase();
    match normalized.as_str() {
        "en" | "english" => Some("en"),
        "es" | "spanish" | "castilian" => Some("es"),
        "fr" | "french" => Some("fr"),
        "de" | "german" => Some("de"),
        "it" | "italian" => Some("it"),
        "pt" | "portuguese" => Some("pt"),
        "ru" | "russian" => Some("ru"),
        "zh" | "chinese" | "mandarin" => Some("zh"),
        "ja" | "japanese" => Some("ja"),
        "ko" | "korean" => Some("ko"),
        "ar" | "arabic" => Some("ar"),
        "hi" | "hindi" => Some("hi"),
        "bn" | "bengali" => Some("bn"),
        "nl" | "dutch" => Some("nl"),
        "pl" | "polish" => Some("pl"),
        "tr" | "turkish" => Some("tr"),
        "vi" | "vietnamese" => Some("vi"),
        "th" | "thai" => Some("th"),
        "id" | "indonesian" => Some("id"),
        "uk" | "ukrainian" => Some("uk"),
        "sv" | "swedish" => Some("sv"),
        "cs" | "czech" => Some("cs"),
        "el" | "greek" => Some("el"),
        "he" | "iw" | "hebrew" => Some("he"),
        "ro" | "romanian" => Some("ro"),
        "hu" | "hungarian" => Some("hu"),
        "fi" | "finnish" => Some("fi"),
        "da" | "danish" => Some("da"),
        "no" | "norwegian" => Some("no"),
        "fa" | "persian" => Some("fa"),
        "ca" | "catalan" => Some("ca"),
        _ => None,
    }
}
