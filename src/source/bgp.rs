use std::net::IpAddr;
use std::str::FromStr;

use ipnet::IpNet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BgpPrefix {
    pub prefix: IpNet,
    pub asn: u32,
}

impl BgpPrefix {
    pub fn new(prefix: IpNet, asn: u32) -> Self {
        Self { prefix, asn }
    }

    pub fn contains(&self, ip: IpAddr) -> bool {
        self.prefix.contains(&ip)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BgpRoute {
    pub network: IpNet,
    pub asn: u32,
    pub hits: u64,
}

impl BgpRoute {
    pub fn new(network: IpNet, asn: u32, hits: u64) -> Self {
        Self { network, asn, hits }
    }
}

pub fn parse_bgp_prefix(line: &str) -> crate::error::Result<BgpPrefix> {
    let fields: Vec<_> = line.split('|').map(str::trim).collect();

    if fields.len() < 2 {
        return Err(crate::error::Error::InvalidTarget);
    }

    let prefix = IpNet::from_str(fields[0]).map_err(|_| crate::error::Error::InvalidTarget)?;

    let asn = fields[1]
        .strip_prefix("AS")
        .or_else(|| fields[1].strip_prefix("as"))
        .and_then(|value| value.parse::<u32>().ok())
        .ok_or(crate::error::Error::InvalidTarget)?;

    Ok(BgpPrefix::new(prefix, asn))
}

pub fn parse_ipv4_routes_jsonl(content: &str) -> crate::error::Result<Vec<BgpRoute>> {
    let mut routes = Vec::new();

    for line in content.lines() {
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        let value: serde_json::Value =
            serde_json::from_str(line).map_err(|_| crate::error::Error::InvalidTarget)?;

        let cidr = value
            .get("CIDR")
            .and_then(|value| value.as_str())
            .ok_or(crate::error::Error::InvalidTarget)?;

        let asn = value
            .get("ASN")
            .and_then(|value| value.as_u64())
            .and_then(|value| u32::try_from(value).ok())
            .ok_or(crate::error::Error::InvalidTarget)?;

        let hits = value
            .get("Hits")
            .and_then(|value| value.as_u64())
            .unwrap_or(0);

        let network = IpNet::from_str(cidr).map_err(|_| crate::error::Error::InvalidTarget)?;

        if !network.addr().is_ipv4() {
            return Err(crate::error::Error::InvalidTarget);
        }

        routes.push(BgpRoute::new(network, asn, hits));
    }

    Ok(routes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bgp_prefix() {
        let prefix = parse_bgp_prefix("1.0.1.0/24 | AS4134").unwrap();

        assert_eq!(prefix.prefix, IpNet::from_str("1.0.1.0/24").unwrap());
        assert_eq!(prefix.asn, 4134);
    }

    #[test]
    fn test_parse_lowercase_asn() {
        let prefix = parse_bgp_prefix("1.0.2.0/23 | as9808").unwrap();

        assert_eq!(prefix.asn, 9808);
    }

    #[test]
    fn test_invalid_prefix() {
        assert!(parse_bgp_prefix("invalid | AS4134").is_err());
    }

    #[test]
    fn test_invalid_asn() {
        assert!(parse_bgp_prefix("1.0.1.0/24 | invalid").is_err());
    }

    #[test]
    fn test_missing_fields() {
        assert!(parse_bgp_prefix("1.0.1.0/24").is_err());
    }

    #[test]
    fn test_prefix_contains_ip() {
        let prefix = BgpPrefix::new(IpNet::from_str("1.0.1.0/24").unwrap(), 4134);

        assert!(prefix.contains(IpAddr::from_str("1.0.1.1").unwrap()));
        assert!(!prefix.contains(IpAddr::from_str("1.0.2.1").unwrap()));
    }

    #[test]
    fn test_parse_ipv4_routes_jsonl() {
        let content = r#"{"CIDR":"117.128.0.0/10","ASN":9808,"Hits":5678}
{"CIDR":"223.122.0.0/15","ASN":4515,"Hits":999}"#;

        let routes = parse_ipv4_routes_jsonl(content).unwrap();

        assert_eq!(routes.len(), 2);
        assert_eq!(routes[0].asn, 9808);
        assert_eq!(routes[0].hits, 5678);
        assert_eq!(
            routes[0].network,
            IpNet::from_str("117.128.0.0/10").unwrap()
        );
    }

    #[test]
    fn test_parse_ipv4_routes_jsonl_skips_empty_lines() {
        let content = r#"
{"CIDR":"117.128.0.0/10","ASN":9808,"Hits":5678}

"#;

        let routes = parse_ipv4_routes_jsonl(content).unwrap();

        assert_eq!(routes.len(), 1);
    }

    #[test]
    fn test_parse_ipv4_routes_jsonl_rejects_invalid_json() {
        assert!(parse_ipv4_routes_jsonl(r#"{"CIDR":"invalid"}"#).is_err());
    }

    #[test]
    fn test_parse_ipv4_routes_jsonl_rejects_missing_asn() {
        assert!(parse_ipv4_routes_jsonl(r#"{"CIDR":"117.128.0.0/10"}"#).is_err());
    }
}
