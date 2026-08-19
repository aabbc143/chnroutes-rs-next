use std::collections::HashMap;

use super::bgp::BgpRoute;
use super::bgp_asn::BgpAsnRecord;
use super::bgp_evidence::BgpAsnEvidence;

pub fn build_evidence(
    route: &BgpRoute,
    asn_index: &HashMap<u32, BgpAsnRecord>,
) -> Option<BgpAsnEvidence> {
    let record = asn_index.get(&route.asn)?;

    Some(BgpAsnEvidence {
        asn: route.asn,
        name: record.name.clone(),
        network_class: record.class.clone(),
    })
}

pub fn classify_route(route: &BgpRoute, asn_index: &HashMap<u32, BgpAsnRecord>) -> bool {
    build_evidence(route, asn_index).is_some()
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
        assert_eq!(evidence.name, "China Telecom");
        assert_eq!(evidence.class, "Eyeball");
    }

    #[test]
    fn test_missing_asn() {
        let index = HashMap::new();

        assert!(build_evidence(&route(), &index).is_none());
    }

    #[test]
    fn test_classify_route() {
        assert!(classify_route(&route(), &index()));
    }
}
