//! Application configuration loaded from environment variables.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::time::Duration;

/// Application configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Signal configuration
    #[serde(default)]
    pub signal: SignalConfig,

    /// NEAR AI configuration
    pub near_ai: NearAiConfig,

    /// Conversation storage configuration
    #[serde(default)]
    pub conversation: ConversationConfig,

    /// Bot configuration
    #[serde(default)]
    pub bot: BotConfig,

    /// Dstack configuration
    #[serde(default)]
    pub dstack: DstackConfig,

    /// Tools configuration
    #[serde(default)]
    pub tools: ToolsConfig,

    /// Payment configuration
    #[serde(default)]
    pub payments: x402_payments::PaymentConfig,

    /// Whisper transcription configuration
    #[serde(default)]
    pub whisper: WhisperConfig,

    /// Group auto-translate (`!translate-on`) configuration
    #[serde(default)]
    pub translate_all: TranslateAllConfig,

    /// Encrypted persistence for per-group bot preferences
    #[serde(default)]
    pub group_preferences: GroupPreferencesConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SignalConfig {
    /// Signal CLI REST API endpoint
    #[serde(default = "default_signal_service")]
    pub service_url: String,

    /// Poll interval for messages
    #[serde(default = "default_poll_interval", with = "humantime_serde")]
    pub poll_interval: Duration,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NearAiConfig {
    /// NEAR AI API key
    pub api_key: String,

    /// API base URL
    #[serde(default = "default_near_ai_url")]
    pub base_url: String,

    /// Default model
    #[serde(default = "default_model")]
    pub model: String,

    /// Request timeout
    #[serde(default = "default_timeout", with = "humantime_serde")]
    pub timeout: Duration,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConversationConfig {
    /// Conversation TTL (how long before inactive conversations expire)
    #[serde(default = "default_ttl", with = "humantime_serde")]
    pub ttl: Duration,

    /// Max messages per conversation (older messages are trimmed)
    #[serde(default = "default_max_messages")]
    pub max_messages: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BotConfig {
    /// System prompt for AI
    #[serde(default = "default_system_prompt")]
    pub system_prompt: String,

    /// Signal username (e.g., "nearai.54")
    #[serde(default)]
    pub signal_username: Option<String>,

    /// GitHub repository URL
    #[serde(default)]
    pub github_repo: Option<String>,

    /// Log level
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DstackConfig {
    /// Dstack guest agent socket path
    #[serde(default = "default_dstack_socket")]
    pub socket_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolsConfig {
    /// Enable tool use system
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Maximum tool calls per message
    #[serde(default = "default_max_tool_calls")]
    pub max_tool_calls: usize,

    /// Web search configuration
    #[serde(default)]
    pub web_search: WebSearchConfig,

    /// Weather tool configuration
    #[serde(default)]
    pub weather: WeatherConfig,

    /// Calculator tool configuration
    #[serde(default)]
    pub calculator: CalculatorConfig,

    /// Poa (DAO protocol) tools configuration
    #[serde(default)]
    pub poa: PoaConfig,
}

/// Configuration for the Poa protocol tool suite.
///
/// Read tools (list/get) are offered to everyone when `enabled`. Write tools
/// (create/update/assign/complete/reject/cancel task) additionally require
/// `enable_writes` and are only offered to / executed for Signal senders listed
/// in `authorized_senders`.
#[derive(Debug, Clone, Deserialize)]
pub struct PoaConfig {
    /// Master switch for Poa tools (off by default — opt-in).
    #[serde(default)]
    pub enabled: bool,

    /// JSON-RPC endpoint for the chain the org lives on.
    pub rpc_url: Option<String>,

    /// Poa subgraph GraphQL endpoint for that chain.
    pub subgraph_url: Option<String>,

    /// TaskManager proxy address (0x…) for the org being managed.
    pub task_manager: Option<String>,

    /// Human-readable network name shown to users.
    #[serde(default = "default_poa_network")]
    pub network_name: String,

    /// Wallet private key (0x hex). If omitted, the wallet is derived from the
    /// TEE via dstack `derive_key(derive_key_path)` so no key ever leaves the
    /// enclave. Prefer the derived path in production.
    pub private_key: Option<String>,

    /// dstack key-derivation path used when `private_key` is not set.
    #[serde(default = "default_poa_derive_path")]
    pub derive_key_path: String,

    /// Whether to offer state-changing write tools at all (off by default).
    #[serde(default)]
    pub enable_writes: bool,

    /// Comma/space-separated Signal sender ids (phone numbers) allowed to use
    /// write tools. Empty means no one — writes stay effectively read-only.
    pub authorized_senders: Option<String>,

    /// HybridVoting proxy address (0x…) — required for governance tools.
    pub voting_contract: Option<String>,

    /// Seconds a staged value-moving action (e.g. complete_task) stays
    /// confirmable via `!poa-confirm`.
    #[serde(default = "default_poa_confirm_ttl")]
    pub confirm_ttl_secs: u64,

    /// Autonomous board steward: periodically post a digest of expired/at-risk
    /// claims to a Signal target. Off by default.
    #[serde(default)]
    pub steward_enabled: bool,

    /// Signal target (group id or number) for steward digests.
    pub steward_target: Option<String>,

    /// Bot account (registered number) to send steward digests from. If unset,
    /// the first registered account is used.
    pub steward_from: Option<String>,

    /// Steward scan interval in seconds (default 6h).
    #[serde(default = "default_poa_steward_interval")]
    pub steward_interval_secs: u64,

    /// A claim counts as "at risk" if it expires within this many seconds
    /// (default 24h).
    #[serde(default = "default_poa_steward_warn")]
    pub steward_warn_window_secs: u64,

    /// Suppress steward posts when there is nothing to report.
    #[serde(default = "default_true")]
    pub steward_quiet_when_empty: bool,
}

impl Default for PoaConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            rpc_url: None,
            subgraph_url: None,
            task_manager: None,
            network_name: default_poa_network(),
            private_key: None,
            derive_key_path: default_poa_derive_path(),
            enable_writes: false,
            authorized_senders: None,
            voting_contract: None,
            confirm_ttl_secs: default_poa_confirm_ttl(),
            steward_enabled: false,
            steward_target: None,
            steward_from: None,
            steward_interval_secs: default_poa_steward_interval(),
            steward_warn_window_secs: default_poa_steward_warn(),
            steward_quiet_when_empty: true,
        }
    }
}

impl PoaConfig {
    /// Parse `authorized_senders` into a set of trimmed sender ids.
    pub fn authorized_sender_list(&self) -> Vec<String> {
        self.authorized_senders
            .as_deref()
            .map(|s| {
                s.split([',', ' ', '\n', '\t'])
                    .map(str::trim)
                    .filter(|p| !p.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Whether a given Signal sender may invoke Poa write tools.
    pub fn is_authorized(&self, sender: &str) -> bool {
        self.enable_writes && self.authorized_sender_list().iter().any(|s| s == sender)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebSearchConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub api_key: Option<String>,
    #[serde(default = "default_search_results")]
    pub max_results: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WeatherConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CalculatorConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WhisperConfig {
    /// Master switch for voice transcription
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// whisper-server base URL (no trailing path)
    #[serde(default = "default_whisper_service")]
    pub service_url: String,

    /// Model name loaded in the sidecar (e.g. `small`)
    #[serde(default = "default_whisper_model")]
    pub model: String,

    /// Max time per transcribe request
    #[serde(default = "default_whisper_timeout", with = "humantime_serde")]
    pub timeout: Duration,

    /// Reject attachments larger than this (rough proxy for max voice length)
    #[serde(default = "default_whisper_max_attachment_bytes")]
    pub max_attachment_bytes: usize,

    /// Prefix line before transcript text in quote-replies
    #[serde(default = "default_whisper_reply_prefix")]
    pub reply_prefix: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TranslateAllConfig {
    /// Master switch for group auto-translate
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Max auto-translate replies per group per minute (NEAR AI protection)
    #[serde(default = "default_translate_all_max_per_minute")]
    pub max_messages_per_minute: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GroupPreferencesConfig {
    /// Persist group transcription + translate-all settings (TEE-encrypted)
    #[serde(default = "default_true")]
    pub persist: bool,

    /// Encrypted preferences file path (Docker volume in production)
    #[serde(default = "default_group_preferences_path")]
    pub storage_path: String,
}

// Default implementations
impl Default for SignalConfig {
    fn default() -> Self {
        Self {
            service_url: default_signal_service(),
            poll_interval: default_poll_interval(),
        }
    }
}

impl Default for ConversationConfig {
    fn default() -> Self {
        Self {
            ttl: default_ttl(),
            max_messages: default_max_messages(),
        }
    }
}

impl Default for BotConfig {
    fn default() -> Self {
        Self {
            system_prompt: default_system_prompt(),
            signal_username: None,
            github_repo: None,
            log_level: default_log_level(),
        }
    }
}

impl Default for DstackConfig {
    fn default() -> Self {
        Self {
            socket_path: default_dstack_socket(),
        }
    }
}

impl Default for ToolsConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            max_tool_calls: default_max_tool_calls(),
            web_search: WebSearchConfig::default(),
            weather: WeatherConfig::default(),
            calculator: CalculatorConfig::default(),
            poa: PoaConfig::default(),
        }
    }
}

impl Default for WebSearchConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            api_key: None,
            max_results: default_search_results(),
        }
    }
}

impl Default for WeatherConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
        }
    }
}

impl Default for CalculatorConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
        }
    }
}

impl Default for WhisperConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            service_url: default_whisper_service(),
            model: default_whisper_model(),
            timeout: default_whisper_timeout(),
            max_attachment_bytes: default_whisper_max_attachment_bytes(),
            reply_prefix: default_whisper_reply_prefix(),
        }
    }
}

impl Default for TranslateAllConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            max_messages_per_minute: default_translate_all_max_per_minute(),
        }
    }
}

impl Default for GroupPreferencesConfig {
    fn default() -> Self {
        Self {
            persist: default_true(),
            storage_path: default_group_preferences_path(),
        }
    }
}

// Default value functions
fn default_signal_service() -> String {
    "http://signal-api:8080".into()
}

fn default_poll_interval() -> Duration {
    Duration::from_millis(200)
}

fn default_near_ai_url() -> String {
    "https://cloud-api.near.ai/v1".into()
}

fn default_model() -> String {
    "deepseek-ai/DeepSeek-V3.1".into()
}

fn default_timeout() -> Duration {
    Duration::from_secs(10)
}

fn default_ttl() -> Duration {
    Duration::from_secs(24 * 60 * 60) // 24 hours
}

fn default_max_messages() -> usize {
    50
}

fn default_system_prompt() -> String {
    r#"You are an AI assistant accessible via Signal, running in a Trusted Execution Environment (TEE) for privacy protection.

## Privacy & Security
- Your conversations are protected by Intel TDX hardware encryption
- Neither the bot operator nor the AI provider can read your messages
- Users can verify this by sending "!verify" for cryptographic attestation

## Available Tools
You have access to these tools - use them when helpful:
- **web_search**: Search the web for current information, news, facts
- **get_weather**: Get current weather for any location
- **calculate**: Evaluate math expressions accurately

## Guidelines
- Be concise - this is mobile chat, not essays
- Use tools proactively for current information (don't guess dates, prices, weather)
- For calculations, use the calculate tool rather than mental math
- If a tool fails, explain what happened and try to help anyway
- Never fabricate search results or weather data"#.into()
}

/// Build system prompt with identity information.
/// This is called at runtime to inject signal_username and github_repo.
pub fn build_system_prompt_with_identity(
    base_prompt: &str,
    signal_username: Option<&str>,
    github_repo: Option<&str>,
) -> String {
    let now = chrono::Utc::now();
    let mut prompt = base_prompt.to_string();

    // Add identity section if either field is configured
    if signal_username.is_some() || github_repo.is_some() {
        prompt.push_str("\n\n## Identity");
        if let Some(username) = signal_username {
            prompt.push_str(&format!("\n- Signal username: @{}", username));
        }
        if let Some(repo) = github_repo {
            prompt.push_str(&format!("\n- Source code: {}", repo));
        }
    }

    // Add current timestamp
    prompt.push_str(&format!(
        "\n\nCurrent date and time: {} UTC",
        now.format("%A, %B %d, %Y at %H:%M")
    ));

    prompt
}

fn default_log_level() -> String {
    "info".into()
}

fn default_dstack_socket() -> String {
    "/var/run/dstack.sock".into()
}

fn default_true() -> bool {
    true
}

fn default_max_tool_calls() -> usize {
    5
}

fn default_search_results() -> usize {
    5
}

fn default_poa_network() -> String {
    "gnosis".into()
}

fn default_poa_derive_path() -> String {
    "poa-tools/task-manager-wallet".into()
}

fn default_poa_confirm_ttl() -> u64 {
    300
}

fn default_poa_steward_interval() -> u64 {
    6 * 60 * 60
}

fn default_poa_steward_warn() -> u64 {
    24 * 60 * 60
}

fn default_whisper_service() -> String {
    "http://whisper-api:9000".into()
}

fn default_whisper_model() -> String {
    "small".into()
}

fn default_whisper_timeout() -> Duration {
    Duration::from_secs(120)
}

fn default_whisper_max_attachment_bytes() -> usize {
    // ~5 min voice at typical Signal bitrates
    10 * 1024 * 1024
}

fn default_whisper_reply_prefix() -> String {
    "📝 Transcript:".into()
}

fn default_translate_all_max_per_minute() -> u32 {
    30
}

fn default_group_preferences_path() -> String {
    "/data/group_prefs.enc".into()
}

impl Config {
    /// Load configuration from environment variables.
    pub fn load() -> Result<Self> {
        // Load .env file if present
        dotenvy::dotenv().ok();

        let config = config::Config::builder()
            .add_source(
                config::Environment::default()
                    .separator("__")
                    // Note: try_parsing(true) would parse +16504928286 as a positive number
                    // stripping the + prefix. Keep strings as strings.
                    .try_parsing(false),
            )
            .build()
            .context("Failed to build configuration")?;

        config
            .try_deserialize()
            .context("Failed to deserialize configuration")
    }
}
