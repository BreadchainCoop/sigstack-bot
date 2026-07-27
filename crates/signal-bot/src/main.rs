//! Signal product bots — transcription or translation (see `BOT__ROLE`).

use signal_bot::bot_identity::BotIdentity;
use signal_bot::commands::*;
use signal_bot::config::{BotRole, Config};
use signal_bot::error::AppResult;
use signal_bot::group_preferences_store::GroupPreferencesStore;
use signal_bot::transcribe_store::TranscribeStore;
use signal_bot::voice_attachment_cache::VoiceAttachmentCache;
use anyhow::Context;
use dstack_client::DstackClient;
use near_ai_client::NearAiClient;
use signal_client::{MessageReceiver, SignalClient};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::signal;
use tokio_stream::StreamExt;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use whisper_client::WhisperClient;

#[tokio::main]
async fn main() -> AppResult<()> {
    let config = Config::load().context("Failed to load configuration")?;

    init_logging(&config.bot.log_level);

    info!(
        role = ?config.bot.role,
        "Starting sigstack Signal bot"
    );

    let dstack = Arc::new(DstackClient::new(&config.dstack.socket_path));

    let signal = Arc::new(
        SignalClient::new(&config.signal.service_url)
            .context("Failed to create Signal client")?,
    );

    if dstack.is_in_tee().await {
        if let Ok(info) = dstack.get_app_info().await {
            info!(
                "Running in TEE - App ID: {}",
                info.app_id.as_deref().unwrap_or("unknown")
            );
        }
    } else {
        warn!("Not running in TEE environment - attestation unavailable");
    }

    if !signal.health_check().await {
        error!("Signal API not reachable at {}", config.signal.service_url);
        return Err(anyhow::anyhow!("Signal API not reachable").into());
    }
    info!("Signal API healthy");

    let bot_identity = BotIdentity::new();

    let handlers: Vec<Box<dyn CommandHandler>> = match config.bot.role {
        BotRole::Transcription => {
            build_transcription_handlers(&config, signal.clone(), dstack.clone()).await?
        }
        BotRole::Translation => {
            build_translation_handlers(
                &config,
                signal.clone(),
                dstack.clone(),
                bot_identity.clone(),
            )
            .await?
        }
    };

    info!("Registered {} command handlers", handlers.len());
    info!("Listening for messages...");

    let receiver = MessageReceiver::new((*signal).clone(), config.signal.poll_interval);
    let mut stream = Box::pin(receiver.stream());

    loop {
        tokio::select! {
            Some(message) = stream.next() => {
                bot_identity.note_inbound(&message);

                let handler = handlers
                    .iter()
                    .find(|h| h.matches(&message));

                if let Some(handler) = handler {
                    let quote_reply = handler.reply_with_quote();
                    let own_reply = handler.handles_own_reply();
                    debug!(
                        handler = handler.label(),
                        source = %message.source,
                        is_group = message.is_group,
                        voice = message.is_voice_note(),
                        has_quote = message.quote.is_some(),
                        own_reply,
                        quote_reply,
                        "Dispatching to handler"
                    );
                    match handler.execute(&message).await {
                        Ok(response) => {
                            if own_reply {
                                continue;
                            }
                            let send_result = if quote_reply {
                                signal.reply_quoted(&message, &response, None).await
                            } else {
                                signal.reply(&message, &response).await
                            };
                            if let Err(e) = send_result {
                                error!("Failed to send reply: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Handler error: {}", e);
                            if own_reply {
                                continue;
                            }
                            let fallback = "Sorry, something went wrong.";
                            let _ = if quote_reply {
                                signal.reply_quoted(&message, fallback, None).await
                            } else {
                                signal.reply(&message, fallback).await
                            };
                        }
                    }
                } else if message.is_voice_note() || !message.text.trim().is_empty() {
                    debug!(
                        source = %message.source,
                        is_group = message.is_group,
                        voice = message.is_voice_note(),
                        "No handler matched message"
                    );
                }
            }
            _ = signal::ctrl_c() => {
                info!("Shutdown signal received");
                break;
            }
        }
    }

    info!("Shutting down...");
    Ok(())
}

async fn build_transcription_handlers(
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

    let transcribe_store = Arc::new(TranscribeStore::new(Some(group_prefs.clone())));
    let voice_cache = VoiceAttachmentCache::with_default_capacity();

    let mut handlers: Vec<Box<dyn CommandHandler>> = Vec::new();

    // Voice only — no NEAR translate-on-voice; translation CVM handles posted text.
    handlers.push(Box::new(
        VoiceHandler::new(
            whisper.clone(),
            signal.clone(),
            config.whisper.reply_prefix.clone(),
            config.whisper.max_attachment_bytes,
        )
        .with_transcribe_store(transcribe_store.clone())
        .with_voice_cache(voice_cache.clone()),
    ));
    handlers.push(Box::new(ManualTranscribeHandler::new(
        whisper.clone(),
        signal.clone(),
        config.whisper.reply_prefix.clone(),
        config.whisper.max_attachment_bytes,
        voice_cache,
    )));
    handlers.push(Box::new(TranscribeHandler::new(transcribe_store, true)));
    handlers.push(Box::new(VerifyHandler::new(dstack)));
    handlers.push(Box::new(HelpHandler::new(
        group_prefs.clone(),
        BotRole::Transcription,
    )));
    handlers.push(Box::new(PrivacyHandler::new(
        group_prefs,
        BotRole::Transcription,
    )));

    info!("Transcription role: voice / !transcribe* / help / privacy / verify");
    Ok(handlers)
}

async fn build_translation_handlers(
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
        "Language Threads enabled: !translate-me-on / !translate-me-off (max {}/min)",
        config.translate_all.max_messages_per_minute
    );

    if config.translate_all.enabled {
        handlers.push(Box::new(TranslateAllHandler::new(
            group_prefs.clone(),
            near_ai.clone(),
            signal.clone(),
        )));
        info!("In-chat translation enabled: !translate-on / !translate-off");
    }

    handlers.push(Box::new(TranslateHandler::new(
        near_ai.clone(),
        signal.clone(),
        "📝 Transcript:",
    )));
    handlers.push(Box::new(TranslateLangsHandler::new()));
    handlers.push(Box::new(SetLanguageHandler::new(group_prefs.clone())));
    handlers.push(Box::new(VerifyHandler::new(dstack)));
    handlers.push(Box::new(HelpHandler::new(
        group_prefs.clone(),
        BotRole::Translation,
    )));
    handlers.push(Box::new(PrivacyHandler::new(
        group_prefs,
        BotRole::Translation,
    )));
    handlers.push(Box::new(ModelsHandler::new(near_ai)));

    info!("Translation role: Language Threads + in-chat + quote !translate");
    Ok(handlers)
}

fn init_logging(level: &str) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}
