//! Application configuration loaded from environment variables.

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::time::Duration;

/// Which product surface this process runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BotRole {
    Transcription,
    Translation,
}

/// Application configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Signal configuration
    #[serde(default)]
    pub signal: SignalConfig,

    /// NEAR AI configuration (required for translation role)
    #[serde(default)]
    pub near_ai: Option<NearAiConfig>,

    /// Bot configuration (includes required `BOT__ROLE`)
    pub bot: BotConfig,

    /// Dstack configuration
    #[serde(default)]
    pub dstack: DstackConfig,

    /// Whisper transcription configuration
    #[serde(default)]
    pub whisper: WhisperConfig,

    /// Group auto-translate (`!translate-on`) configuration
    #[serde(default)]
    pub translate_all: TranslateAllConfig,

    /// Encrypted persistence for per-group bot preferences
    #[serde(default)]
    pub group_preferences: GroupPreferencesConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SignalConfig {
    /// Signal CLI REST API endpoint
    #[serde(default = "default_signal_service")]
    pub service_url: String,

    /// Poll interval for messages
    #[serde(default = "default_poll_interval", with = "humantime_serde")]
    pub poll_interval: Duration,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NearAiConfig {
    /// NEAR AI API key
    pub api_key: String,

    /// API base URL
    #[serde(default = "default_near_ai_url")]
    pub base_url: String,

    /// Default model
    #[serde(default = "default_model")]
    pub model: String,

    /// Request timeout
    #[serde(default = "default_timeout", with = "humantime_serde")]
    pub timeout: Duration,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BotConfig {
    /// Product role: `transcription` or `translation` (required)
    pub role: BotRole,

    /// Signal username (e.g., "nearai.54")
    #[serde(default)]
    pub signal_username: Option<String>,

    /// GitHub repository URL
    #[serde(default)]
    pub github_repo: Option<String>,

    /// Log level
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DstackConfig {
    /// Dstack guest agent socket path
    #[serde(default = "default_dstack_socket")]
    pub socket_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WhisperConfig {
    /// Master switch for voice transcription
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// whisper-server base URL (no trailing path)
    #[serde(default = "default_whisper_service")]
    pub service_url: String,

    /// Model name loaded in the sidecar (e.g. `small`)
    #[serde(default = "default_whisper_model")]
    pub model: String,

    /// Max time per transcribe request
    #[serde(default = "default_whisper_timeout", with = "humantime_serde")]
    pub timeout: Duration,

    /// Reject attachments larger than this (rough proxy for max voice length)
    #[serde(default = "default_whisper_max_attachment_bytes")]
    pub max_attachment_bytes: usize,

    /// Prefix line before transcript text in quote-replies
    #[serde(default = "default_whisper_reply_prefix")]
    pub reply_prefix: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TranslateAllConfig {
    /// Master switch for group auto-translate
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Max auto-translate replies per group per minute (NEAR AI protection)
    #[serde(default = "default_translate_all_max_per_minute")]
    pub max_messages_per_minute: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GroupPreferencesConfig {
    /// Persist group transcription + translate-all settings (TEE-encrypted)
    #[serde(default = "default_true")]
    pub persist: bool,

    /// Encrypted preferences file path (Docker volume in production)
    #[serde(default = "default_group_preferences_path")]
    pub storage_path: String,
}

// Default implementations
impl Default for SignalConfig {
    fn default() -> Self {
        Self {
            service_url: default_signal_service(),
            poll_interval: default_poll_interval(),
        }
    }
}

impl Default for DstackConfig {
    fn default() -> Self {
        Self {
            socket_path: default_dstack_socket(),
        }
    }
}

impl Default for WhisperConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            service_url: default_whisper_service(),
            model: default_whisper_model(),
            timeout: default_whisper_timeout(),
            max_attachment_bytes: default_whisper_max_attachment_bytes(),
            reply_prefix: default_whisper_reply_prefix(),
        }
    }
}

impl Default for TranslateAllConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            max_messages_per_minute: default_translate_all_max_per_minute(),
        }
    }
}

impl Default for GroupPreferencesConfig {
    fn default() -> Self {
        Self {
            persist: default_true(),
            storage_path: default_group_preferences_path(),
        }
    }
}

fn default_signal_service() -> String {
    "http://signal-api:8080".into()
}

fn default_poll_interval() -> Duration {
    Duration::from_millis(200)
}

fn default_near_ai_url() -> String {
    "https://cloud-api.near.ai/v1".into()
}

fn default_model() -> String {
    "deepseek-ai/DeepSeek-V3.1".into()
}

fn default_timeout() -> Duration {
    Duration::from_secs(10)
}

fn default_log_level() -> String {
    "info".into()
}

fn default_dstack_socket() -> String {
    "/var/run/dstack.sock".into()
}

fn default_true() -> bool {
    true
}

fn default_whisper_service() -> String {
    "http://whisper-api:9000".into()
}

fn default_whisper_model() -> String {
    "small".into()
}

fn default_whisper_timeout() -> Duration {
    Duration::from_secs(120)
}

fn default_whisper_max_attachment_bytes() -> usize {
    // ~5 min voice at typical Signal bitrates
    10 * 1024 * 1024
}

fn default_whisper_reply_prefix() -> String {
    "📝 Transcript:".into()
}

fn default_translate_all_max_per_minute() -> u32 {
    30
}

fn default_group_preferences_path() -> String {
    "/data/group_prefs.enc".into()
}

impl Config {
    /// Load configuration from environment variables.
    pub fn load() -> Result<Self> {
        // Load .env file if present
        dotenvy::dotenv().ok();

        let config = config::Config::builder()
            .add_source(
                config::Environment::default()
                    .separator("__")
                    // Note: try_parsing(true) would parse +15551234567 as a positive number
                    // stripping the + prefix. Keep strings as strings.
                    .try_parsing(false),
            )
            .build()
            .context("Failed to build configuration")?;

        let cfg: Self = config
            .try_deserialize()
            .context("Failed to deserialize configuration")?;

        cfg.validate()?;
        Ok(cfg)
    }

    pub(crate) fn validate(&self) -> Result<()> {
        match self.bot.role {
            BotRole::Transcription => {
                if !self.whisper.enabled {
                    bail!("BOT__ROLE=transcription requires WHISPER__ENABLED=true");
                }
            }
            BotRole::Translation => {
                let Some(near) = &self.near_ai else {
                    bail!("BOT__ROLE=translation requires NEAR_AI__API_KEY (and related NEAR_AI__* settings)");
                };
                if near.api_key.trim().is_empty() {
                    bail!("BOT__ROLE=translation requires a non-empty NEAR_AI__API_KEY");
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transcription_config(whisper_enabled: bool) -> Config {
        Config {
            signal: SignalConfig::default(),
            near_ai: None,
            bot: BotConfig {
                role: BotRole::Transcription,
                signal_username: None,
                github_repo: None,
                log_level: "info".into(),
            },
            dstack: DstackConfig::default(),
            whisper: WhisperConfig {
                enabled: whisper_enabled,
                ..WhisperConfig::default()
            },
            translate_all: TranslateAllConfig::default(),
            group_preferences: GroupPreferencesConfig::default(),
        }
    }

    fn translation_config(api_key: Option<&str>) -> Config {
        Config {
            signal: SignalConfig::default(),
            near_ai: api_key.map(|k| NearAiConfig {
                api_key: k.into(),
                base_url: default_near_ai_url(),
                model: default_model(),
                timeout: default_timeout(),
            }),
            bot: BotConfig {
                role: BotRole::Translation,
                signal_username: None,
                github_repo: None,
                log_level: "info".into(),
            },
            dstack: DstackConfig::default(),
            whisper: WhisperConfig {
                enabled: false,
                ..WhisperConfig::default()
            },
            translate_all: TranslateAllConfig::default(),
            group_preferences: GroupPreferencesConfig::default(),
        }
    }

    #[test]
    fn transcription_requires_whisper_enabled() {
        assert!(transcription_config(true).validate().is_ok());
        let err = transcription_config(false).validate().unwrap_err();
        assert!(err.to_string().contains("WHISPER__ENABLED"));
    }

    #[test]
    fn translation_requires_near_ai_key() {
        assert!(translation_config(Some("sk-test")).validate().is_ok());

        let err = translation_config(None).validate().unwrap_err();
        assert!(err.to_string().contains("NEAR_AI__API_KEY"));

        let err = translation_config(Some("   ")).validate().unwrap_err();
        assert!(err.to_string().contains("non-empty"));
    }

    #[test]
    fn load_transcription_from_env() {
        std::env::set_var("BOT__ROLE", "transcription");
        std::env::remove_var("NEAR_AI__API_KEY");
        let cfg = Config::load().expect("load transcription");
        assert_eq!(cfg.bot.role, BotRole::Transcription);
        assert!(cfg.whisper.enabled);
        std::env::remove_var("BOT__ROLE");
    }

    #[test]
    fn load_translation_from_env() {
        std::env::set_var("BOT__ROLE", "translation");
        std::env::set_var("NEAR_AI__API_KEY", "sk-test-key");
        let cfg = Config::load().expect("load translation");
        assert_eq!(cfg.bot.role, BotRole::Translation);
        assert_eq!(
            cfg.near_ai.as_ref().map(|n| n.api_key.as_str()),
            Some("sk-test-key")
        );
        std::env::remove_var("BOT__ROLE");
        std::env::remove_var("NEAR_AI__API_KEY");
    }

    #[test]
    fn section_defaults() {
        assert_eq!(
            SignalConfig::default().service_url,
            "http://signal-api:8080"
        );
        assert_eq!(WhisperConfig::default().model, "small");
        assert!(TranslateAllConfig::default().enabled);
        assert!(GroupPreferencesConfig::default().persist);
    }
}
