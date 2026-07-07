//! Poa protocol tools for the Signal bot.
//!
//! Poa is a DAO coordination protocol (BeaconProxy orgs with TaskManager,
//! voting, participation tokens). These tools let the bot read an org's
//! projects/tasks from the Poa subgraph and — when the bot's wallet has been
//! granted project-manager rights on-chain — create and update tasks in the
//! org's `TaskManager` contract.
//!
//! Read tools are available to everyone the bot talks to. Write tools are
//! marked [`tools::Tool::requires_authorization`] and are only offered to /
//! executable by Signal senders on the configured allowlist. Value-moving tools
//! additionally set [`tools::Tool::requires_confirmation`].
//!
//! The suite spans the org lifecycle: reading projects/tasks/proposals, task
//! authoring (create/update/assign/complete/reject/cancel), the bot doing work
//! itself (claim/submit/apply — it can *earn* participation tokens), and
//! governance (create polls, vote).

mod client;
mod common;
mod contract;
mod subgraph;
mod tools_gov;
mod tools_participate;
mod tools_read;
mod tools_write;
mod units;

pub mod steward;

use std::sync::Arc;
use tools::ConfirmationStore;

pub use client::{PoaClient, PoaClientConfig, PoaError};
pub use contract::TxOutcome;
pub use steward::{FlaggedTask, StewardReport};
pub use subgraph::{ProjectInfo, ProposalInfo, TaskInfo};
pub use tools_gov::{PoaCreatePollTool, PoaListProposalsTool, PoaVoteTool};
pub use tools_participate::{PoaApplyForTaskTool, PoaClaimTaskTool, PoaSubmitTaskTool};
pub use tools_read::{PoaGetTaskTool, PoaListProjectsTool, PoaListTasksTool, PoaWalletInfoTool};
pub use tools_write::{
    PoaAssignTaskTool, PoaCancelTaskTool, PoaCompleteTaskTool, PoaCreateTaskTool,
    PoaRejectTaskTool, PoaUpdateTaskTool,
};

/// Build every Poa tool backed by the given client.
///
/// Returns read tools first, then write tools (the write tools all report
/// `requires_authorization() == true`; `poa_complete_task` also requires
/// confirmation, which is why the shared [`ConfirmationStore`] is threaded in).
pub fn all_tools(
    client: Arc<PoaClient>,
    confirm: Arc<ConfirmationStore>,
) -> Vec<Arc<dyn tools::Tool>> {
    vec![
        // Reads.
        Arc::new(PoaListProjectsTool::new(client.clone())),
        Arc::new(PoaListTasksTool::new(client.clone())),
        Arc::new(PoaGetTaskTool::new(client.clone())),
        Arc::new(PoaListProposalsTool::new(client.clone())),
        Arc::new(PoaWalletInfoTool::new(client.clone())),
        // Task authoring (write).
        Arc::new(PoaCreateTaskTool::new(client.clone())),
        Arc::new(PoaUpdateTaskTool::new(client.clone())),
        Arc::new(PoaAssignTaskTool::new(client.clone())),
        Arc::new(PoaCompleteTaskTool::new(client.clone(), confirm)),
        Arc::new(PoaRejectTaskTool::new(client.clone())),
        Arc::new(PoaCancelTaskTool::new(client.clone())),
        // Participation (write) — the bot doing work.
        Arc::new(PoaClaimTaskTool::new(client.clone())),
        Arc::new(PoaSubmitTaskTool::new(client.clone())),
        Arc::new(PoaApplyForTaskTool::new(client.clone())),
        // Governance (write).
        Arc::new(PoaCreatePollTool::new(client.clone())),
        Arc::new(PoaVoteTool::new(client)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_client() -> Arc<PoaClient> {
        let config = PoaClientConfig::parse(
            "http://localhost:1",
            "http://localhost:2/graphql",
            "0x00000000000000000000000000000000000000aa",
            "testnet",
        )
        .unwrap();
        Arc::new(PoaClient::from_key_bytes(config, &[0x42u8; 32]).unwrap())
    }

    fn dummy_tools() -> Vec<Arc<dyn tools::Tool>> {
        let confirm = Arc::new(ConfirmationStore::new(std::time::Duration::from_secs(300)));
        all_tools(dummy_client(), confirm)
    }

    #[test]
    fn test_read_write_tool_split() {
        let tools = dummy_tools();
        assert_eq!(tools.len(), 16);

        let read: Vec<&str> = tools
            .iter()
            .filter(|t| !t.requires_authorization())
            .map(|t| t.name())
            .collect();
        let write: Vec<&str> = tools
            .iter()
            .filter(|t| t.requires_authorization())
            .map(|t| t.name())
            .collect();

        // 5 read tools, 11 write tools.
        assert_eq!(read.len(), 5, "read tools: {:?}", read);
        assert_eq!(write.len(), 11, "write tools: {:?}", write);
        assert!(read.contains(&"poa_list_projects"));
        assert!(read.contains(&"poa_list_proposals"));
        assert!(read.contains(&"poa_wallet_info"));
        assert!(write.contains(&"poa_create_task"));
        assert!(write.contains(&"poa_claim_task"));
        assert!(write.contains(&"poa_create_poll"));
        assert!(write.contains(&"poa_vote"));

        // Every write tool must carry a longer-than-default timeout for mining.
        for t in tools.iter().filter(|t| t.requires_authorization()) {
            assert!(
                t.timeout_override().is_some(),
                "{} missing timeout override",
                t.name()
            );
        }
    }

    #[test]
    fn test_only_complete_requires_confirmation() {
        let tools = dummy_tools();
        let need_confirm: Vec<&str> = tools
            .iter()
            .filter(|t| t.requires_confirmation())
            .map(|t| t.name())
            .collect();
        assert_eq!(need_confirm, vec!["poa_complete_task"]);
    }
}
