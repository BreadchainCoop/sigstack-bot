//! Plan SKU ↔ Stripe Price mapping.

use signal_bot::entitlements_store::PlanSku;

use crate::config::StripeConfig;

pub fn parse_plan_sku(s: &str) -> Option<PlanSku> {
    match s.trim() {
        "all-access-3" => Some(PlanSku::AllAccess3),
        "all-access-10" => Some(PlanSku::AllAccess10),
        _ => None,
    }
}

pub fn plan_sku_str(sku: PlanSku) -> &'static str {
    match sku {
        PlanSku::AllAccess3 => "all-access-3",
        PlanSku::AllAccess10 => "all-access-10",
        PlanSku::BundleAllAlpha => "bundle-all-alpha",
    }
}

pub fn price_id_for(sku: PlanSku, cfg: &StripeConfig) -> Result<&str, String> {
    let id = match sku {
        PlanSku::AllAccess3 => cfg.price_all_access_3.as_str(),
        PlanSku::AllAccess10 => cfg.price_all_access_10.as_str(),
        PlanSku::BundleAllAlpha => {
            return Err("alpha is not a Stripe checkout SKU".into());
        }
    };
    if id.is_empty() {
        return Err(format!(
            "Stripe Price ID not configured for {}",
            plan_sku_str(sku)
        ));
    }
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_known_skus() {
        assert_eq!(parse_plan_sku("all-access-3"), Some(PlanSku::AllAccess3));
        assert_eq!(parse_plan_sku("all-access-10"), Some(PlanSku::AllAccess10));
        assert!(parse_plan_sku("bundle-all-alpha").is_none());
        assert!(parse_plan_sku("nope").is_none());
    }
}
