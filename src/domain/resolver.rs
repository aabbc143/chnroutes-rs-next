use std::future::Future;
use std::net::IpAddr;
use std::pin::Pin;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::policy::normalize_domain;

pub type ResolverFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub const DEFAULT_STALE_GRACE_SECS: u64 = 300;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainRecord {
    pub domain: String,
    pub a: Vec<IpAddr>,
    pub aaaa: Vec<IpAddr>,
    pub ttl: u64,
    pub resolved_at: u64,
    pub refresh_at: u64,
    pub expires_at: u64,
    pub stale_until: u64,
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
        Self::new_with_stale_grace(
            domain,
            a,
            aaaa,
            ttl,
            resolved_at,
            DEFAULT_STALE_GRACE_SECS,
            resolver,
            generation,
        )
    }

    pub fn new_with_stale_grace(
        domain: impl AsRef<str>,
        a: Vec<IpAddr>,
        aaaa: Vec<IpAddr>,
        ttl: u64,
        resolved_at: u64,
        stale_grace_secs: u64,
        resolver: impl Into<String>,
        generation: u64,
    ) -> Self {
        let refresh_at = resolved_at.saturating_add(refresh_offset(ttl));
        let expires_at = resolved_at.saturating_add(ttl);
        let stale_until = expires_at.saturating_add(stale_grace_secs);

        Self {
            domain: normalize_domain(domain.as_ref()),
            a,
            aaaa,
            ttl,
            resolved_at,
            refresh_at,
            expires_at,
            stale_until,
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

    pub fn needs_refresh_at(&self, now: u64) -> bool {
        now >= self.refresh_at
    }

    pub fn is_stale_at(&self, now: u64) -> bool {
        now >= self.expires_at && now < self.stale_until
    }

    pub fn is_unusable_at(&self, now: u64) -> bool {
        now >= self.stale_until
    }

    pub fn is_expired(&self) -> bool {
        self.is_expired_at(unix_now())
    }

    pub fn is_empty(&self) -> bool {
        self.a.is_empty() && self.aaaa.is_empty()
    }
}

fn refresh_offset(ttl: u64) -> u64 {
    ttl.saturating_mul(3) / 4
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    NxDomain,
    NoData,
    ServFail,
    Timeout,
    Cancelled,
    Other(String),
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NxDomain => write!(f, "NXDOMAIN"),
            Self::NoData => write!(f, "NODATA"),
            Self::ServFail => write!(f, "SERVFAIL"),
            Self::Timeout => write!(f, "DNS resolution timed out"),
            Self::Cancelled => write!(f, "DNS resolution cancelled"),
            Self::Other(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for ResolveError {}

pub trait Resolver: Send + Sync {
    fn identity(&self) -> &str;

    fn resolve<'a>(
        &'a self,
        domain: &'a str,
        generation: u64,
    ) -> ResolverFuture<'a, Result<DomainRecord, ResolveError>>;
}


/// Resolver implementation backed by the operating system's DNS resolver.
///
/// The platform call is blocking, so the async adapter executes it on Tokio's
/// blocking pool instead of blocking the async worker thread.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemDnsResolver;

impl Resolver for SystemDnsResolver {
    fn identity(&self) -> &str {
        "system"
    }

    fn resolve<'a>(
        &'a self,
        domain: &'a str,
        generation: u64,
    ) -> ResolverFuture<'a, Result<DomainRecord, ResolveError>> {
        let domain = domain.to_owned();

        Box::pin(async move {
            tokio::task::spawn_blocking(move || resolve_system(&domain, generation))
                .await
                .map_err(|error| ResolveError::Other(format!("DNS worker failed: {error}")))?
        })
    }
}

fn resolve_system(domain: &str, generation: u64) -> Result<DomainRecord, ResolveError> {
    const A: u16 = 1;
    const AAAA: u16 = 28;

    let a = system_resolver::lookup(domain, A).map_err(classify_system_error)?;
    let aaaa = system_resolver::lookup(domain, AAAA).map_err(classify_system_error)?;

    let a_ips = decode_addresses(&a, A)?;
    let aaaa_ips = decode_addresses(&aaaa, AAAA)?;

    if a_ips.is_empty() && aaaa_ips.is_empty() {
        return Err(ResolveError::NoData);
    }

    let ttl = a
        .iter()
        .chain(aaaa.iter())
        .map(|record| record.ttl.as_secs())
        .min()
        .unwrap_or(0);

    Ok(DomainRecord::from_now(
        domain,
        a_ips,
        aaaa_ips,
        ttl,
        "system",
        generation,
    ))
}

fn decode_addresses(
    records: &[system_resolver::Record],
    rtype: u16,
) -> Result<Vec<IpAddr>, ResolveError> {
    records
        .iter()
        .filter(|record| record.rtype == rtype)
        .map(|record| {
            match rtype {
                1 if record.rdata.len() == 4 => {
                    Ok(IpAddr::from([record.rdata[0], record.rdata[1], record.rdata[2], record.rdata[3]]))
                }
                28 if record.rdata.len() == 16 => {
                    let mut bytes = [0_u8; 16];
                    bytes.copy_from_slice(&record.rdata);
                    Ok(IpAddr::from(bytes))
                }
                _ => Err(ResolveError::Other(format!(
                    "invalid DNS RDATA for type {rtype}: {} bytes",
                    record.rdata.len()
                ))),
            }
        })
        .collect()
}

fn classify_system_error(error: system_resolver::Error) -> ResolveError {
    match error {
        system_resolver::Error::NameDoesNotExist => ResolveError::NxDomain,
        system_resolver::Error::NoResponse => ResolveError::Timeout,
        system_resolver::Error::ResponseCode { rcode: 2 } => ResolveError::ServFail,
        other => ResolveError::Other(other.to_string()),
    }
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
    fn record_normalizes_domain_and_calculates_lifecycle() {
        let record = DomainRecord::new_with_stale_grace(
            " WWW.Example.COM. ",
            vec![ip("1.2.3.4")],
            vec![ip("2001:db8::1")],
            300,
            1_000,
            120,
            "system",
            7,
        );

        assert_eq!(record.domain, "www.example.com");
        assert_eq!(record.ttl, 300);
        assert_eq!(record.resolved_at, 1_000);
        assert_eq!(record.refresh_at, 1_225);
        assert_eq!(record.expires_at, 1_300);
        assert_eq!(record.stale_until, 1_420);
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
    fn ttl_refreshes_before_expiry() {
        let record = DomainRecord::new_with_stale_grace(
            "example.com",
            vec![],
            vec![],
            60,
            100,
            300,
            "system",
            1,
        );

        assert_eq!(record.refresh_at, 145);
        assert!(!record.needs_refresh_at(144));
        assert!(record.needs_refresh_at(145));
        assert!(!record.is_expired_at(159));
        assert!(record.is_expired_at(160));
    }

    #[test]
    fn stale_window_is_distinct_from_dns_expiry() {
        let record = DomainRecord::new_with_stale_grace(
            "example.com",
            vec![],
            vec![],
            60,
            100,
            120,
            "system",
            1,
        );

        assert!(record.is_stale_at(160));
        assert!(!record.is_unusable_at(160));
        assert!(!record.is_stale_at(219));
        assert!(record.is_unusable_at(220));
    }

    #[test]
    fn zero_ttl_refreshes_immediately() {
        let record = DomainRecord::new_with_stale_grace(
            "example.com",
            vec![],
            vec![],
            0,
            100,
            120,
            "system",
            1,
        );

        assert_eq!(record.refresh_at, 100);
        assert_eq!(record.expires_at, 100);
        assert_eq!(record.stale_until, 220);
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
}
