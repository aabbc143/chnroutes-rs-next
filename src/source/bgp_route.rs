use super::bgp_decision::BgpNetworkType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BgpRouteDecision {
    Direct,
    Proxy,
    Unknown,
}

impl BgpRouteDecision {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Proxy => "proxy",
            Self::Unknown => "unknown",
        }
    }
}

pub fn decide_bgp_route(network_type: BgpNetworkType) -> BgpRouteDecision {
    match network_type {
        BgpNetworkType::MainlandIsp => BgpRouteDecision::Direct,
        BgpNetworkType::MainlandHosting => BgpRouteDecision::Unknown,
        BgpNetworkType::HongKong => BgpRouteDecision::Unknown,
        BgpNetworkType::Overseas => BgpRouteDecision::Proxy,
        BgpNetworkType::Unknown => BgpRouteDecision::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mainland_isp_is_direct() {
        assert_eq!(
            decide_bgp_route(BgpNetworkType::MainlandIsp),
            BgpRouteDecision::Direct
        );
    }

    #[test]
    fn test_mainland_hosting_is_unknown() {
        assert_eq!(
            decide_bgp_route(BgpNetworkType::MainlandHosting),
            BgpRouteDecision::Unknown
        );
    }

    #[test]
    fn test_hong_kong_is_unknown() {
        assert_eq!(
            decide_bgp_route(BgpNetworkType::HongKong),
            BgpRouteDecision::Unknown
        );
    }

    #[test]
    fn test_overseas_is_proxy() {
        assert_eq!(
            decide_bgp_route(BgpNetworkType::Overseas),
            BgpRouteDecision::Proxy
        );
    }

    #[test]
    fn test_unknown_is_unknown() {
        assert_eq!(
            decide_bgp_route(BgpNetworkType::Unknown),
            BgpRouteDecision::Unknown
        );
    }
}
