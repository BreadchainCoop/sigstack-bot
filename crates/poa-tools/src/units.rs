//! Decimal ↔ wei helpers for 18-decimal participation tokens.

use crate::client::PoaError;
use alloy::primitives::U256;

const DECIMALS: usize = 18;

/// Parse a human decimal amount (e.g. "12.5") into an 18-decimal U256.
pub fn parse_pt(amount: &str) -> Result<U256, PoaError> {
    let amount = amount.trim();
    if amount.is_empty() || amount.starts_with('-') {
        return Err(PoaError::InvalidArguments(format!(
            "invalid amount '{}'",
            amount
        )));
    }

    let (whole, frac) = match amount.split_once('.') {
        Some((w, f)) => (w, f),
        None => (amount, ""),
    };

    if frac.len() > DECIMALS {
        return Err(PoaError::InvalidArguments(format!(
            "amount '{}' has more than {} decimal places",
            amount, DECIMALS
        )));
    }

    let whole = if whole.is_empty() { "0" } else { whole };
    let padded_frac = format!("{:0<width$}", frac, width = DECIMALS);
    let combined = format!("{}{}", whole, padded_frac);

    U256::from_str_radix(&combined, 10)
        .map_err(|_| PoaError::InvalidArguments(format!("invalid amount '{}'", amount)))
}

/// Format an 18-decimal wei string as a human decimal amount.
pub fn format_pt(wei: &str) -> String {
    let wei = wei.trim_start_matches('0');
    if wei.is_empty() {
        return "0".into();
    }
    if wei.len() <= DECIMALS {
        let frac = format!("{:0>width$}", wei, width = DECIMALS);
        let frac = frac.trim_end_matches('0');
        if frac.is_empty() {
            "0".into()
        } else {
            format!("0.{}", frac)
        }
    } else {
        let (whole, frac) = wei.split_at(wei.len() - DECIMALS);
        let frac = frac.trim_end_matches('0');
        if frac.is_empty() {
            whole.into()
        } else {
            format!("{}.{}", whole, frac)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pt() {
        assert_eq!(parse_pt("1").unwrap(), U256::from(10).pow(U256::from(18)));
        assert_eq!(
            parse_pt("2.5").unwrap(),
            U256::from(25) * U256::from(10).pow(U256::from(17))
        );
        assert_eq!(parse_pt("0.000000000000000001").unwrap(), U256::from(1));
        assert!(parse_pt("-1").is_err());
        assert!(parse_pt("abc").is_err());
        assert!(parse_pt("1.0000000000000000001").is_err());
    }

    #[test]
    fn test_format_pt() {
        assert_eq!(format_pt("1000000000000000000"), "1");
        assert_eq!(format_pt("2500000000000000000"), "2.5");
        assert_eq!(format_pt("1"), "0.000000000000000001");
        assert_eq!(format_pt("0"), "0");
        assert_eq!(format_pt(""), "0");
    }

    #[test]
    fn test_roundtrip() {
        for s in ["1", "2.5", "1000", "0.25"] {
            let wei = parse_pt(s).unwrap().to_string();
            assert_eq!(format_pt(&wei), s);
        }
    }
}
