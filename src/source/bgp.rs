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

pub fn parse_bgp_prefix(line: &str) -> crate::error::Result<BgpPrefix> {
    let fields: Vec<_> = line.split('|').map(str::trim).collect();

    if fields.len() < 2 {
        return Err(crate::error::Error::InvalidTarget);
    }

    let prefix = IpNet::from_str(fields[0])
        .map_err(|_| crate::error::Error::InvalidTarget)?;

    let asn = fields[1]
        .strip_prefix("AS")
        .or_else(|| fields[1].strip_prefix("as"))
        .and_then(|value| value.parse::<u32>().ok())
        .ok_or(crate::error::Error::InvalidTarget)?;

    Ok(BgpPrefix::new(prefix, asn))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bgp_prefix() {
        let prefix = parse_bgp_prefix("1.0.1.0/24 | AS4134").unwrap();

        assert_eq!(
            prefix.prefix,
            IpNet::from_str("1.0.1.0/24").unwrap()
        );
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
        let prefix = BgpPrefix::new(
            IpNet::from_str("1.0.1.0/24").unwrap(),
            4134,
        );

        assert!(prefix.contains(IpAddr::from_str("1.0.1.1").unwrap()));
        assert!(!prefix.contains(IpAddr::from_str("1.0.2.1").unwrap()));
    }
}
