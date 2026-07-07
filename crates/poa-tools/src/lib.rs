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
//! executable by Signal senders on the configured allowlist.

mod client;
mod contract;
mod subgraph;
mod tools_read;
mod tools_write;
mod units;

use std::sync::Arc;

pub use client::{PoaClient, PoaClientConfig, PoaError};
pub use contract::TxOutcome;
pub use tools_read::{PoaGetTaskTool, PoaListProjectsTool, PoaListTasksTool, PoaWalletInfoTool};
pub use tools_write::{
    PoaAssignTaskTool, PoaCancelTaskTool, PoaCompleteTaskTool, PoaCreateTaskTool,
    PoaRejectTaskTool, PoaUpdateTaskTool,
};

/// Build every Poa tool backed by the given client.
///
/// Returns read tools first, then write tools (the write tools all report
/// `requires_authorization() == true`).
pub fn all_tools(client: Arc<PoaClient>) -> Vec<Arc<dyn tools::Tool>> {
    vec![
        Arc::new(PoaListProjectsTool::new(client.clone())),
        Arc::new(PoaListTasksTool::new(client.clone())),
        Arc::new(PoaGetTaskTool::new(client.clone())),
        Arc::new(PoaWalletInfoTool::new(client.clone())),
        Arc::new(PoaCreateTaskTool::new(client.clone())),
        Arc::new(PoaUpdateTaskTool::new(client.clone())),
        Arc::new(PoaAssignTaskTool::new(client.clone())),
        Arc::new(PoaCompleteTaskTool::new(client.clone())),
        Arc::new(PoaRejectTaskTool::new(client.clone())),
        Arc::new(PoaCancelTaskTool::new(client)),
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

    #[test]
    fn test_read_write_tool_split() {
        let tools = all_tools(dummy_client());
        assert_eq!(tools.len(), 10);

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

        // 4 read tools, 6 write tools.
        assert_eq!(read.len(), 4, "read tools: {:?}", read);
        assert_eq!(write.len(), 6, "write tools: {:?}", write);
        assert!(read.contains(&"poa_list_projects"));
        assert!(read.contains(&"poa_wallet_info"));
        assert!(write.contains(&"poa_create_task"));
        assert!(write.contains(&"poa_complete_task"));

        // Every write tool must carry a longer-than-default timeout for mining.
        for t in tools.iter().filter(|t| t.requires_authorization()) {
            assert!(
                t.timeout_override().is_some(),
                "{} missing timeout override",
                t.name()
            );
        }
    }
}
