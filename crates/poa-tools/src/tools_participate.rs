//! Participation Poa tools — the bot doing work itself (claim / submit / apply).
//!
//! These let an authorized operator direct the bot to claim a task, deliver a
//! submission, or apply for an application-gated task, all signed by the bot
//! wallet. When the work is later approved the bot *earns* participation
//! tokens, making it a first-class contributor rather than only an organizer.

use crate::client::PoaClient;
use crate::common::{map_err, parse_metadata_hash, parse_task_id};
use async_trait::async_trait;
use serde::Deserialize;
use std::sync::Arc;
use tools::{FunctionDefinition, Tool, ToolDefinition, ToolError};

const WRITE_TIMEOUT: u64 = 90;

#[derive(Deserialize)]
struct TaskIdArg {
    task_id: u64,
}

#[derive(Deserialize)]
struct TaskWithHashArg {
    task_id: u64,
    /// Free text (sha256-hashed on chain) or a 0x bytes32 CID.
    content: String,
}

/// Claim an unclaimed (or expired-claim) task for the bot wallet.
pub struct PoaClaimTaskTool {
    client: Arc<PoaClient>,
}

impl PoaClaimTaskTool {
    pub fn new(client: Arc<PoaClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for PoaClaimTaskTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            tool_type: "function".into(),
            function: FunctionDefinition {
                name: "poa_claim_task".into(),
                description: "Claim an UNCLAIMED task (or take over an expired claim) for the bot \
                              wallet, committing the bot to do the work. Requires CLAIM rights. \
                              Use poa_submit_task afterwards to deliver."
                    .into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "task_id": { "type": "integer", "description": "Numeric task id" }
                    },
                    "required": ["task_id"]
                }),
            },
        }
    }

    fn name(&self) -> &str {
        "poa_claim_task"
    }

    fn requires_authorization(&self) -> bool {
        true
    }

    fn timeout_override(&self) -> Option<u64> {
        Some(WRITE_TIMEOUT)
    }

    async fn execute(&self, arguments: &str) -> Result<String, ToolError> {
        let args: TaskIdArg = serde_json::from_str(arguments)
            .map_err(|e| ToolError::InvalidArguments(e.to_string()))?;
        let outcome = self
            .client
            .claim_task(parse_task_id(args.task_id))
            .await
            .map_err(map_err)?;
        Ok(format!(
            "Claimed task #{} for the bot wallet. Transaction {} confirmed.",
            args.task_id, outcome.hash
        ))
    }
}

/// Submit finished work for a task the bot has claimed.
pub struct PoaSubmitTaskTool {
    client: Arc<PoaClient>,
}

impl PoaSubmitTaskTool {
    pub fn new(client: Arc<PoaClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for PoaSubmitTaskTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            tool_type: "function".into(),
            function: FunctionDefinition {
                name: "poa_submit_task".into(),
                description: "Submit finished work for a task the bot currently claims. The \
                              content is sha256-hashed on chain (or pass a 0x bytes32 CID). The \
                              task must be CLAIMED by the bot wallet."
                    .into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "task_id": { "type": "integer", "description": "Numeric task id" },
                        "content": { "type": "string", "description": "Submission text or 0x bytes32 CID" }
                    },
                    "required": ["task_id", "content"]
                }),
            },
        }
    }

    fn name(&self) -> &str {
        "poa_submit_task"
    }

    fn requires_authorization(&self) -> bool {
        true
    }

    fn timeout_override(&self) -> Option<u64> {
        Some(WRITE_TIMEOUT)
    }

    async fn execute(&self, arguments: &str) -> Result<String, ToolError> {
        let args: TaskWithHashArg = serde_json::from_str(arguments)
            .map_err(|e| ToolError::InvalidArguments(e.to_string()))?;
        let hash = parse_metadata_hash(&args.content);
        let outcome = self
            .client
            .submit_task(parse_task_id(args.task_id), hash)
            .await
            .map_err(map_err)?;
        Ok(format!(
            "Submitted work for task #{}. Transaction {} confirmed.",
            args.task_id, outcome.hash
        ))
    }
}

/// Apply for an application-gated task.
pub struct PoaApplyForTaskTool {
    client: Arc<PoaClient>,
}

impl PoaApplyForTaskTool {
    pub fn new(client: Arc<PoaClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for PoaApplyForTaskTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            tool_type: "function".into(),
            function: FunctionDefinition {
                name: "poa_apply_for_task".into(),
                description: "Apply for a task that requires an application. The application \
                              content is sha256-hashed on chain (or pass a 0x bytes32 CID). A \
                              project manager must approve before the bot is assigned."
                    .into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "task_id": { "type": "integer", "description": "Numeric task id" },
                        "content": { "type": "string", "description": "Application text or 0x bytes32 CID" }
                    },
                    "required": ["task_id", "content"]
                }),
            },
        }
    }

    fn name(&self) -> &str {
        "poa_apply_for_task"
    }

    fn requires_authorization(&self) -> bool {
        true
    }

    fn timeout_override(&self) -> Option<u64> {
        Some(WRITE_TIMEOUT)
    }

    async fn execute(&self, arguments: &str) -> Result<String, ToolError> {
        let args: TaskWithHashArg = serde_json::from_str(arguments)
            .map_err(|e| ToolError::InvalidArguments(e.to_string()))?;
        let hash = parse_metadata_hash(&args.content);
        let outcome = self
            .client
            .apply_for_task(parse_task_id(args.task_id), hash)
            .await
            .map_err(map_err)?;
        Ok(format!(
            "Applied for task #{}. Transaction {} confirmed.",
            args.task_id, outcome.hash
        ))
    }
}
