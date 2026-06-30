//! UI menu language for `!help` and `!privacy` (per-group).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum MenuLanguage {
    #[default]
    En,
    Es,
}

impl MenuLanguage {
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "en" => Some(Self::En),
            "es" => Some(Self::Es),
            _ => None,
        }
    }
}
