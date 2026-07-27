//! Shared types for signal-bot product crates (handlers trait + errors).

pub mod error;
pub mod handler;

pub use error::{AppError, AppResult};
pub use handler::CommandHandler;
