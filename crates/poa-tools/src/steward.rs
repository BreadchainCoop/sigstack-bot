//! Autonomous board steward.
//!
//! A read-only watcher that scans the org's tasks and surfaces ones that need
//! attention: claims whose deadline has passed (open to takeover under the v6
//! takeover rules) and claims about to expire. The scan is a pure function over
//! subgraph data + a supplied "now", so it is deterministic and testable; the
//! periodic loop that posts the digest to a Signal group lives in the bot (which
//! owns the Signal client).

use crate::subgraph::TaskInfo;

/// One task flagged by the steward.
#[derive(Debug, Clone, PartialEq)]
pub struct FlaggedTask {
    pub task_id: String,
    pub title: String,
    pub assignee: Option<String>,
    pub deadline: u64,
}

/// Result of a steward scan.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StewardReport {
    /// Claims whose deadline has passed (open to takeover).
    pub expired_claims: Vec<FlaggedTask>,
    /// Claims whose deadline is within the warn window.
    pub at_risk: Vec<FlaggedTask>,
    /// Count of currently open (unclaimed) tasks.
    pub open_count: usize,
}

impl StewardReport {
    /// True when nothing needs surfacing.
    pub fn is_empty(&self) -> bool {
        self.expired_claims.is_empty() && self.at_risk.is_empty()
    }

    /// Render a human digest for Signal.
    pub fn render(&self) -> String {
        let mut out = String::from("🩺 Board steward digest\n");
        if !self.expired_claims.is_empty() {
            out.push_str(&format!(
                "\n⏰ {} expired claim(s) — open to takeover:\n",
                self.expired_claims.len()
            ));
            for t in &self.expired_claims {
                out.push_str(&fmt_flagged(t));
            }
        }
        if !self.at_risk.is_empty() {
            out.push_str(&format!(
                "\n⚠️ {} claim(s) expiring soon:\n",
                self.at_risk.len()
            ));
            for t in &self.at_risk {
                out.push_str(&fmt_flagged(t));
            }
        }
        out.push_str(&format!(
            "\n📋 {} open task(s) awaiting a claimer.",
            self.open_count
        ));
        out
    }
}

fn fmt_flagged(t: &FlaggedTask) -> String {
    format!(
        "  • #{} {}{}\n",
        t.task_id,
        t.title,
        t.assignee
            .as_ref()
            .map(|a| format!(" (claimer: {})", a))
            .unwrap_or_default()
    )
}

fn parse_unix(s: &Option<String>) -> Option<u64> {
    s.as_ref().and_then(|v| v.parse::<u64>().ok())
}

/// Classify tasks into a report given the current unix time and warn window.
pub fn scan(tasks: &[TaskInfo], now_unix: u64, warn_window_secs: u64) -> StewardReport {
    let mut report = StewardReport::default();

    for t in tasks {
        match t.status.as_str() {
            "Open" => report.open_count += 1,
            // Only ASSIGNED claims are takeover-able; SUBMITTED work must be reviewed.
            "Assigned" => {
                // Effective deadline is the nearest of claim/absolute deadlines.
                let deadline = [
                    parse_unix(&t.claim_deadline),
                    parse_unix(&t.absolute_deadline),
                ]
                .into_iter()
                .flatten()
                .min();
                if let Some(dl) = deadline {
                    let flagged = FlaggedTask {
                        task_id: t.task_id.clone(),
                        title: t.title.clone(),
                        assignee: t.assignee_username.clone().or_else(|| t.assignee.clone()),
                        deadline: dl,
                    };
                    if dl <= now_unix {
                        report.expired_claims.push(flagged);
                    } else if dl <= now_unix + warn_window_secs {
                        report.at_risk.push(flagged);
                    }
                }
            }
            _ => {}
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(id: &str, status: &str, claim_deadline: Option<&str>) -> TaskInfo {
        TaskInfo {
            task_id: id.into(),
            title: format!("task {}", id),
            status: status.into(),
            payout: "0".into(),
            metadata_hash: String::new(),
            bounty_token: String::new(),
            bounty_payout: "0".into(),
            assignee: Some("0xabc".into()),
            assignee_username: None,
            project_id: String::new(),
            project_title: String::new(),
            requires_application: false,
            absolute_deadline: None,
            claim_deadline: claim_deadline.map(str::to_string),
            completion_window: None,
        }
    }

    #[test]
    fn test_scan_classifies() {
        let now = 1_000u64;
        let tasks = vec![
            task("1", "Open", None),
            task("2", "Assigned", Some("900")),  // expired
            task("3", "Assigned", Some("1050")), // at risk (window 100)
            task("4", "Assigned", Some("5000")), // healthy
            task("5", "Submitted", Some("900")), // never flagged (must be reviewed)
            task("6", "Completed", None),
        ];
        let report = scan(&tasks, now, 100);
        assert_eq!(report.open_count, 1);
        assert_eq!(report.expired_claims.len(), 1);
        assert_eq!(report.expired_claims[0].task_id, "2");
        assert_eq!(report.at_risk.len(), 1);
        assert_eq!(report.at_risk[0].task_id, "3");
        assert!(!report.is_empty());
    }

    #[test]
    fn test_empty_report() {
        let report = scan(&[task("1", "Completed", None)], 1000, 100);
        assert!(report.is_empty());
        assert!(report.render().contains("0 open task"));
    }
}
