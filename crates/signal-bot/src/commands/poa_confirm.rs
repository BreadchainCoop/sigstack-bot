//! `!poa-confirm <code>` — the deterministic second step for value-moving Poa
//! tool calls (e.g. approving a task, which mints a payout).
//!
//! When a tool that [`tools::Tool::requires_confirmation`] is invoked in chat it
//! stages the call and replies with a code. The operator confirms by sending
//! `!poa-confirm <code>`; this handler re-dispatches the staged call with
//! `confirmed = true`. Confirmation is only accepted from a still-authorized
//! sender for their own staged action.

use crate::commands::{CommandHandler, ToolAuthorization};
use crate::error::AppResult;
use async_trait::async_trait;
use signal_client::BotMessage;
use std::sync::Arc;
use tools::{ConfirmationStore, FunctionCall, ToolCall, ToolContext, ToolExecutor, ToolRegistry};
use tracing::{info, warn};

pub struct PoaConfirmHandler {
    registry: Arc<ToolRegistry>,
    confirm: Arc<ConfirmationStore>,
    authorization: ToolAuthorization,
}

impl PoaConfirmHandler {
    pub fn new(
        registry: Arc<ToolRegistry>,
        confirm: Arc<ConfirmationStore>,
        authorization: ToolAuthorization,
    ) -> Self {
        Self {
            registry,
            confirm,
            authorization,
        }
    }
}

#[async_trait]
impl CommandHandler for PoaConfirmHandler {
    fn trigger(&self) -> Option<&str> {
        Some("!poa-confirm")
    }

    async fn execute(&self, message: &BotMessage) -> AppResult<String> {
        // Re-verify authorization at confirm time (defense in depth).
        if !self.authorization.is_authorized(&message.source) {
            return Ok("You are not authorized to confirm Poa actions.".into());
        }

        let code = message
            .text
            .trim_start_matches("!poa-confirm")
            .trim()
            .to_string();
        if code.is_empty() {
            return Ok("Usage: !poa-confirm <code>".into());
        }

        let Some(pending) = self.confirm.take(&message.source, &code) else {
            return Ok("No matching pending action (wrong code, already used, or expired).".into());
        };

        info!(
            "Confirming staged Poa action '{}' for {}",
            pending.tool_name,
            &message.source[..message.source.len().min(8)]
        );

        let call = ToolCall {
            id: format!("confirm-{}", code),
            call_type: "function".into(),
            function: FunctionCall {
                name: pending.tool_name.clone(),
                arguments: pending.arguments.clone(),
            },
        };
        let ctx = ToolContext {
            sender: message.source.clone(),
            authorized: true,
            confirmed: true,
        };

        let executor = ToolExecutor::new(self.registry.clone());
        let result = executor.execute_ctx(&call, &ctx).await;
        if !result.success {
            warn!(
                "Confirmed action '{}' failed: {}",
                pending.tool_name, result.content
            );
        }
        Ok(result.content)
    }
}
