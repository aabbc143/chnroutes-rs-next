use super::bgp_evidence::BgpAsnEvidence;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BgpNetworkType {
    MainlandIsp,
    MainlandHosting,
    HongKong,
    Overseas,
    Unknown,
}

impl BgpNetworkType {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::MainlandIsp => "mainland-isp",
            Self::MainlandHosting => "mainland-hosting",
            Self::HongKong => "hong-kong",
            Self::Overseas => "overseas",
            Self::Unknown => "unknown",
        }
    }
}


pub fn classify_bgp_network(evidence: &BgpAsnEvidence) -> BgpNetworkType {
    let country = evidence.country.to_ascii_uppercase();
    let class = evidence.network_class.to_ascii_lowercase();

    if country == "HK" {
        return BgpNetworkType::HongKong;
    }

    if country != "CN" {
        return BgpNetworkType::Overseas;
    }

    if class.contains("hosting") {
        return BgpNetworkType::MainlandHosting;
    }

    if class.contains("eyeball")
        || class.contains("isp")
        || class.contains("access")
    {
        return BgpNetworkType::MainlandIsp;
    }

    BgpNetworkType::Unknown
}


#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(country: &str, class: &str) -> BgpAsnEvidence {
        BgpAsnEvidence {
            asn: 4134,
            name: "test".to_string(),
            network_class: class.to_string(),
            country: country.to_string(),
            registry: "APNIC".to_string(),
        }
    }


    #[test]
    fn test_cn_isp() {
        let result = classify_bgp_network(
            &evidence("CN", "Eyeball")
        );

        assert_eq!(result, BgpNetworkType::MainlandIsp);
    }


    #[test]
    fn test_cn_hosting() {
        let result = classify_bgp_network(
            &evidence("CN", "Hosting")
        );

        assert_eq!(result, BgpNetworkType::MainlandHosting);
    }


    #[test]
    fn test_hk() {
        let result = classify_bgp_network(
            &evidence("HK", "Eyeball")
        );

        assert_eq!(result, BgpNetworkType::HongKong);
    }


    #[test]
    fn test_overseas() {
        let result = classify_bgp_network(
            &evidence("US", "Hosting")
        );

        assert_eq!(result, BgpNetworkType::Overseas);
    }


    #[test]
    fn test_unknown() {
        let result = classify_bgp_network(
            &evidence("CN", "Unknown")
        );

        assert_eq!(result, BgpNetworkType::Unknown);
    }
}
