use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::{DomainPolicyAction, DomainRecord, RouteIntent, RouteIntentAction, RouteIntentOwner};
use super::policy::normalize_domain;

/// Runtime state belonging to one domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainStateEntry {
    pub domain: String,
    pub action: DomainPolicyAction,
    pub generation: u64,
    pub record: Option<DomainRecord>,
    pub intents: Vec<RouteIntent>,
    pub last_error: Option<String>,
}

impl DomainStateEntry {
    pub fn new(domain: impl AsRef<str>, action: DomainPolicyAction) -> Self {
        Self {
            domain: normalize_domain(domain.as_ref()),
            action,
            generation: 0,
            record: None,
            intents: Vec::new(),
            last_error: None,
        }
    }

    pub fn begin_resolution(&mut self) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.last_error = None;
        self.generation
    }

    /// Accept a resolver result only if it belongs to the current generation.
    ///
    /// This prevents a slow DNS response from an older request from
    /// overwriting a newer result.
    pub fn accept_record(&mut self, record: DomainRecord) -> bool {
        if record.generation != self.generation {
            return false;
        }

        self.record = Some(record);
        self.last_error = None;
        self.rebuild_intents();
        true
    }

    pub fn reject_record(&mut self, generation: u64, error: ResolveStateError) -> bool {
        if generation != self.generation {
            return false;
        }

        self.last_error = Some(error.to_string());
        true
    }

    pub fn clear_record(&mut self) {
        self.record = None;
        self.intents.clear();
    }

    pub fn rebuild_intents(&mut self) {
        self.intents.clear();

        let Some(record) = self.record.as_ref() else {
            return;
        };

        let Some(action) = Option::<RouteIntentAction>::from(self.action) else {
            return;
        };

        for ip in record.all_ips() {
            self.intents.push(RouteIntent::from_ip(
                ip,
                action,
                RouteIntentOwner::Domain(self.domain.clone()),
                record.generation,
                record.expires_at,
            ));
        }
    }

    pub fn is_expired_at(&self, now: u64) -> bool {
        self.record
            .as_ref()
            .map(|record| record.is_expired_at(now))
            .unwrap_or(true)
    }
}

/// Aggregate runtime state for all domains.
///
/// The state is intentionally independent from the RouteBackend. It is the
/// source from which the Reconciler builds desired RouteIntents.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DomainState {
    entries: HashMap<String, DomainStateEntry>,
}

impl DomainState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, domain: &str) -> Option<&DomainStateEntry> {
        self.entries.get(&normalize_domain(domain))
    }

    pub fn get_mut(&mut self, domain: &str) -> Option<&mut DomainStateEntry> {
        let domain = normalize_domain(domain);
        self.entries.get_mut(&domain)
    }

    pub fn upsert(
        &mut self,
        domain: impl AsRef<str>,
        action: DomainPolicyAction,
    ) -> &mut DomainStateEntry {
        let domain = normalize_domain(domain.as_ref());
        self.entries
            .entry(domain.clone())
            .or_insert_with(|| DomainStateEntry::new(domain, action))
    }

    pub fn remove(&mut self, domain: &str) -> Option<DomainStateEntry> {
        self.entries.remove(&normalize_domain(domain))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn desired_intents(&self, now: u64) -> Vec<RouteIntent> {
        let mut result = Vec::new();

        for entry in self.entries.values() {
            if entry.is_expired_at(now) {
                continue;
            }
            result.extend(entry.intents.iter().cloned());
        }

        result
    }

    pub fn domains_needing_refresh(&self, now: u64) -> Vec<String> {
        self.entries
            .values()
            .filter(|entry| entry.record.is_none() || entry.is_expired_at(now))
            .map(|entry| entry.domain.clone())
            .collect()
    }

    pub fn entries(&self) -> impl Iterator<Item = &DomainStateEntry> {
        self.entries.values()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveStateError {
    NxDomain,
    ServFail,
    Timeout,
    Cancelled,
    Other(String),
}

impl std::fmt::Display for ResolveStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NxDomain => f.write_str("NXDOMAIN"),
            Self::ServFail => f.write_str("SERVFAIL"),
            Self::Timeout => f.write_str("DNS resolution timed out"),
            Self::Cancelled => f.write_str("DNS resolution cancelled"),
            Self::Other(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ResolveStateError {}


#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;

    fn record(domain: &str, generation: u64) -> DomainRecord {
        DomainRecord::new(
            domain,
            vec!["1.2.3.4".parse::<IpAddr>().unwrap()],
            vec!["2001:db8::1".parse::<IpAddr>().unwrap()],
            300,
            1_000,
            "test",
            generation,
        )
    }

    #[test]
    fn newer_generation_replaces_older_resolution() {
        let mut entry = DomainStateEntry::new("example.com", DomainPolicyAction::Direct);
        assert_eq!(entry.begin_resolution(), 1);
        assert!(entry.accept_record(record("example.com", 1)));

        assert_eq!(entry.begin_resolution(), 2);
        assert!(!entry.accept_record(record("example.com", 1)));
        assert!(entry.accept_record(record("example.com", 2)));

        assert_eq!(entry.record.as_ref().unwrap().generation, 2);
        assert_eq!(entry.intents.len(), 2);
    }

    #[test]
    fn state_aggregates_desired_intents() {
        let mut state = DomainState::new();
        let entry = state.upsert("example.com", DomainPolicyAction::Direct);
        let generation = entry.begin_resolution();
        assert!(entry.accept_record(record("example.com", generation)));

        assert_eq!(state.desired_intents(1_100).len(), 2);
    }

    #[test]
    fn expired_domain_is_not_desired() {
        let mut state = DomainState::new();
        let entry = state.upsert("example.com", DomainPolicyAction::Direct);
        let generation = entry.begin_resolution();
        assert!(entry.accept_record(record("example.com", generation)));

        assert!(state.desired_intents(1_300).is_empty());
        assert_eq!(
            state.domains_needing_refresh(1_300),
            vec!["example.com".to_string()]
        );
    }

    #[test]
    fn stale_error_does_not_replace_new_generation() {
        let mut entry = DomainStateEntry::new("example.com", DomainPolicyAction::Direct);
        let first = entry.begin_resolution();
        let second = entry.begin_resolution();

        assert!(!entry.reject_record(first, ResolveStateError::Timeout));
        assert!(entry.last_error.is_none());

        assert!(entry.reject_record(second, ResolveStateError::ServFail));
        assert_eq!(entry.last_error.as_deref(), Some("SERVFAIL"));
    }

    #[test]
    fn no_override_does_not_create_route_intents() {
        let mut entry = DomainStateEntry::new("example.com", DomainPolicyAction::NoOverride);
        let generation = entry.begin_resolution();
        assert!(entry.accept_record(record("example.com", generation)));
        assert!(entry.intents.is_empty());
    }
}
