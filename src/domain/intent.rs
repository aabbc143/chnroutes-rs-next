use std::net::IpAddr;

use ipnet::{IpNet, Ipv4Net, Ipv6Net};
use serde::{Deserialize, Serialize};

use super::policy::DomainPolicyAction;

/// The enforcement action requested for an IP route intent.
///
/// This intentionally mirrors the policy vocabulary instead of pretending
/// that every action is a literal operating-system route operation. The
/// RouteBackend decides which actions it can enforce directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RouteIntentAction {
    Direct,
    Proxy,
    Auto,
    Block,
}

/// Identifies the owner of an intent.
///
/// Ownership is required for reconciliation: the same IP can be produced by
/// multiple domains, and removing one domain must not remove an intent still
/// required by another owner.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RouteIntentOwner {
    Domain(String),
}

/// A desired routing state produced by the Domain Routing layer.
///
/// A DNS A/AAAA answer becomes a host route (/32 or /128). The intent is
/// deliberately not a Windows route-table entry: gateway, interface index,
/// metrics, and platform-specific details remain RouteBackend concerns.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RouteIntent {
    pub destination: IpNet,
    pub action: RouteIntentAction,
    pub owner: RouteIntentOwner,
    pub generation: u64,
    pub expires_at: u64,
}

impl RouteIntent {
    pub fn new(
        destination: IpNet,
        action: RouteIntentAction,
        owner: RouteIntentOwner,
        generation: u64,
        expires_at: u64,
    ) -> Self {
        Self {
            destination,
            action,
            owner,
            generation,
            expires_at,
        }
    }

    pub fn from_ip(
        ip: IpAddr,
        action: RouteIntentAction,
        owner: RouteIntentOwner,
        generation: u64,
        expires_at: u64,
    ) -> Self {
        let destination = match ip {
            IpAddr::V4(ip) => IpNet::V4(
                Ipv4Net::new(ip, 32).expect("IPv4 host prefix must always be valid"),
            ),
            IpAddr::V6(ip) => IpNet::V6(
                Ipv6Net::new(ip, 128).expect("IPv6 host prefix must always be valid"),
            ),
        };

        Self::new(destination, action, owner, generation, expires_at)
    }

    pub fn is_expired_at(&self, now: u64) -> bool {
        now >= self.expires_at
    }

    pub fn is_expired(&self) -> bool {
        self.is_expired_at(unix_now())
    }
}

impl RouteIntentAction {
    /// Convert an effective domain policy action into a route intent action.
    ///
    /// NoOverride is represented as None: it means that the domain policy did
    /// not claim ownership of the final routing decision and therefore must
    /// never create a RouteIntent.
    pub fn from_policy(action: DomainPolicyAction) -> Option<Self> {
        match action {
            DomainPolicyAction::Direct => Some(Self::Direct),
            DomainPolicyAction::Proxy => Some(Self::Proxy),
            DomainPolicyAction::Auto => Some(Self::Auto),
            DomainPolicyAction::Block => Some(Self::Block),
            DomainPolicyAction::NoOverride => None,
        }
    }
}

fn unix_now() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(value: &str) -> IpAddr {
        value.parse().unwrap()
    }

    #[test]
    fn ipv4_domain_answer_becomes_host_route() {
        let intent = RouteIntent::from_ip(
            ip("1.2.3.4"),
            RouteIntentAction::Direct,
            RouteIntentOwner::Domain("example.com".into()),
            7,
            1_300,
        );

        assert_eq!(intent.destination.to_string(), "1.2.3.4/32");
        assert_eq!(intent.generation, 7);
        assert_eq!(intent.expires_at, 1_300);
    }

    #[test]
    fn ipv6_domain_answer_becomes_host_route() {
        let intent = RouteIntent::from_ip(
            ip("2001:db8::1"),
            RouteIntentAction::Proxy,
            RouteIntentOwner::Domain("example.com".into()),
            8,
            2_000,
        );

        assert_eq!(intent.destination.to_string(), "2001:db8::1/128");
        assert_eq!(intent.action, RouteIntentAction::Proxy);
    }

    #[test]
    fn owner_participates_in_identity() {
        let a = RouteIntent::from_ip(
            ip("1.2.3.4"),
            RouteIntentAction::Direct,
            RouteIntentOwner::Domain("a.example".into()),
            1,
            100,
        );
        let b = RouteIntent::from_ip(
            ip("1.2.3.4"),
            RouteIntentAction::Direct,
            RouteIntentOwner::Domain("b.example".into()),
            1,
            100,
        );

        assert_ne!(a, b);
    }

    #[test]
    fn expiry_is_inclusive() {
        let intent = RouteIntent::new(
            "1.2.3.4/32".parse().unwrap(),
            RouteIntentAction::Direct,
            RouteIntentOwner::Domain("example.com".into()),
            1,
            100,
        );

        assert!(!intent.is_expired_at(99));
        assert!(intent.is_expired_at(100));
    }

    #[test]
    fn policy_action_conversion_is_explicit() {
        assert_eq!(
            RouteIntentAction::from_policy(DomainPolicyAction::Direct),
            RouteIntentAction::Direct
        );
        assert_eq!(
            RouteIntentAction::from_policy(DomainPolicyAction::Proxy),
            RouteIntentAction::Proxy
        );
        assert_eq!(
            RouteIntentAction::from_policy(DomainPolicyAction::Auto),
            RouteIntentAction::Auto
        );
        assert_eq!(
            RouteIntentAction::from_policy(DomainPolicyAction::Block),
            RouteIntentAction::Block
        );
    }
}
