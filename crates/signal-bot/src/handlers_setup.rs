//! Build role-specific command handler stacks.

use crate::bot_identity::BotIdentity;
use crate::commands::*;
use crate::config::{BotRole, Config};
use crate::error::AppResult;
use crate::group_preferences_store::GroupPreferencesStore;
use crate::transcribe_prefs::GroupTranscribePrefs;
use anyhow::Context;
use dstack_client::DstackClient;
use near_ai_client::NearAiClient;
use signal_bot_transcription::build_voice_handlers;
use signal_client::SignalClient;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn};
use whisper_client::WhisperClient;

/// Build handlers for the configured bot role.
pub async fn build_handlers(
    config: &Config,
    signal: Arc<SignalClient>,
    dstack: Arc<DstackClient>,
    bot_identity: Arc<BotIdentity>,
) -> AppResult<Vec<Box<dyn CommandHandler>>> {
    match config.bot.role {
        BotRole::Transcription => build_transcription_handlers(config, signal, dstack).await,
        BotRole::Translation => {
            build_translation_handlers(config, signal, dstack, bot_identity).await
        }
    }
}

/// Transcription CVM: voice / !transcribe* / !transcription / help-transcription / verify.
pub async fn build_transcription_handlers(
    config: &Config,
    signal: Arc<SignalClient>,
    dstack: Arc<DstackClient>,
) -> AppResult<Vec<Box<dyn CommandHandler>>> {
    let whisper = Arc::new(
        WhisperClient::new(&config.whisper.service_url, config.whisper.timeout)
            .context("Failed to create Whisper client")?,
    );

    if whisper.health_check().await {
        info!(
            "Whisper healthy at {} (model={})",
            config.whisper.service_url, config.whisper.model
        );
    } else {
        warn!(
            "Whisper health check failed at {} — will retry on requests",
            config.whisper.service_url
        );
    }

    let group_prefs = GroupPreferencesStore::open(
        dstack.clone(),
        PathBuf::from(&config.group_preferences.storage_path),
        config.group_preferences.persist,
        config.translate_all.max_messages_per_minute,
    )
    .await;

    // Voice only — no NEAR translate-on-voice; translation CVM handles posted text.
    let mut handlers = build_voice_handlers(
        whisper,
        signal,
        config.whisper.reply_prefix.clone(),
        config.whisper.max_attachment_bytes,
        Arc::new(GroupTranscribePrefs(group_prefs.clone())),
    );
    handlers.push(Box::new(TranscriptionMenuHandler::new()));
    handlers.push(Box::new(HelpTranscriptionHandler::new()));
    handlers.push(Box::new(VerifyHandler::new(dstack, BotRole::Transcription)));

    info!(
        "Transcription role: voice / !transcribe* / !transcription / help-transcription / verify (hub !help / !info / !privacy on translation bot only)"
    );
    Ok(handlers)
}

/// Translation CVM: Language Threads + in-chat + quote !translate.
pub async fn build_translation_handlers(
    config: &Config,
    signal: Arc<SignalClient>,
    dstack: Arc<DstackClient>,
    bot_identity: Arc<BotIdentity>,
) -> AppResult<Vec<Box<dyn CommandHandler>>> {
    let near_cfg = config
        .near_ai
        .as_ref()
        .context("NEAR AI config missing after validation")?;

    let near_ai = Arc::new(
        NearAiClient::new(
            &near_cfg.api_key,
            &near_cfg.base_url,
            &near_cfg.model,
            near_cfg.timeout,
        )
        .context("Failed to create NEAR AI client")?,
    );

    if near_ai.health_check().await {
        info!("NEAR AI healthy - Model: {}", near_cfg.model);
    } else {
        warn!("NEAR AI health check failed - will retry on requests");
    }

    let group_prefs = GroupPreferencesStore::open(
        dstack.clone(),
        PathBuf::from(&config.group_preferences.storage_path),
        config.group_preferences.persist,
        config.translate_all.max_messages_per_minute,
    )
    .await;

    if config.group_preferences.persist {
        info!(
            "Group preferences persistence enabled: {}",
            config.group_preferences.storage_path
        );
    }

    let mut handlers: Vec<Box<dyn CommandHandler>> = Vec::new();

    handlers.push(Box::new(TranslateMeHandler::new(
        group_prefs.clone(),
        near_ai.clone(),
        signal.clone(),
        bot_identity,
    )));
    info!(
        "Language Threads enabled: !translate-me-thread / !leave / !enable-in-chat (max {}/min)",
        config.translate_all.max_messages_per_minute
    );

    if config.translate_all.enabled {
        handlers.push(Box::new(TranslateAllHandler::new(
            group_prefs.clone(),
            near_ai.clone(),
            signal.clone(),
        )));
        info!(
            "In-chat translation enabled: !translate-all-on / !translate-me-on / !enable-threads"
        );
    }

    handlers.push(Box::new(TranslationMenuHandler::new(
        config.translate_all.enabled,
    )));
    handlers.push(Box::new(TranslationThreadsMenuHandler::new()));
    handlers.push(Box::new(TranslationInChatMenuHandler::new(
        config.translate_all.enabled,
    )));
    handlers.push(Box::new(HelpThreadsHandler::new()));
    handlers.push(Box::new(HelpInChatHandler::new()));
    handlers.push(Box::new(HelpTranscriptionHandler::new()));
    handlers.push(Box::new(TranscriptionPairingHandler::new(
        signal.clone(),
        config.signal.peer_phone.clone(),
    )));
    handlers.push(Box::new(InChatMenuHandler::new(
        config.translate_all.enabled,
    )));

    handlers.push(Box::new(TranslateHandler::new(
        near_ai.clone(),
        signal.clone(),
        "📝 Transcript:",
    )));
    handlers.push(Box::new(TranslateLangsHandler::new()));
    handlers.push(Box::new(RenameHandler::new(
        group_prefs.clone(),
        signal.clone(),
    )));
    handlers.push(Box::new(CommandsHandler::new(group_prefs.clone())));
    handlers.push(Box::new(VerifyHandler::new(
        dstack.clone(),
        BotRole::Translation,
    )));
    handlers.push(Box::new(HelpHandler::new(BotRole::Translation)));
    handlers.push(Box::new(InfoHandler::new(
        group_prefs,
        BotRole::Translation,
    )));
    handlers.push(Box::new(PrivacyHandler::new()));

    info!("Translation role: hub menus + in-chat + Language Threads");
    Ok(handlers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        BotConfig, DstackConfig, GroupPreferencesConfig, NearAiConfig, SignalConfig,
        TranslateAllConfig, WhisperConfig,
    };
    use std::time::Duration;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn base_config(role: BotRole) -> Config {
        Config {
            signal: SignalConfig {
                service_url: "http://127.0.0.1:9".into(),
                poll_interval: Duration::from_millis(50),
                phone_number: None,
                peer_phone: None,
            },
            near_ai: None,
            bot: BotConfig {
                role,
                signal_username: None,
                github_repo: None,
                log_level: "info".into(),
            },
            dstack: DstackConfig {
                socket_path: "/tmp/sigstack-bot-test-dstack.sock".into(),
            },
            whisper: WhisperConfig::default(),
            translate_all: TranslateAllConfig {
                enabled: true,
                max_messages_per_minute: 30,
            },
            group_preferences: GroupPreferencesConfig {
                persist: false,
                storage_path: "/tmp/sigstack-bot-test-prefs.enc".into(),
            },
        }
    }

    fn labels(handlers: &[Box<dyn CommandHandler>]) -> Vec<&'static str> {
        handlers.iter().map(|h| h.label()).collect()
    }

    #[tokio::test]
    async fn transcription_registers_expected_handlers() {
        let whisper = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&whisper)
            .await;

        let mut config = base_config(BotRole::Transcription);
        config.whisper.service_url = whisper.uri();

        let signal = Arc::new(SignalClient::new(&config.signal.service_url).unwrap());
        let dstack = Arc::new(DstackClient::new(&config.dstack.socket_path));
        let identity = BotIdentity::new();

        let handlers = build_handlers(&config, signal, dstack, identity)
            .await
            .expect("transcription handlers");

        assert_eq!(handlers.len(), 6);
        assert_eq!(
            labels(&handlers),
            vec![
                "voice",
                "manual_transcribe",
                "transcribe",
                "transcription_menu",
                "help_transcription",
                "command", // verify
            ]
        );
    }

    #[tokio::test]
    async fn translation_registers_expected_handlers_with_translate_all() {
        let near = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&near)
            .await;

        let mut config = base_config(BotRole::Translation);
        config.near_ai = Some(NearAiConfig {
            api_key: "test-key".into(),
            base_url: near.uri(),
            model: "test-model".into(),
            timeout: Duration::from_secs(5),
        });

        let signal = Arc::new(SignalClient::new("http://127.0.0.1:9").unwrap());
        let dstack = Arc::new(DstackClient::new(&config.dstack.socket_path));
        let identity = BotIdentity::new();

        let handlers = build_handlers(&config, signal, dstack, identity)
            .await
            .expect("translation handlers");

        assert_eq!(handlers.len(), 18);
        let got = labels(&handlers);
        assert!(got.contains(&"translate_me"));
        assert!(got.contains(&"translate_all"));
        assert!(got.contains(&"translation_menu"));
        assert!(got.contains(&"translation_threads_menu"));
        assert!(got.contains(&"translation_in_chat_menu"));
        assert!(got.contains(&"help_threads"));
        assert!(got.contains(&"help_in_chat"));
        assert!(got.contains(&"help_transcription"));
        assert!(got.contains(&"transcription_pairing"));
        assert!(got.contains(&"in_chat_menu"));
        assert!(!got.contains(&"translate_parallel"));
        assert!(!got.contains(&"parallel_menu"));
        assert!(!got.contains(&"models"));
        assert!(got.contains(&"translate"));
        assert!(got.contains(&"translate_langs"));
        assert!(got.contains(&"rename"));
        assert!(got.contains(&"commands"));
        assert!(!got.contains(&"set_language"));
        assert!(got.contains(&"help"));
        assert!(got.contains(&"info"));
        assert!(got.contains(&"privacy"));
    }

    #[tokio::test]
    async fn translation_omits_translate_all_when_disabled() {
        let near = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&near)
            .await;

        let mut config = base_config(BotRole::Translation);
        config.translate_all.enabled = false;
        config.near_ai = Some(NearAiConfig {
            api_key: "test-key".into(),
            base_url: near.uri(),
            model: "test-model".into(),
            timeout: Duration::from_secs(5),
        });

        let signal = Arc::new(SignalClient::new("http://127.0.0.1:9").unwrap());
        let dstack = Arc::new(DstackClient::new(&config.dstack.socket_path));
        let identity = BotIdentity::new();

        let handlers = build_handlers(&config, signal, dstack, identity)
            .await
            .expect("translation handlers");

        assert_eq!(handlers.len(), 17);
        assert!(!labels(&handlers).contains(&"translate_all"));
        assert!(labels(&handlers).contains(&"translate_me"));
        assert!(labels(&handlers).contains(&"in_chat_menu"));
        assert!(labels(&handlers).contains(&"translation_threads_menu"));
        assert!(labels(&handlers).contains(&"help_threads"));
        assert!(labels(&handlers).contains(&"help_in_chat"));
        assert!(labels(&handlers).contains(&"help_transcription"));
        assert!(labels(&handlers).contains(&"info"));
    }

    #[tokio::test]
    async fn translation_requires_near_ai_config() {
        let config = base_config(BotRole::Translation);
        let signal = Arc::new(SignalClient::new("http://127.0.0.1:9").unwrap());
        let dstack = Arc::new(DstackClient::new(&config.dstack.socket_path));
        let identity = BotIdentity::new();

        let result = build_handlers(&config, signal, dstack, identity).await;
        assert!(result.is_err(), "missing NEAR AI should fail");
        assert!(
            result.err().unwrap().to_string().contains("NEAR AI"),
            "error should mention NEAR AI"
        );
    }
}
