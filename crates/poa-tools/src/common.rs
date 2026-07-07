//! Shared argument-parsing helpers for Poa tools.

use crate::client::PoaError;
use alloy::primitives::{Address, B256, U256};
use sha2::{Digest, Sha256};
use tools::ToolError;

/// Map a `PoaError` to a `ToolError`, preserving invalid-argument classification.
pub(crate) fn map_err(e: PoaError) -> ToolError {
    match e {
        PoaError::InvalidArguments(m) => ToolError::InvalidArguments(m),
        other => ToolError::ExternalService(other.to_string()),
    }
}

/// Parse a 0x-prefixed bytes32 (project id, metadata hash, etc).
pub(crate) fn parse_b32(s: &str, field: &str) -> Result<B256, ToolError> {
    s.parse::<B256>().map_err(|_| {
        ToolError::InvalidArguments(format!("invalid {} (expected 0x + 64 hex): {}", field, s))
    })
}

/// Parse a metadata/reason hash: accept a `0x` bytes32 directly, or hash an
/// arbitrary string with sha256 so the model can pass plain text with no CID.
pub(crate) fn parse_metadata_hash(s: &str) -> B256 {
    if let Ok(h) = s.parse::<B256>() {
        return h;
    }
    B256::from_slice(&Sha256::digest(s.as_bytes()))
}

/// Parse a 0x wallet address.
pub(crate) fn parse_address(s: &str, field: &str) -> Result<Address, ToolError> {
    s.parse::<Address>()
        .map_err(|_| ToolError::InvalidArguments(format!("invalid {} address: {}", field, s)))
}

/// Widen a numeric task id.
pub(crate) fn parse_task_id(id: u64) -> U256 {
    U256::from(id)
}
