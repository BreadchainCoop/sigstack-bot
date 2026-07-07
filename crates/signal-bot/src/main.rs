//! Signal AI Proxy Bot - Main entry point.

use signal_bot::commands::*;
use signal_bot::config::Config;
use signal_bot::error::AppResult;
use signal_bot::group_preferences_store::GroupPreferencesStore;
use signal_bot::transcribe_store::TranscribeStore;
use signal_bot::voice_attachment_cache::VoiceAttachmentCache;
use anyhow::Context;
use conversation_store::ConversationStore;
use dstack_client::DstackClient;
use near_ai_client::NearAiClient;
use pacto_client::{PactoAgent, PactoClient};
use signal_bot::pacto_agent;
use signal_client::{MessageReceiver, SignalClient};
use whisper_client::WhisperClient;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::signal;
use tokio_stream::StreamExt;
use tools::{ToolRegistry, builtin::{CalculatorTool, WeatherTool, WebSearchTool}};
use tracing::{debug, error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use x402_payments::CreditStore;

/// Create and configure tool registry based on config.
fn create_tool_registry(config: &signal_bot::config::ToolsConfig) -> ToolRegistry {
    let mut registry = ToolRegistry::new();

    if !config.enabled {
        info!("Tools system disabled by configuration");
        return registry;
    }

    // Calculator - always available (no API key needed)
    if config.calculator.enabled {
        registry.register(Arc::new(CalculatorTool::new()));
        info!("Registered tool: calculate");
    }

    // Weather - always available (no API key needed)
    if config.weather.enabled {
        registry.register(Arc::new(WeatherTool::new()));
        info!("Registered tool: get_weather");
    }

    // Web search - requires API key
    if config.web_search.enabled {
        if let Some(api_key) = &config.web_search.api_key {
            let tool = WebSearchTool::new(api_key.clone())
                .with_max_results(config.web_search.max_results);
            registry.register(Arc::new(tool));
            info!("Registered tool: web_search (max_results: {})", config.web_search.max_results);
        } else {
            warn!("Web search tool enabled but TOOLS__WEB_SEARCH__API_KEY not set - skipping");
        }
    }

    let enabled_count = registry.list_enabled().len();
    info!("Tool registry ready with {} enabled tools", enabled_count);

    registry
}

#[tokio::main]
async fn main() -> AppResult<()> {
    // Load configuration
    let config = Config::load().context("Failed to load configuration")?;

    // Initialize logging
    init_logging(&config.bot.log_level);

    info!("Starting Signal AI Proxy Bot...");

    // Initialize clients
    let near_ai = Arc::new(
        NearAiClient::new(
            &config.near_ai.api_key,
            &config.near_ai.base_url,
            &config.near_ai.model,
            config.near_ai.timeout,
        )
        .context("Failed to create NEAR AI client")?,
    );

    let conversations = Arc::new(ConversationStore::new(
        config.conversation.max_messages,
        config.conversation.ttl,
    ));

    let dstack = Arc::new(DstackClient::new(&config.dstack.socket_path));

    let signal = Arc::new(
        SignalClient::new(&config.signal.service_url)
            .context("Failed to create Signal client")?,
    );

    // Create tool registry based on config
    let tool_registry = Arc::new(create_tool_registry(&config.tools));

    // Initialize payment system
    let credit_store = if config.payments.enabled {
        info!("Initializing payment system...");

        // Create separate DstackClient instances for payment system
        let payment_dstack = DstackClient::new(&config.dstack.socket_path);
        let server_dstack = DstackClient::new(&config.dstack.socket_path);

        let store = CreditStore::new(
            payment_dstack,
            config.payments.storage_path.clone(),
        )
        .await
        .context("Failed to initialize credit store")?;

        // Spawn payment HTTP server
        if let Some(handle) = x402_payments::spawn_payment_server(
            config.payments.clone(),
            server_dstack,
        )
        .await
        .context("Failed to start payment server")? {
            info!("Payment server started on port {}", config.payments.server_port);
            // Store handle to keep server running (we don't await it)
            tokio::spawn(async move {
                if let Err(e) = handle.await {
                    error!("Payment server error: {:?}", e);
                }
            });
        }

        Some(store)
    } else {
        info!("Payments disabled");
        None
    };

    // Health checks
    if near_ai.health_check().await {
        info!("NEAR AI healthy - Model: {}", config.near_ai.model);
    } else {
        warn!("NEAR AI health check failed - will retry on requests");
    }

    info!(
        "In-memory conversation store ready (max_messages={}, ttl={:?})",
        config.conversation.max_messages, config.conversation.ttl
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

    let pacto_client = if config.pacto.enabled {
        let pacto = Arc::new(PactoClient::new(
            &config.pacto.socket_path,
            config.pacto.bot_id.clone(),
            config.pacto.timeout,
        ));

        match pacto.version().await {
            Ok(v) => info!(
                "Pacto daemon healthy (v{}) - sending as bot '{}'",
                v.version,
                pacto.bot_id()
            ),
            Err(e) => warn!(
                "Pacto daemon not reachable at {} ({e}) - !pact will retry on use",
                config.pacto.socket_path
            ),
        }

        Some(pacto)
    } else {
        info!("Pacto messaging disabled");
        None
    };

    // Inbound Pacto DM agent — gives Pacto users the same DM experience as
    // Signal users (AI chat + core commands). Outbound `!pact` still works
    // without it; disable via PACTO__AGENT_ENABLED=false.
    if let Some(ref pacto) = pacto_client {
        if config.pacto.agent_enabled {
            let (agent, inbound) = PactoAgent::spawn(
                config.pacto.socket_path.clone(),
                config.pacto.bot_id.clone(),
                std::time::Duration::from_secs(3),
            );
            pacto_agent::spawn(
                inbound,
                agent,
                pacto_agent::PactoAgentDeps {
                    near_ai: near_ai.clone(),
                    conversations: conversations.clone(),
                    tool_registry: tool_registry.clone(),
                    dstack: dstack.clone(),
                    pacto_client: pacto.clone(),
                    system_prompt: config.bot.system_prompt.clone(),
                    max_tool_calls: config.tools.max_tool_calls,
                    signal_username: config.bot.signal_username.clone(),
                    github_repo: config.bot.github_repo.clone(),
                },
            );
            info!("Pacto DM agent enabled: Pacto users get AI chat + commands");
        } else {
            info!("Pacto DM agent disabled (outbound !pact only)");
        }
    }

    let whisper_client = if config.whisper.enabled {
        let whisper = Arc::new(
            WhisperClient::new(
                &config.whisper.service_url,
                config.whisper.timeout,
            )
            .context("Failed to create Whisper client")?,
        );

        if whisper.health_check().await {
            info!(
                "Whisper API healthy at {} (model: {})",
                config.whisper.service_url, config.whisper.model
            );
        } else {
            warn!(
                "Whisper API not reachable at {} — voice notes will fail until whisper-api is up",
                config.whisper.service_url
            );
        }

        Some(whisper)
    } else {
        info!("Whisper transcription disabled");
        None
    };

    // Create command handlers
    let chat = if let Some(ref store) = credit_store {
        ChatHandler::with_payments(
            near_ai.clone(),
            conversations.clone(),
            signal.clone(),
            tool_registry.clone(),
            config.bot.system_prompt.clone(),
            config.tools.max_tool_calls,
            config.bot.signal_username.clone(),
            config.bot.github_repo.clone(),
            store.clone(),
            config.payments.pricing.clone(),
        )
    } else {
        ChatHandler::new(
            near_ai.clone(),
            conversations.clone(),
            signal.clone(),
            tool_registry.clone(),
            config.bot.system_prompt.clone(),
            config.tools.max_tool_calls,
            config.bot.signal_username.clone(),
            config.bot.github_repo.clone(),
        )
    };

    let mut handlers: Vec<Box<dyn CommandHandler>> = Vec::new();

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

    let transcribe_store = Arc::new(TranscribeStore::new(Some(group_prefs.clone())));
    let whisper_available = whisper_client.is_some();

    let voice_attachment_cache = VoiceAttachmentCache::with_default_capacity();

    if let Some(ref whisper) = whisper_client {
        let mut voice = VoiceHandler::new(
            whisper.clone(),
            signal.clone(),
            config.whisper.reply_prefix.clone(),
            config.whisper.max_attachment_bytes,
        )
        .with_transcribe_store(transcribe_store.clone());
        if config.translate_all.enabled {
            voice = voice.with_translate_all(group_prefs.clone(), near_ai.clone());
        }
        handlers.push(Box::new(voice));
        handlers.push(Box::new(ManualTranscribeHandler::new(
            whisper.clone(),
            signal.clone(),
            config.whisper.reply_prefix.clone(),
            config.whisper.max_attachment_bytes,
            voice_attachment_cache.clone(),
        )));
        info!("Voice note transcription enabled");
    }

    handlers.push(Box::new(TranscribeHandler::new(
        transcribe_store,
        whisper_available,
    )));

    if config.translate_all.enabled {
        handlers.push(Box::new(TranslateAllHandler::new(
            group_prefs.clone(),
            near_ai.clone(),
            signal.clone(),
        )));
        info!(
            "Group auto-translate enabled: !translate-on, !translate-off (max {}/min)",
            config.translate_all.max_messages_per_minute
        );
    }

    handlers.push(Box::new(TranslateHandler::new(
        near_ai.clone(),
        signal.clone(),
        config.whisper.reply_prefix.clone(),
    )));
    handlers.push(Box::new(TranslateLangsHandler::new()));
    handlers.push(Box::new(AskHandler::new(chat.clone())));
    info!("AI chat: DM free-text; groups use !ask");
    handlers.push(Box::new(chat));
    handlers.push(Box::new(VerifyHandler::new(dstack.clone())));
    handlers.push(Box::new(ClearHandler::new(conversations.clone())));
    handlers.push(Box::new(SetLanguageHandler::new(group_prefs.clone())));
    handlers.push(Box::new(HelpHandler::new(group_prefs.clone())));
    handlers.push(Box::new(PrivacyHandler::new(group_prefs.clone())));
    handlers.push(Box::new(ModelsHandler::new(near_ai.clone())));

    // Add payment handlers if enabled
    if let Some(ref store) = credit_store {
        handlers.push(Box::new(BalanceHandler::new(store.clone())));
        handlers.push(Box::new(DepositHandler::new(config.payments.clone())));
        info!("Payment commands enabled: !balance, !deposit");
    }

    // Add Pacto messaging if enabled
    if let Some(ref pacto) = pacto_client {
        let default_recipient = config
            .pacto
            .default_recipient
            .clone()
            .filter(|r| !r.trim().is_empty());
        handlers.push(Box::new(PactHandler::new(pacto.clone(), default_recipient)));
        info!("Pacto messaging enabled: !pact");
    }

    info!("Registered {} command handlers", handlers.len());
    info!("NEAR AI endpoint: {}", config.near_ai.base_url);
    info!("Listening for messages...");

    // Start message receiver
    let receiver = MessageReceiver::new((*signal).clone(), config.signal.poll_interval);
    let mut stream = Box::pin(receiver.stream());

    // Main message loop
    loop {
        tokio::select! {
            Some(message) = stream.next() => {
                if let Some(audio) = message.primary_audio_attachment() {
                    voice_attachment_cache.remember(
                        message.reply_target(),
                        message.message_timestamp,
                        audio.clone(),
                    );
                }

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

fn init_logging(level: &str) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}
