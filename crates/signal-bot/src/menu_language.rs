//! Persisted menu-language field (legacy). UI menus are English-only for now.

use serde::{Deserialize, Serialize};

/// Kept for forward-compatible deserialize of older group-pref snapshots.
/// Command paths no longer read this for UI.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_code_parses_legacy_codes() {
        assert_eq!(MenuLanguage::from_code("en"), Some(MenuLanguage::En));
        assert_eq!(MenuLanguage::from_code("es"), Some(MenuLanguage::Es));
        assert_eq!(MenuLanguage::from_code("fr"), None);
        assert_eq!(MenuLanguage::default(), MenuLanguage::En);
    }
}
