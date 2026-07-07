//! Types for the pacto-bot-api JSON-RPC contract (schemas/jsonrpc.json).

use serde::Deserialize;

/// Result of `handler.register`.
#[derive(Debug, Clone, Deserialize)]
pub struct Registration {
    pub handler_id: String,
    pub reconnect_token: String,
    pub registered_events: Vec<String>,
}

/// Result of `agent.version`.
#[derive(Debug, Clone, Deserialize)]
pub struct DaemonVersion {
    pub version: String,
    #[serde(default)]
    pub commit: Option<String>,
}
