use super::bgp::{BgpPrefix, BgpRoute};
use super::bgp_asn::BgpAsnRecord;
use std::collections::HashMap;
use std::net::IpAddr;

pub fn lookup_asn<'a>(
    route: &BgpRoute,
    asn_index: &'a HashMap<u32, BgpAsnRecord>,
) -> Option<&'a BgpAsnRecord> {
    asn_index.get(&route.asn)
}

pub fn lookup_prefix(route: &BgpRoute) -> BgpPrefix {
    BgpPrefix::new(route.network, route.asn)
}

pub fn route_contains_ip(route: &BgpRoute, ip: IpAddr) -> bool {
    route.network.contains(&ip)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn route() -> BgpRoute {
        BgpRoute::new(ipnet::IpNet::from_str("1.0.1.0/24").unwrap(), 4134, 100)
    }

    fn record() -> BgpAsnRecord {
        BgpAsnRecord {
            asn: "AS4134".to_string(),
            name: "China Telecom".to_string(),
            class: "Eyeball".to_string(),
            country: "CN".to_string(),
            registry: "APNIC".to_string(),
        }
    }

    #[test]
    fn test_lookup_asn() {
        let mut index = HashMap::new();
        index.insert(4134, record());

        let result = lookup_asn(&route(), &index).unwrap();

        assert_eq!(result.asn, "AS4134");
        assert_eq!(result.name, "China Telecom");
    }

    #[test]
    fn test_lookup_missing_asn() {
        let index = HashMap::new();

        assert!(lookup_asn(&route(), &index).is_none());
    }

    #[test]
    fn test_lookup_prefix() {
        let prefix = lookup_prefix(&route());

        assert_eq!(prefix.asn, 4134);
        assert_eq!(prefix.prefix, ipnet::IpNet::from_str("1.0.1.0/24").unwrap());
    }

    #[test]
    fn test_route_contains_ip() {
        let route = route();

        assert!(route_contains_ip(
            &route,
            IpAddr::from_str("1.0.1.1").unwrap()
        ));

        assert!(!route_contains_ip(
            &route,
            IpAddr::from_str("1.0.2.1").unwrap()
        ));
    }
}
