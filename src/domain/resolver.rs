use std::future::Future;
use std::net::IpAddr;
use std::pin::Pin;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::policy::normalize_domain;

pub type ResolverFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A DNS resolution snapshot used by the Domain Routing layer.
///
/// resolved_at and expires_at are Unix timestamps in seconds. Keeping the
/// record independent from a concrete DNS library lets later implementations
/// use the Windows resolver, DoH, DoT, or another resolver without changing
/// the policy/reconcile layers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainRecord {
    pub domain: String,
    pub a: Vec<IpAddr>,
    pub aaaa: Vec<IpAddr>,
    pub ttl: u64,
    pub resolved_at: u64,
    pub expires_at: u64,
    pub resolver: String,
    pub generation: u64,
}

impl DomainRecord {
    pub fn new(
        domain: impl AsRef<str>,
        a: Vec<IpAddr>,
        aaaa: Vec<IpAddr>,
        ttl: u64,
        resolved_at: u64,
        resolver: impl Into<String>,
        generation: u64,
    ) -> Self {
        Self {
            domain: normalize_domain(domain.as_ref()),
            a,
            aaaa,
            ttl,
            resolved_at,
            expires_at: resolved_at.saturating_add(ttl),
            resolver: resolver.into(),
            generation,
        }
    }

    pub fn from_now(
        domain: impl AsRef<str>,
        a: Vec<IpAddr>,
        aaaa: Vec<IpAddr>,
        ttl: u64,
        resolver: impl Into<String>,
        generation: u64,
    ) -> Self {
        Self::new(
            domain,
            a,
            aaaa,
            ttl,
            unix_now(),
            resolver,
            generation,
        )
    }

    pub fn all_ips(&self) -> impl Iterator<Item = IpAddr> + '_ {
        self.a.iter().chain(self.aaaa.iter()).copied()
    }

    pub fn is_expired_at(&self, now: u64) -> bool {
        now >= self.expires_at
    }

    pub fn is_expired(&self) -> bool {
        self.is_expired_at(unix_now())
    }

    pub fn is_empty(&self) -> bool {
        self.a.is_empty() && self.aaaa.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    /// DNS explicitly reported that the queried name does not exist.
    NxDomain,
    /// The resolver reported a server-side failure.
    ServFail,
    /// The lookup exceeded its configured deadline.
    Timeout,
    /// The lookup was cancelled before completion.
    Cancelled,
    /// Transport/protocol/configuration error that does not fit the above
    /// categories.
    Other(String),
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NxDomain => write!(f, "NXDOMAIN"),
            Self::ServFail => write!(f, "SERVFAIL"),
            Self::Timeout => write!(f, "DNS resolution timed out"),
            Self::Cancelled => write!(f, "DNS resolution cancelled"),
            Self::Other(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for ResolveError {}

/// Resolver abstraction.
///
/// The trait deliberately owns neither policy nor route state. It only turns
/// a domain into a timestamped A/AAAA snapshot. generation is supplied by the
/// caller so a late result can be rejected by the state/reconcile layer.
pub trait Resolver: Send + Sync {
    fn identity(&self) -> &str;

    fn resolve<'a>(
        &'a self,
        domain: &'a str,
        generation: u64,
    ) -> ResolverFuture<'a, Result<DomainRecord, ResolveError>>;
}

fn unix_now() -> u64 {
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
    fn record_normalizes_domain_and_calculates_expiry() {
        let record = DomainRecord::new(
            " WWW.Example.COM. ",
            vec![ip("1.2.3.4")],
            vec![ip("2001:db8::1")],
            300,
            1_000,
            "system",
            7,
        );

        assert_eq!(record.domain, "www.example.com");
        assert_eq!(record.ttl, 300);
        assert_eq!(record.resolved_at, 1_000);
        assert_eq!(record.expires_at, 1_300);
        assert_eq!(record.generation, 7);
        assert_eq!(record.resolver, "system");
    }

    #[test]
    fn all_ips_contains_both_address_families() {
        let record = DomainRecord::new(
            "example.com",
            vec![ip("1.2.3.4")],
            vec![ip("2001:db8::1")],
            60,
            100,
            "system",
            1,
        );

        let ips: Vec<_> = record.all_ips().collect();
        assert_eq!(ips, vec![ip("1.2.3.4"), ip("2001:db8::1")]);
    }

    #[test]
    fn expiry_is_inclusive() {
        let record = DomainRecord::new(
            "example.com",
            vec![],
            vec![],
            60,
            100,
            "system",
            1,
        );

        assert!(!record.is_expired_at(159));
        assert!(record.is_expired_at(160));
    }

    #[test]
    fn empty_record_is_detectable() {
        let record = DomainRecord::new(
            "example.com",
            vec![],
            vec![],
            60,
            100,
            "system",
            1,
        );

        assert!(record.is_empty());
    }

    #[test]
    fn resolve_error_has_distinct_dns_states() {
        assert_ne!(ResolveError::NxDomain, ResolveError::ServFail);
        assert_ne!(ResolveError::ServFail, ResolveError::Timeout);
        assert_ne!(ResolveError::Timeout, ResolveError::Cancelled);
    }
}
