use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::policy::normalize_domain;
use super::{DomainPolicyAction, DomainRecord, RouteIntent, RouteIntentAction, RouteIntentOwner};

pub const MIN_RESOLVE_RETRY_SECS: u64 = 5;
pub const MAX_RESOLVE_RETRY_SECS: u64 = 300;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainStateEntry {
    pub domain: String,
    pub action: DomainPolicyAction,
    pub generation: u64,
    pub record: Option<DomainRecord>,
    pub intents: Vec<RouteIntent>,
    pub last_error: Option<String>,
    pub resolution_in_flight: bool,
    pub failure_count: u32,
    pub next_retry_at: Option<u64>,
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
            resolution_in_flight: false,
            failure_count: 0,
            next_retry_at: None,
        }
    }

    pub fn begin_resolution(&mut self) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.last_error = None;
        self.resolution_in_flight = true;
        self.generation
    }

    pub fn set_action(&mut self, action: DomainPolicyAction) -> bool {
        if self.action == action {
            return false;
        }

        self.action = action;
        self.generation = self.generation.wrapping_add(1);
        self.last_error = None;
        self.resolution_in_flight = false;
        self.failure_count = 0;
        self.next_retry_at = None;
        self.rebuild_intents();
        true
    }

    pub fn is_resolution_in_flight(&self) -> bool {
        self.resolution_in_flight
    }

    pub fn accept_record(&mut self, record: DomainRecord) -> bool {
        if record.generation != self.generation
            || normalize_domain(&record.domain) != self.domain
        {
            return false;
        }

        self.record = Some(record);
        self.last_error = None;
        self.resolution_in_flight = false;
        self.failure_count = 0;
        self.next_retry_at = None;
        self.rebuild_intents();
        true
    }

    pub fn reject_record(
        &mut self,
        generation: u64,
        error: ResolveStateError,
        now: u64,
    ) -> bool {
        if generation != self.generation {
            return false;
        }

        self.last_error = Some(error.to_string());
        self.resolution_in_flight = false;
        self.failure_count = self.failure_count.saturating_add(1);
        self.next_retry_at = Some(
            now.saturating_add(resolve_retry_delay_secs(self.failure_count)),
        );
        true
    }

    pub fn can_retry_at(&self, now: u64) -> bool {
        self.next_retry_at
            .map(|retry_at| now >= retry_at)
            .unwrap_or(true)
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

        let Some(action) = RouteIntentAction::from_policy(self.action) else {
            return;
        };

        for ip in record.all_ips() {
            self.intents.push(RouteIntent::from_ip(
                ip,
                action,
                RouteIntentOwner::Domain(self.domain.clone()),
                self.generation,
                record.stale_until,
            ));
        }
    }

    /// True when the record has passed its local stale grace window and can no
    /// longer produce desired routing state.
    pub fn is_unusable_at(&self, now: u64) -> bool {
        self.record
            .as_ref()
            .map(|record| record.is_unusable_at(now))
            .unwrap_or(true)
    }

    /// True when a background refresh should be scheduled.
    pub fn needs_refresh_at(&self, now: u64) -> bool {
        self.record
            .as_ref()
            .map(|record| record.needs_refresh_at(now))
            .unwrap_or(true)
    }
}

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
        let entry = self
            .entries
            .entry(domain.clone())
            .or_insert_with(|| DomainStateEntry::new(&domain, action));

        entry.set_action(action);
        entry
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
            if entry.is_unusable_at(now) {
                continue;
            }
            result.extend(entry.intents.iter().cloned());
        }

        result
    }

    pub fn domains_needing_refresh(&self, now: u64) -> Vec<String> {
        let mut result: Vec<String> = self
            .entries
            .values()
            .filter(|entry| {
                entry.needs_refresh_at(now)
                    && !entry.is_resolution_in_flight()
                    && entry.can_retry_at(now)
            })
            .map(|entry| entry.domain.clone())
            .collect();

        result.sort();
        result
    }

    pub fn entries(&self) -> impl Iterator<Item = &DomainStateEntry> {
        self.entries.values()
    }
}

fn resolve_retry_delay_secs(failure_count: u32) -> u64 {
    let shift = failure_count.saturating_sub(1).min(63);
    MIN_RESOLVE_RETRY_SECS
        .saturating_mul(1_u64 << shift)
        .min(MAX_RESOLVE_RETRY_SECS)
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
        DomainRecord::new_with_stale_grace(
            domain,
            vec!["1.2.3.4".parse::<IpAddr>().unwrap()],
            vec!["2001:db8::1".parse::<IpAddr>().unwrap()],
            100,
            1_000,
            100,
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
        assert_eq!(entry.intents[0].generation, 2);
        assert_eq!(entry.intents[0].expires_at, 1_200);
    }

    #[test]
    fn state_aggregates_desired_intents_before_stale_until() {
        let mut state = DomainState::new();
        let entry = state.upsert("example.com", DomainPolicyAction::Direct);
        let generation = entry.begin_resolution();
        assert!(entry.accept_record(record("example.com", generation)));

        assert_eq!(state.desired_intents(1_050).len(), 2);
        assert_eq!(state.desired_intents(1_100).len(), 2);
        assert_eq!(state.desired_intents(1_199).len(), 2);
        assert!(state.desired_intents(1_200).is_empty());
    }

    #[test]
    fn refresh_starts_before_dns_expiry() {
        let mut state = DomainState::new();
        let entry = state.upsert("example.com", DomainPolicyAction::Direct);
        let generation = entry.begin_resolution();
        assert!(entry.accept_record(record("example.com", generation)));

        assert!(state.domains_needing_refresh(1_074).is_empty());
        assert_eq!(
            state.domains_needing_refresh(1_075),
            vec!["example.com".to_string()]
        );
    }

    #[test]
    fn stale_window_keeps_routes_during_transient_refresh_failure() {
        let mut state = DomainState::new();
        let entry = state.upsert("example.com", DomainPolicyAction::Direct);
        let generation = entry.begin_resolution();
        assert!(entry.accept_record(record("example.com", generation)));

        assert!(!entry.is_unusable_at(1_150));
        assert!(entry.is_unusable_at(1_200));
        assert_eq!(state.desired_intents(1_150).len(), 2);
        assert!(state.desired_intents(1_200).is_empty());
    }

    #[test]
    fn refresh_is_not_scheduled_twice_while_resolution_is_in_flight() {
        let mut state = DomainState::new();
        let entry = state.upsert("example.com", DomainPolicyAction::Direct);
        assert_eq!(entry.begin_resolution(), 1);

        assert!(state.domains_needing_refresh(1_000).is_empty());
        assert!(state.get("example.com").unwrap().is_resolution_in_flight());

        let entry = state.get_mut("example.com").unwrap();
        assert!(entry.reject_record(1, ResolveStateError::Timeout, 1_000));
        assert!(state.domains_needing_refresh(1_000).is_empty());
        assert!(state.domains_needing_refresh(1_004).is_empty());
        assert_eq!(
            state.domains_needing_refresh(1_005),
            vec!["example.com".to_string()]
        );
    }

    #[test]
    fn resolution_failure_uses_exponential_backoff_with_cap() {
        let mut entry = DomainStateEntry::new("example.com", DomainPolicyAction::Direct);
        assert_eq!(entry.begin_resolution(), 1);

        let expected = [5_u64, 10, 20, 40, 80, 160, 300, 300];
        let mut now = 1_000_u64;
        for (index, delay) in expected.into_iter().enumerate() {
            assert!(entry.reject_record(
                (index as u64) + 1,
                ResolveStateError::Timeout,
                now,
            ));
            assert_eq!(entry.next_retry_at, Some(now + delay));
            now += delay;
            if index + 1 < expected.len() {
                assert_eq!(entry.begin_resolution(), (index as u64) + 2);
            }
        }

        assert_eq!(entry.failure_count, 8);
        assert_eq!(MAX_RESOLVE_RETRY_SECS, 300);
    }

    #[test]
    fn successful_resolution_resets_failure_backoff() {
        let mut entry = DomainStateEntry::new("example.com", DomainPolicyAction::Direct);
        assert_eq!(entry.begin_resolution(), 1);
        assert!(entry.reject_record(1, ResolveStateError::ServFail, 1_000));
        assert_eq!(entry.failure_count, 1);
        assert_eq!(entry.next_retry_at, Some(1_005));

        assert_eq!(entry.begin_resolution(), 2);
        assert!(entry.accept_record(record("example.com", 2)));
        assert_eq!(entry.failure_count, 0);
        assert_eq!(entry.next_retry_at, None);
    }

    #[test]
    fn no_override_does_not_create_route_intents() {
        let mut entry = DomainStateEntry::new("example.com", DomainPolicyAction::NoOverride);
        let generation = entry.begin_resolution();
        assert!(entry.accept_record(record("example.com", generation)));
        assert!(entry.intents.is_empty());
    }
}
