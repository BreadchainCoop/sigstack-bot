//! Tool type definitions following OpenAI function calling schema.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::error::ToolError;

/// Tool definition sent to LLM (OpenAI-compatible schema).
#[derive(Debug, Clone, Serialize)]
pub struct ToolDefinition {
    /// Always "function".
    #[serde(rename = "type")]
    pub tool_type: String,
    /// Function details.
    pub function: FunctionDefinition,
}

/// Function definition within a tool.
#[derive(Debug, Clone, Serialize)]
pub struct FunctionDefinition {
    /// Function name (e.g., "web_search").
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// JSON Schema for parameters.
    pub parameters: serde_json::Value,
}

/// Tool call requested by LLM.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolCall {
    /// Unique ID for this call.
    pub id: String,
    /// Always "function".
    #[serde(rename = "type")]
    pub call_type: String,
    /// Function to call.
    pub function: FunctionCall,
}

/// Function call details.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FunctionCall {
    /// Function name.
    pub name: String,
    /// JSON string of arguments.
    pub arguments: String,
}

/// Result of executing a tool.
#[derive(Debug, Clone)]
pub struct ToolResult {
    /// ID of the tool call this responds to.
    pub tool_call_id: String,
    /// Result content (or error message).
    pub content: String,
    /// Whether execution succeeded.
    pub success: bool,
}

impl ToolResult {
    /// Create a successful result.
    pub fn success(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            content: content.into(),
            success: true,
        }
    }

    /// Create an error result.
    pub fn error(tool_call_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            content: message.into(),
            success: false,
        }
    }
}

/// Context passed to a tool at execution time.
///
/// Carries who is asking and whether a prior confirmation step has cleared, so
/// tools can implement per-sender behavior (allowlists, two-step confirmation)
/// without the trait leaking bot internals.
#[derive(Debug, Clone, Default)]
pub struct ToolContext {
    /// Signal sender id (phone number) of the human driving this turn.
    pub sender: String,
    /// Whether the sender is authorized for privileged tools.
    pub authorized: bool,
    /// Whether a required confirmation has already been satisfied for this call
    /// (set true when re-dispatched by the `!poa-confirm` flow).
    pub confirmed: bool,
}

/// Trait for implementing tools.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get the tool definition for the LLM.
    fn definition(&self) -> ToolDefinition;

    /// Get the tool name.
    fn name(&self) -> &str;

    /// Execute the tool with JSON arguments.
    async fn execute(&self, arguments: &str) -> Result<String, ToolError>;

    /// Execute with execution context (sender, authorization, confirmation).
    ///
    /// Defaults to ignoring the context and calling [`Tool::execute`]. Tools
    /// that need the sender or a confirmation gate override this.
    async fn execute_ctx(
        &self,
        arguments: &str,
        _ctx: &ToolContext,
    ) -> Result<String, ToolError> {
        self.execute(arguments).await
    }

    /// Whether this tool performs a privileged/state-changing action that must
    /// only be offered to and executed for authorized senders.
    ///
    /// Read-only tools return `false` (the default). Tools that move funds or
    /// mutate on-chain / external state (e.g. Poa task writes) return `true`;
    /// the bot then filters them by its sender allowlist before offering them
    /// to the model or running them.
    fn requires_authorization(&self) -> bool {
        false
    }

    /// Whether this tool must be confirmed via a second, deterministic step
    /// before it actually runs (e.g. value-moving actions like minting a
    /// payout). The bot mediates the confirmation; the tool sees `confirmed`
    /// on the [`ToolContext`].
    fn requires_confirmation(&self) -> bool {
        false
    }

    /// Optional per-tool execution timeout in seconds, overriding the executor
    /// default. On-chain writes need longer than typical HTTP tools because the
    /// call has to be mined and confirmed.
    fn timeout_override(&self) -> Option<u64> {
        None
    }
}
