//! Pacto client error types.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum PactoError {
    #[error("Pacto daemon socket not found at {0}")]
    SocketNotFound(String),

    #[error("Pacto daemon I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Pacto daemon request timed out after {0:?}")]
    Timeout(std::time::Duration),

    #[error("Pacto daemon returned error {code}: {message}")]
    Rpc { code: i64, message: String },

    #[error("Pacto protocol error: {0}")]
    Protocol(String),
}
