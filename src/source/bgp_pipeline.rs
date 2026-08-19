use std::collections::HashMap;

use super::bgp::BgpRoute;
use super::bgp_asn::BgpAsnRecord;
use super::bgp_decision::{classify, BgpDecision};
use super::bgp_evidence::BgpEvidence;

pub fn build_evidence(
    route: &BgpRoute,
    asn_index: &HashMap<u32, BgpAsnRecord>,
) -> Option<BgpEvidence> {
    let record = asn_index.get(&route.asn)?;

    Some(BgpEvidence {
        asn: route.asn,
        asn_name: Some(record.name.clone()),
        asn_class: Some(record.class.clone()),
        network_type: None,
        country: None,
        registry: None,
        source: Some("bgp".to_string()),
    })
}

pub fn classify_route(
    route: &BgpRoute,
    asn_index: &HashMap<u32, BgpAsnRecord>,
) -> BgpDecision {
    match build_evidence(route, asn_index) {
        Some(evidence) => classify(&evidence),
        None => BgpDecision::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn route() -> BgpRoute {
        BgpRoute::new(ipnet::IpNet::from_str("1.0.1.0/24").unwrap(), 4134, 100)
    }

    fn index() -> HashMap<u32, BgpAsnRecord> {
        let mut index = HashMap::new();

        index.insert(
            4134,
            BgpAsnRecord {
                asn: "AS4134".to_string(),
                name: "China Telecom".to_string(),
                class: "Eyeball".to_string(),
            },
        );

        index
    }

    #[test]
    fn test_build_evidence() {
        let evidence = build_evidence(&route(), &index()).unwrap();

        assert_eq!(evidence.asn, 4134);
        assert_eq!(evidence.asn_name.as_deref(), Some("China Telecom"));
        assert_eq!(evidence.asn_class.as_deref(), Some("Eyeball"));
        assert_eq!(evidence.source.as_deref(), Some("bgp"));
    }

    #[test]
    fn test_missing_asn_returns_none() {
        let index = HashMap::new();

        assert!(build_evidence(&route(), &index).is_none());
    }

    #[test]
    fn test_classify_route() {
        let decision = classify_route(&route(), &index());

        assert_ne!(decision, BgpDecision::Unknown);
    }

    #[test]
    fn test_missing_asn_is_unknown() {
        let index = HashMap::new();

        assert_eq!(classify_route(&route(), &index), BgpDecision::Unknown);
    }
}
