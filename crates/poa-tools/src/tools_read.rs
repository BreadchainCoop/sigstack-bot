//! Read-only Poa tools (no authorization required).

use crate::client::PoaClient;
use crate::subgraph::TaskInfo;
use crate::units::format_pt;
use async_trait::async_trait;
use serde::Deserialize;
use std::sync::Arc;
use tools::{FunctionDefinition, Tool, ToolDefinition, ToolError};

fn map_err(e: crate::client::PoaError) -> ToolError {
    match e {
        crate::client::PoaError::InvalidArguments(m) => ToolError::InvalidArguments(m),
        other => ToolError::ExternalService(other.to_string()),
    }
}

fn format_task_line(t: &TaskInfo) -> String {
    let mut line = format!(
        "#{} [{}] {} — payout {} PT (project: {})",
        t.task_id,
        t.status,
        t.title,
        format_pt(&t.payout),
        t.project_title
    );
    if let Some(user) = t.assignee_username.as_ref().or(t.assignee.as_ref()) {
        line.push_str(&format!(" — assignee: {}", user));
    }
    line
}

fn format_task_detail(t: &TaskInfo) -> String {
    let mut out = format!(
        "Task #{}\n\
         Title: {}\n\
         Status: {}\n\
         Payout: {} PT (raw wei: {})\n\
         Project: {} (project_id: {})\n\
         Metadata hash: {}\n\
         Requires application: {}\n",
        t.task_id,
        t.title,
        t.status,
        format_pt(&t.payout),
        t.payout,
        t.project_title,
        t.project_id,
        t.metadata_hash,
        t.requires_application,
    );
    if let Some(user) = t.assignee_username.as_ref().or(t.assignee.as_ref()) {
        out.push_str(&format!("Assignee: {}\n", user));
    }
    if t.bounty_token != "0x0000000000000000000000000000000000000000" && !t.bounty_token.is_empty()
    {
        out.push_str(&format!(
            "Bounty: {} (raw) of token {}\n",
            t.bounty_payout, t.bounty_token
        ));
    }
    if let Some(d) = &t.absolute_deadline {
        out.push_str(&format!("Absolute deadline (unix): {}\n", d));
    }
    if let Some(d) = &t.claim_deadline {
        out.push_str(&format!("Current claim deadline (unix): {}\n", d));
    }
    if let Some(w) = &t.completion_window {
        out.push_str(&format!("Completion window (seconds): {}\n", w));
    }
    out
}

/// List the org's projects.
pub struct PoaListProjectsTool {
    client: Arc<PoaClient>,
}

impl PoaListProjectsTool {
    pub fn new(client: Arc<PoaClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for PoaListProjectsTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            tool_type: "function".into(),
            function: FunctionDefinition {
                name: "poa_list_projects".into(),
                description: "List the Poa organization's projects (id, title, task count). \
                              Use this first to find the project_id needed for other poa tools."
                    .into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
        }
    }

    fn name(&self) -> &str {
        "poa_list_projects"
    }

    async fn execute(&self, _arguments: &str) -> Result<String, ToolError> {
        let projects = self.client.list_projects().await.map_err(map_err)?;
        if projects.is_empty() {
            return Ok("No projects found for this organization.".into());
        }
        let mut out = format!("Projects ({}):\n", projects.len());
        for p in projects {
            out.push_str(&format!(
                "- {} — {} task(s), PT cap: {} (project_id: {})\n",
                p.title,
                p.task_count,
                if p.cap == "0" {
                    "unlimited".to_string()
                } else {
                    format_pt(&p.cap)
                },
                p.project_id
            ));
        }
        Ok(out)
    }
}

#[derive(Deserialize)]
struct ListTasksArgs {
    status: Option<String>,
    project_id: Option<String>,
}

/// List tasks, optionally filtered by status / project.
pub struct PoaListTasksTool {
    client: Arc<PoaClient>,
}

impl PoaListTasksTool {
    pub fn new(client: Arc<PoaClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for PoaListTasksTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            tool_type: "function".into(),
            function: FunctionDefinition {
                name: "poa_list_tasks".into(),
                description: "List tasks in the Poa organization, newest first. \
                              Optionally filter by status and/or project."
                    .into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "status": {
                            "type": "string",
                            "enum": ["open", "assigned", "submitted", "completed", "cancelled"],
                            "description": "Filter by task status"
                        },
                        "project_id": {
                            "type": "string",
                            "description": "Filter by project id (0x-prefixed bytes32 from poa_list_projects)"
                        }
                    },
                    "required": []
                }),
            },
        }
    }

    fn name(&self) -> &str {
        "poa_list_tasks"
    }

    async fn execute(&self, arguments: &str) -> Result<String, ToolError> {
        let args: ListTasksArgs = serde_json::from_str(arguments)
            .map_err(|e| ToolError::InvalidArguments(e.to_string()))?;
        let tasks = self
            .client
            .list_tasks(args.status.as_deref(), args.project_id.as_deref())
            .await
            .map_err(map_err)?;
        if tasks.is_empty() {
            return Ok("No tasks matched.".into());
        }
        let mut out = format!("Tasks ({}):\n", tasks.len());
        for t in tasks.iter().take(50) {
            out.push_str(&format_task_line(t));
            out.push('\n');
        }
        if tasks.len() > 50 {
            out.push_str(&format!("...and {} more (use filters)\n", tasks.len() - 50));
        }
        Ok(out)
    }
}

#[derive(Deserialize)]
struct GetTaskArgs {
    task_id: u64,
}

/// Get full details of a single task.
pub struct PoaGetTaskTool {
    client: Arc<PoaClient>,
}

impl PoaGetTaskTool {
    pub fn new(client: Arc<PoaClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for PoaGetTaskTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            tool_type: "function".into(),
            function: FunctionDefinition {
                name: "poa_get_task".into(),
                description: "Get full details of one Poa task by numeric id, including the \
                              current payout, metadata hash, bounty and deadlines. Always call \
                              this before poa_update_task so unchanged fields can be passed back."
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
        "poa_get_task"
    }

    async fn execute(&self, arguments: &str) -> Result<String, ToolError> {
        let args: GetTaskArgs = serde_json::from_str(arguments)
            .map_err(|e| ToolError::InvalidArguments(e.to_string()))?;
        match self.client.get_task(args.task_id).await.map_err(map_err)? {
            Some(t) => Ok(format_task_detail(&t)),
            None => Ok(format!("Task #{} not found.", args.task_id)),
        }
    }
}

/// Show the bot's on-chain identity and gas balance.
pub struct PoaWalletInfoTool {
    client: Arc<PoaClient>,
}

impl PoaWalletInfoTool {
    pub fn new(client: Arc<PoaClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for PoaWalletInfoTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            tool_type: "function".into(),
            function: FunctionDefinition {
                name: "poa_wallet_info".into(),
                description: "Show the bot's Poa wallet address, gas balance and configured \
                              TaskManager. The wallet address is what an org must grant \
                              project-manager rights to before write tools work."
                    .into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
        }
    }

    fn name(&self) -> &str {
        "poa_wallet_info"
    }

    async fn execute(&self, _arguments: &str) -> Result<String, ToolError> {
        let balance = self.client.native_balance().await.map_err(map_err)?;
        Ok(format!(
            "Bot wallet: {}\n\
             Network: {}\n\
             Native balance: {} wei ({} in whole tokens)\n\
             TaskManager: {}",
            self.client.address(),
            self.client.config.network_name,
            balance,
            format_pt(&balance.to_string()),
            self.client.config.task_manager,
        ))
    }
}
