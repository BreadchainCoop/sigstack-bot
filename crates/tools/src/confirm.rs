//! Two-step confirmation for value-moving tool calls.
//!
//! A tool that [`crate::Tool::requires_confirmation`] does not act on its first
//! invocation. Instead it *stages* the intended call here and returns a
//! human-readable summary plus a short code. The bot surfaces that to the user,
//! who must reply with a deterministic confirm command (`!poa-confirm <code>`).
//! The confirm handler [`ConfirmationStore::take`]s the staged call and
//! re-dispatches it through the executor with `confirmed = true`.
//!
//! The code is a per-sender nonce, not a secret: safety comes from requiring a
//! deliberate second message from the *same* sender, which the LLM cannot forge.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// A staged, not-yet-executed tool call awaiting confirmation.
#[derive(Debug, Clone)]
pub struct PendingAction {
    /// Short confirmation code the user must echo.
    pub code: String,
    /// Tool to re-dispatch on confirmation.
    pub tool_name: String,
    /// Raw JSON arguments for that tool.
    pub arguments: String,
    /// Human-readable summary shown when staged.
    pub summary: String,
    /// When this staged action stops being valid.
    expires_at: Instant,
}

/// Per-sender store of pending confirmations.
pub struct ConfirmationStore {
    pending: Mutex<HashMap<String, PendingAction>>,
    ttl: Duration,
    counter: AtomicU64,
}

impl ConfirmationStore {
    /// Create a store where staged actions expire after `ttl`.
    pub fn new(ttl: Duration) -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
            ttl,
            counter: AtomicU64::new(1),
        }
    }

    /// Stage a call for `sender`, returning the confirmation code. Any prior
    /// pending action for that sender is replaced.
    pub fn stage(&self, sender: &str, tool_name: &str, arguments: &str, summary: &str) -> String {
        let n = self.counter.fetch_add(1, Ordering::Relaxed);
        let code = format!("{:04}", n % 10000);
        let action = PendingAction {
            code: code.clone(),
            tool_name: tool_name.to_string(),
            arguments: arguments.to_string(),
            summary: summary.to_string(),
            expires_at: Instant::now() + self.ttl,
        };
        self.pending
            .lock()
            .unwrap()
            .insert(sender.to_string(), action);
        code
    }

    /// Consume the pending action for `sender` if `code` matches and it has not
    /// expired. Returns `None` otherwise (and clears an expired entry).
    pub fn take(&self, sender: &str, code: &str) -> Option<PendingAction> {
        let mut map = self.pending.lock().unwrap();
        match map.get(sender) {
            Some(a) if a.code == code && a.expires_at > Instant::now() => map.remove(sender),
            Some(a) if a.expires_at <= Instant::now() => {
                map.remove(sender);
                None
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stage_and_take() {
        let store = ConfirmationStore::new(Duration::from_secs(300));
        let code = store.stage("+1", "poa_complete_task", "{\"task_id\":3}", "mint 5 PT");
        // Wrong code / wrong sender rejected.
        assert!(store.take("+1", "9999").is_none());
        assert!(store.take("+2", &code).is_none());
        // Correct code consumes exactly once.
        let action = store.take("+1", &code).expect("should confirm");
        assert_eq!(action.tool_name, "poa_complete_task");
        assert!(store.take("+1", &code).is_none());
    }

    #[test]
    fn test_expiry() {
        let store = ConfirmationStore::new(Duration::from_millis(0));
        let code = store.stage("+1", "t", "{}", "s");
        assert!(store.take("+1", &code).is_none());
    }

    #[test]
    fn test_restage_replaces() {
        let store = ConfirmationStore::new(Duration::from_secs(300));
        let c1 = store.stage("+1", "t", "{}", "s1");
        let c2 = store.stage("+1", "t", "{}", "s2");
        assert_ne!(c1, c2);
        assert!(store.take("+1", &c1).is_none());
        assert!(store.take("+1", &c2).is_some());
    }
}
