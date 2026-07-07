//! Read-side queries against the Poa subgraph.

use crate::client::{PoaClient, PoaError};
use serde_json::{json, Value};

/// A project row from the subgraph.
#[derive(Debug, Clone)]
pub struct ProjectInfo {
    pub project_id: String,
    pub title: String,
    pub cap: String,
    pub task_count: usize,
}

/// A task row from the subgraph.
#[derive(Debug, Clone)]
pub struct TaskInfo {
    pub task_id: String,
    pub title: String,
    pub status: String,
    pub payout: String,
    pub metadata_hash: String,
    pub bounty_token: String,
    pub bounty_payout: String,
    pub assignee: Option<String>,
    pub assignee_username: Option<String>,
    pub project_id: String,
    pub project_title: String,
    pub requires_application: bool,
    pub absolute_deadline: Option<String>,
    pub claim_deadline: Option<String>,
    pub completion_window: Option<String>,
}

/// Map a user-facing status filter to the subgraph enum value.
pub fn subgraph_status(user_status: &str) -> Result<&'static str, PoaError> {
    match user_status.to_ascii_lowercase().as_str() {
        "open" | "unclaimed" => Ok("Open"),
        "assigned" | "claimed" => Ok("Assigned"),
        "submitted" => Ok("Submitted"),
        "completed" => Ok("Completed"),
        "cancelled" | "canceled" => Ok("Cancelled"),
        other => Err(PoaError::InvalidArguments(format!(
            "unknown status '{}' (use open, assigned, submitted, completed, or cancelled)",
            other
        ))),
    }
}

fn str_field(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn opt_str_field(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|s| !s.is_empty())
}

fn task_from_json(t: &Value) -> TaskInfo {
    TaskInfo {
        task_id: str_field(t, "taskId"),
        title: str_field(t, "title"),
        status: str_field(t, "status"),
        payout: str_field(t, "payout"),
        metadata_hash: str_field(t, "metadataHash"),
        bounty_token: str_field(t, "bountyToken"),
        bounty_payout: str_field(t, "bountyPayout"),
        assignee: opt_str_field(t, "assignee"),
        assignee_username: opt_str_field(t, "assigneeUsername"),
        project_id: t
            .get("project")
            .map(|p| str_field(p, "projectId"))
            .unwrap_or_default(),
        project_title: t
            .get("project")
            .map(|p| str_field(p, "title"))
            .unwrap_or_default(),
        requires_application: t
            .get("requiresApplication")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        absolute_deadline: opt_str_field(t, "absoluteDeadline"),
        claim_deadline: opt_str_field(t, "claimDeadline"),
        completion_window: opt_str_field(t, "completionWindow"),
    }
}

const TASK_FIELDS: &str = "taskId title status payout metadataHash bountyToken bountyPayout \
     assignee assigneeUsername requiresApplication absoluteDeadline claimDeadline \
     completionWindow project { projectId title }";

impl PoaClient {
    async fn subgraph_query(&self, query: &str, variables: Value) -> Result<Value, PoaError> {
        let response = self
            .http
            .post(&self.config.subgraph_url)
            .json(&json!({ "query": query, "variables": variables }))
            .send()
            .await
            .map_err(|e| PoaError::Subgraph(format!("request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(PoaError::Subgraph(format!(
                "HTTP {} from subgraph",
                response.status()
            )));
        }

        let body: Value = response
            .json()
            .await
            .map_err(|e| PoaError::Subgraph(format!("invalid JSON: {}", e)))?;

        if let Some(errors) = body.get("errors") {
            return Err(PoaError::Subgraph(format!("GraphQL errors: {}", errors)));
        }

        body.get("data")
            .cloned()
            .ok_or_else(|| PoaError::Subgraph("missing data field".into()))
    }

    fn task_manager_id(&self) -> String {
        format!("{:#x}", self.config.task_manager).to_lowercase()
    }

    /// List the org's (non-deleted) projects.
    pub async fn list_projects(&self) -> Result<Vec<ProjectInfo>, PoaError> {
        let query = "query($tm: Bytes!) { \
            taskManager(id: $tm) { \
                projects(first: 100, where: { deleted: false }) { \
                    projectId title cap tasks(first: 1000) { id } \
                } \
            } }";
        let data = self
            .subgraph_query(query, json!({ "tm": self.task_manager_id() }))
            .await?;

        let projects = data
            .pointer("/taskManager/projects")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        Ok(projects
            .iter()
            .map(|p| ProjectInfo {
                project_id: str_field(p, "projectId"),
                title: str_field(p, "title"),
                cap: str_field(p, "cap"),
                task_count: p
                    .get("tasks")
                    .and_then(Value::as_array)
                    .map(Vec::len)
                    .unwrap_or(0),
            })
            .collect())
    }

    /// List tasks for the org, optionally filtered by status and/or project id.
    pub async fn list_tasks(
        &self,
        status: Option<&str>,
        project_id: Option<&str>,
    ) -> Result<Vec<TaskInfo>, PoaError> {
        let mut where_clause = json!({ "taskManager": self.task_manager_id() });
        if let Some(s) = status {
            where_clause["status"] = json!(subgraph_status(s)?);
        }

        let query = format!(
            "query($where: Task_filter!) {{ \
                tasks(first: 200, orderBy: taskId, orderDirection: desc, where: $where) {{ \
                    {} \
                }} }}",
            TASK_FIELDS
        );
        let data = self
            .subgraph_query(&query, json!({ "where": where_clause }))
            .await?;

        let mut tasks: Vec<TaskInfo> = data
            .get("tasks")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(task_from_json)
            .collect();

        // Project filtering happens client-side to avoid depending on the
        // subgraph's composite Project id format.
        if let Some(pid) = project_id {
            let pid = pid.to_ascii_lowercase();
            tasks.retain(|t| t.project_id.to_ascii_lowercase() == pid);
        }

        Ok(tasks)
    }

    /// Fetch a single task by its numeric id.
    pub async fn get_task(&self, task_id: u64) -> Result<Option<TaskInfo>, PoaError> {
        let query = format!(
            "query($where: Task_filter!) {{ tasks(first: 1, where: $where) {{ {} }} }}",
            TASK_FIELDS
        );
        let where_clause = json!({
            "taskManager": self.task_manager_id(),
            "taskId": task_id.to_string(),
        });
        let data = self
            .subgraph_query(&query, json!({ "where": where_clause }))
            .await?;

        Ok(data
            .get("tasks")
            .and_then(Value::as_array)
            .and_then(|a| a.first())
            .map(task_from_json))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::PoaClientConfig;
    use alloy::signers::local::PrivateKeySigner;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_client(subgraph_url: String) -> PoaClient {
        let signer = PrivateKeySigner::from_slice(&[0x42u8; 32]).unwrap();
        PoaClient::new(
            PoaClientConfig {
                rpc_url: "http://localhost:1".into(),
                subgraph_url,
                task_manager: "0x00000000000000000000000000000000000000aa"
                    .parse()
                    .unwrap(),
                network_name: "testnet".into(),
            },
            signer,
        )
        .unwrap()
    }

    #[test]
    fn test_status_mapping() {
        assert_eq!(subgraph_status("open").unwrap(), "Open");
        assert_eq!(subgraph_status("UNCLAIMED").unwrap(), "Open");
        assert_eq!(subgraph_status("claimed").unwrap(), "Assigned");
        assert_eq!(subgraph_status("Submitted").unwrap(), "Submitted");
        assert!(subgraph_status("bogus").is_err());
    }

    #[tokio::test]
    async fn test_list_tasks_parses_response() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "data": { "tasks": [{
                "taskId": "7",
                "title": "Fix the docs",
                "status": "Open",
                "payout": "5000000000000000000",
                "metadataHash": "0x1111111111111111111111111111111111111111111111111111111111111111",
                "bountyToken": "0x0000000000000000000000000000000000000000",
                "bountyPayout": "0",
                "assignee": null,
                "assigneeUsername": null,
                "requiresApplication": false,
                "absoluteDeadline": null,
                "claimDeadline": null,
                "completionWindow": null,
                "project": { "projectId": "0xabc0", "title": "Docs" }
            }]}
        });
        Mock::given(method("POST"))
            .and(path("/graphql"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let client = test_client(format!("{}/graphql", server.uri()));
        let tasks = client.list_tasks(Some("open"), None).await.unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].task_id, "7");
        assert_eq!(tasks[0].title, "Fix the docs");
        assert_eq!(tasks[0].project_title, "Docs");
    }

    #[tokio::test]
    async fn test_graphql_errors_surface() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errors": [{ "message": "boom" }]
            })))
            .mount(&server)
            .await;

        let client = test_client(server.uri());
        let result = client.list_projects().await;
        assert!(matches!(result, Err(PoaError::Subgraph(_))));
    }
}
