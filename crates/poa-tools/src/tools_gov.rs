//! Governance Poa tools — read proposals, create non-executable polls, vote.
//!
//! Proposal *creation* here is deliberately limited to **non-executable polls**
//! (every option maps to an empty on-chain batch). The bot distils a question
//! from chat and surfaces it for the members to vote on; it never authors
//! arbitrary executable calls. Voting is a plain weight distribution. Both are
//! privileged (bot wallet must hold a creator hat / voting-class hat).

use crate::client::PoaClient;
use crate::common::{map_err, parse_metadata_hash};
use async_trait::async_trait;
use serde::Deserialize;
use std::sync::Arc;
use tools::{FunctionDefinition, Tool, ToolDefinition, ToolError};

const WRITE_TIMEOUT: u64 = 90;

/// List governance proposals (subgraph read, no authorization).
pub struct PoaListProposalsTool {
    client: Arc<PoaClient>,
}

impl PoaListProposalsTool {
    pub fn new(client: Arc<PoaClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for PoaListProposalsTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            tool_type: "function".into(),
            function: FunctionDefinition {
                name: "poa_list_proposals".into(),
                description: "List recent governance proposals for the org's HybridVoting \
                              contract (id, title, status, options, voting window)."
                    .into(),
                parameters: serde_json::json!({
                    "type": "object", "properties": {}, "required": []
                }),
            },
        }
    }

    fn name(&self) -> &str {
        "poa_list_proposals"
    }

    async fn execute(&self, _arguments: &str) -> Result<String, ToolError> {
        let proposals = self.client.list_proposals().await.map_err(map_err)?;
        if proposals.is_empty() {
            return Ok("No proposals found.".into());
        }
        let mut out = format!("Proposals ({}):\n", proposals.len());
        for p in proposals.iter().take(30) {
            out.push_str(&format!(
                "#{} [{}] {} — {} option(s), ends @ {}{}\n",
                p.proposal_id,
                p.status,
                p.title,
                p.num_options,
                p.end_timestamp,
                p.winning_option
                    .as_ref()
                    .map(|w| format!(" (winner: option {})", w))
                    .unwrap_or_default(),
            ));
        }
        Ok(out)
    }
}

#[derive(Deserialize)]
struct CreatePollArgs {
    title: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default = "default_options")]
    num_options: u8,
    #[serde(default = "default_duration")]
    minutes_duration: u32,
}

fn default_options() -> u8 {
    2
}

fn default_duration() -> u32 {
    // 3 days.
    3 * 24 * 60
}

/// Create a non-executable governance poll.
pub struct PoaCreatePollTool {
    client: Arc<PoaClient>,
}

impl PoaCreatePollTool {
    pub fn new(client: Arc<PoaClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for PoaCreatePollTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            tool_type: "function".into(),
            function: FunctionDefinition {
                name: "poa_create_poll".into(),
                description: "Create a NON-executable governance poll on the org's HybridVoting \
                              contract for members to vote on. Options are unnamed indices \
                              0..num_options (describe them in the description). Requires the bot \
                              wallet to hold a proposal-creator hat. Does not execute any on-chain \
                              action — it only surfaces a decision for a vote."
                    .into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "title": { "type": "string", "description": "Poll question / title" },
                        "description": { "type": "string", "description": "Details + what each option index means (0x bytes32 CID or free text)" },
                        "num_options": { "type": "integer", "description": "Number of options (default 2, e.g. yes/no)" },
                        "minutes_duration": { "type": "integer", "description": "Voting window in minutes (default 4320 = 3 days)" }
                    },
                    "required": ["title"]
                }),
            },
        }
    }

    fn name(&self) -> &str {
        "poa_create_poll"
    }

    fn requires_authorization(&self) -> bool {
        true
    }

    fn timeout_override(&self) -> Option<u64> {
        Some(WRITE_TIMEOUT)
    }

    async fn execute(&self, arguments: &str) -> Result<String, ToolError> {
        let args: CreatePollArgs = serde_json::from_str(arguments)
            .map_err(|e| ToolError::InvalidArguments(e.to_string()))?;
        if args.num_options < 2 {
            return Err(ToolError::InvalidArguments(
                "a poll needs at least 2 options".into(),
            ));
        }
        let voting = self.client.require_voting().map_err(map_err)?;
        let description_hash = args
            .description
            .as_deref()
            .map(parse_metadata_hash)
            .unwrap_or(alloy::primitives::B256::ZERO);

        let outcome = self
            .client
            .create_poll(
                voting,
                args.title.clone(),
                description_hash,
                args.minutes_duration,
                args.num_options,
            )
            .await
            .map_err(map_err)?;
        Ok(format!(
            "Created poll \"{}\" ({} options, open {} min). Transaction {} confirmed.",
            args.title, args.num_options, args.minutes_duration, outcome.hash
        ))
    }
}

#[derive(Deserialize)]
struct VoteArgs {
    proposal_id: u64,
    /// Option indices to put weight on.
    options: Vec<u8>,
    /// Weights per option (must sum to 100). Defaults to equal split.
    #[serde(default)]
    weights: Option<Vec<u8>>,
}

/// Cast a vote on a proposal.
pub struct PoaVoteTool {
    client: Arc<PoaClient>,
}

impl PoaVoteTool {
    pub fn new(client: Arc<PoaClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for PoaVoteTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            tool_type: "function".into(),
            function: FunctionDefinition {
                name: "poa_vote".into(),
                description: "Cast the bot wallet's vote on a proposal. Provide the option \
                              indices and optional per-option weights (must sum to 100; defaults \
                              to an equal split). Requires the bot to hold a voting-class hat."
                    .into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "proposal_id": { "type": "integer", "description": "Numeric proposal id" },
                        "options": { "type": "array", "items": { "type": "integer" }, "description": "Option indices to support" },
                        "weights": { "type": "array", "items": { "type": "integer" }, "description": "Weights per option summing to 100 (optional)" }
                    },
                    "required": ["proposal_id", "options"]
                }),
            },
        }
    }

    fn name(&self) -> &str {
        "poa_vote"
    }

    fn requires_authorization(&self) -> bool {
        true
    }

    fn timeout_override(&self) -> Option<u64> {
        Some(WRITE_TIMEOUT)
    }

    async fn execute(&self, arguments: &str) -> Result<String, ToolError> {
        let args: VoteArgs = serde_json::from_str(arguments)
            .map_err(|e| ToolError::InvalidArguments(e.to_string()))?;
        if args.options.is_empty() {
            return Err(ToolError::InvalidArguments("no options given".into()));
        }

        // Resolve weights: explicit, else an even split summing to 100.
        let weights = match args.weights {
            Some(w) => {
                if w.len() != args.options.len() {
                    return Err(ToolError::InvalidArguments(
                        "weights length must match options length".into(),
                    ));
                }
                if w.iter().map(|x| *x as u32).sum::<u32>() != 100 {
                    return Err(ToolError::InvalidArguments(
                        "weights must sum to 100".into(),
                    ));
                }
                w
            }
            None => even_split(args.options.len())?,
        };

        let voting = self.client.require_voting().map_err(map_err)?;
        let outcome = self
            .client
            .vote(
                voting,
                alloy::primitives::U256::from(args.proposal_id),
                args.options.clone(),
                weights,
            )
            .await
            .map_err(map_err)?;
        Ok(format!(
            "Voted on proposal #{} (options {:?}). Transaction {} confirmed.",
            args.proposal_id, args.options, outcome.hash
        ))
    }
}

/// Split 100 as evenly as possible across `n` options (remainder on the first).
fn even_split(n: usize) -> Result<Vec<u8>, ToolError> {
    if n == 0 || n > 100 {
        return Err(ToolError::InvalidArguments("invalid option count".into()));
    }
    let base = (100 / n) as u8;
    let mut weights = vec![base; n];
    let remainder = (100 - base as usize * n) as u8;
    weights[0] += remainder;
    Ok(weights)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_even_split() {
        assert_eq!(even_split(2).unwrap(), vec![50, 50]);
        assert_eq!(even_split(3).unwrap(), vec![34, 33, 33]);
        assert_eq!(even_split(4).unwrap(), vec![25, 25, 25, 25]);
        assert_eq!(even_split(1).unwrap(), vec![100]);
        let s = even_split(3).unwrap();
        assert_eq!(s.iter().map(|x| *x as u32).sum::<u32>(), 100);
    }
}
