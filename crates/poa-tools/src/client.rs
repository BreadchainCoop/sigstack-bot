//! Poa client: wallet + RPC + subgraph endpoints shared by all Poa tools.

use alloy::network::EthereumWallet;
use alloy::primitives::{Address, B256};
use alloy::providers::{Provider, ProviderBuilder};
use alloy::signers::local::PrivateKeySigner;
use thiserror::Error;
use tracing::info;

/// Errors from Poa client operations.
#[derive(Error, Debug)]
pub enum PoaError {
    /// Configuration problem (bad address, bad URL, missing key).
    #[error("Poa configuration error: {0}")]
    Config(String),

    /// Subgraph query failed.
    #[error("Poa subgraph error: {0}")]
    Subgraph(String),

    /// On-chain call failed.
    #[error("Poa chain error: {0}")]
    Chain(String),

    /// Invalid tool arguments.
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),
}

/// Static configuration for a Poa deployment (one org's TaskManager).
#[derive(Debug, Clone)]
pub struct PoaClientConfig {
    /// JSON-RPC endpoint of the chain the org lives on (e.g. Gnosis).
    pub rpc_url: String,
    /// Poa subgraph GraphQL endpoint for that chain.
    pub subgraph_url: String,
    /// Address of the org's TaskManager proxy.
    pub task_manager: Address,
    /// Optional address of the org's HybridVoting proxy (governance tools).
    pub voting_contract: Option<Address>,
    /// Human-readable network name shown to users (e.g. "gnosis").
    pub network_name: String,
}

impl PoaClientConfig {
    /// Build config from string fields, parsing the TaskManager address.
    pub fn parse(
        rpc_url: impl Into<String>,
        subgraph_url: impl Into<String>,
        task_manager: &str,
        network_name: impl Into<String>,
    ) -> Result<Self, PoaError> {
        let task_manager: Address = task_manager
            .trim()
            .parse()
            .map_err(|e| PoaError::Config(format!("invalid TaskManager address: {}", e)))?;
        Ok(Self {
            rpc_url: rpc_url.into(),
            subgraph_url: subgraph_url.into(),
            task_manager,
            voting_contract: None,
            network_name: network_name.into(),
        })
    }

    /// Set the HybridVoting contract address (parsed from a string).
    pub fn with_voting_contract(mut self, addr: Option<&str>) -> Result<Self, PoaError> {
        self.voting_contract = match addr.map(str::trim).filter(|s| !s.is_empty()) {
            Some(a) => Some(
                a.parse()
                    .map_err(|e| PoaError::Config(format!("invalid voting address: {}", e)))?,
            ),
            None => None,
        };
        Ok(self)
    }
}

/// Shared client used by every Poa tool.
pub struct PoaClient {
    pub(crate) config: PoaClientConfig,
    pub(crate) signer: PrivateKeySigner,
    pub(crate) http: reqwest::Client,
    pub(crate) rpc_url: reqwest::Url,
}

impl PoaClient {
    /// Create a client from config and a hex private key (`0x`-prefixed or not).
    pub fn from_hex_key(config: PoaClientConfig, hex_key: &str) -> Result<Self, PoaError> {
        let signer: PrivateKeySigner = hex_key
            .trim()
            .parse()
            .map_err(|e| PoaError::Config(format!("invalid private key: {}", e)))?;
        Self::new(config, signer)
    }

    /// Create a client from config and a raw 32-byte key, e.g. one derived from
    /// the TEE via dstack. Only the first 32 bytes are used.
    pub fn from_key_bytes(config: PoaClientConfig, key_bytes: &[u8]) -> Result<Self, PoaError> {
        if key_bytes.len() < 32 {
            return Err(PoaError::Config(format!(
                "derived key too short: {} bytes",
                key_bytes.len()
            )));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&key_bytes[..32]);
        let signer = PrivateKeySigner::from_bytes(&B256::from(arr))
            .map_err(|e| PoaError::Config(format!("invalid derived key: {}", e)))?;
        Self::new(config, signer)
    }

    /// Create a client from config and a wallet signer.
    pub fn new(config: PoaClientConfig, signer: PrivateKeySigner) -> Result<Self, PoaError> {
        let rpc_url: reqwest::Url = config
            .rpc_url
            .parse()
            .map_err(|e| PoaError::Config(format!("invalid RPC URL: {}", e)))?;

        info!(
            wallet = %signer.address(),
            task_manager = %config.task_manager,
            network = %config.network_name,
            "Poa client initialized"
        );

        Ok(Self {
            config,
            signer,
            http: reqwest::Client::new(),
            rpc_url,
        })
    }

    /// The configured HybridVoting address, or a config error if governance
    /// tools are used without one set.
    pub(crate) fn require_voting(&self) -> Result<Address, PoaError> {
        self.config.voting_contract.ok_or_else(|| {
            PoaError::Config(
                "governance tools need TOOLS__POA__VOTING_CONTRACT to be set".to_string(),
            )
        })
    }

    /// The bot's wallet address (must be granted project-manager rights on-chain).
    pub fn address(&self) -> Address {
        self.signer.address()
    }

    /// Build a provider with the wallet attached for sending transactions.
    pub(crate) fn provider(&self) -> impl Provider + Clone {
        ProviderBuilder::new()
            .with_gas_estimation()
            .wallet(EthereumWallet::from(self.signer.clone()))
            .connect_http(self.rpc_url.clone())
    }

    /// Native token balance of the bot wallet, in wei.
    pub async fn native_balance(&self) -> Result<alloy::primitives::U256, PoaError> {
        self.provider()
            .get_balance(self.address())
            .await
            .map_err(|e| PoaError::Chain(format!("balance query failed: {}", e)))
    }
}
