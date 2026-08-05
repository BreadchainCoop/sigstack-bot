//! Signal product bots — transcription or translation (see `BOT__ROLE`).

use anyhow::Context;
use dstack_client::DstackClient;
use signal_bot::bot_identity::BotIdentity;
use signal_bot::config::Config;
use signal_bot::dispatch::dispatch_message;
use signal_bot::error::AppResult;
use signal_bot::group_invite_acceptor::{
    run_invite_acceptor, InvitePolicy, DEFAULT_INVITE_POLL_INTERVAL,
};
use signal_bot::handlers_setup::build_handlers;
use signal_client::{MessageReceiver, SignalClient};
use std::sync::Arc;
use tokio::signal;
use tokio_stream::StreamExt;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

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
        SignalClient::new(&config.signal.service_url).context("Failed to create Signal client")?,
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

    if let (Some(self_phone), Some(peer_raw)) = (
        config.signal.phone_number.as_deref(),
        config.signal.peer_phone.as_deref(),
    ) {
        let peer = peer_raw.trim();
        if !peer.is_empty() {
            match signal.trust_identity(self_phone, peer).await {
                Ok(()) => info!(peer, "Trusted Signal peer identity (PEER_PHONE)"),
                Err(e) => warn!(
                    peer,
                    error = %e,
                    "Could not trust PEER_PHONE identity — peer messages may not decrypt until trusted"
                ),
            }
        }
    }

    let bot_identity = BotIdentity::new();

    let handlers = build_handlers(
        &config,
        signal.clone(),
        dstack.clone(),
        bot_identity.clone(),
    )
    .await?;

    info!("Registered {} command handlers", handlers.len());

    match InvitePolicy::for_role(config.bot.role, config.signal.peer_phone.as_deref()) {
        Some(policy) => {
            let signal_invites = signal.clone();
            let phone = config.signal.phone_number.clone();
            tokio::spawn(async move {
                run_invite_acceptor(signal_invites, phone, policy, DEFAULT_INVITE_POLL_INTERVAL)
                    .await;
            });
        }
        None => {
            warn!(
                "Group invite auto-accept disabled (transcription requires SIGNAL__PEER_PHONE = translation bot)"
            );
        }
    }

    info!("Listening for messages...");

    let receiver = MessageReceiver::new((*signal).clone(), config.signal.poll_interval);
    let mut stream = Box::pin(receiver.stream());

    loop {
        tokio::select! {
            Some(message) = stream.next() => {
                let _ = dispatch_message(&handlers, &signal, &bot_identity, &message).await;
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
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}
