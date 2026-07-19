//! Client for the pacto-bot-api daemon (https://github.com/covenant-gov/pacto-bot-api).
//!
//! Speaks JSON-RPC 2.0 over the daemon's Unix socket (newline-delimited
//! frames). The client registers as a handler with the `SendMessages`
//! capability and can then publish encrypted DMs into Pacto as the
//! configured bot identity.

mod agent;
mod client;
mod error;
mod types;

pub use agent::{InboundDm, PactoAgent};
pub use client::PactoClient;
pub use error::PactoError;
pub use types::{DaemonVersion, Registration};
