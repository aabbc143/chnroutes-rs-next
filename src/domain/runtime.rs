use super::{
    DomainPolicy, DomainPolicyAction, DomainState, ResolveError, Resolver, RouteIntent,
};

/// Result of a domain resolution attempt.
///
/// A resolver result can arrive after a newer generation has already started.
/// In that case the result is deliberately discarded rather than overwriting
/// current runtime state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveOutcome {
    Accepted,
    Stale,
}

/// Coordinates policy evaluation, resolution generation, and DomainState.
///
/// This is intentionally not a RouteBackend or a traffic proxy. Its only job
/// is to turn a domain policy into current domain runtime state. Reconciliation
/// and backend enforcement remain separate.
pub struct DomainRuntime<R> {
    policy: DomainPolicy,
    resolver: R,
    state: DomainState,
}

impl<R> DomainRuntime<R>
where
    R: Resolver,
{
    pub fn new(policy: DomainPolicy, resolver: R) -> Self {
        Self {
            policy,
            resolver,
            state: DomainState::new(),
        }
    }

    pub fn policy(&self) -> &DomainPolicy {
        &self.policy
    }

    pub fn policy_mut(&mut self) -> &mut DomainPolicy {
        &mut self.policy
    }

    pub fn resolver(&self) -> &R {
        &self.resolver
    }

    pub fn state(&self) -> &DomainState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut DomainState {
        &mut self.state
    }

    pub fn desired_intents(&self, now: u64) -> Vec<RouteIntent> {
        self.state.desired_intents(now)
    }

    /// Resolve one domain using the action selected by the current policy.
    ///
    /// The generation is allocated before awaiting the resolver. A late
    /// response from an older request therefore cannot overwrite newer state.
    pub async fn resolve_domain(
        &mut self,
        domain: &str,
    ) -> Result<ResolveOutcome, ResolveError> {
        let normalized = super::policy::normalize_domain(domain);
        let action = self.policy.evaluate(&normalized).action;

        let generation = {
            let entry = self.state.upsert(normalized.clone(), action);
            entry.begin_resolution()
        };

        let result = self.resolver.resolve(&normalized, generation).await;

        match result {
            Ok(record) => {
                let entry = self
                    .state
                    .get_mut(&normalized)
                    .expect("state entry must exist after begin_resolution");

                if entry.accept_record(record) {
                    Ok(ResolveOutcome::Accepted)
                } else {
                    Ok(ResolveOutcome::Stale)
                }
            }
            Err(error) => {
                let entry = self
                    .state
                    .get_mut(&normalized)
                    .expect("state entry must exist after begin_resolution");

                entry.reject_record(generation, super::state::ResolveStateError::from(&error));
                Err(error)
            }
        }
    }

    /// Remove a domain from policy/runtime ownership.
    pub fn remove_domain(&mut self, domain: &str) -> Option<super::DomainStateEntry> {
        self.state.remove(domain)
    }
}

impl From<&ResolveError> for super::state::ResolveStateError {
    fn from(error: &ResolveError) -> Self {
        match error {
            ResolveError::NxDomain => Self::NxDomain,
            ResolveError::ServFail => Self::ServFail,
            ResolveError::Timeout => Self::Timeout,
            ResolveError::Cancelled => Self::Cancelled,
            ResolveError::Other(message) => Self::Other(message.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{DomainRecord, ResolverFuture};
    use std::net::IpAddr;

    struct TestResolver;

    impl Resolver for TestResolver {
        fn identity(&self) -> &str {
            "test"
        }

        fn resolve<'a>(
            &'a self,
            domain: &'a str,
            generation: u64,
        ) -> ResolverFuture<'a, Result<DomainRecord, ResolveError>> {
            Box::pin(async move {
                Ok(DomainRecord::new(
                    domain,
                    vec!["1.2.3.4".parse::<IpAddr>().unwrap()],
                    vec!["2001:db8::1".parse::<IpAddr>().unwrap()],
                    60,
                    100,
                    "test",
                    generation,
                ))
            })
        }
    }

    #[tokio::test]
    async fn resolution_flows_from_policy_to_state() {
        let mut policy = DomainPolicy::new(DomainPolicyAction::Direct);
        policy.add_rule(super::super::policy::DomainRule::new(
            "proxy-example",
            super::super::policy::DomainRuleMatcher::Suffix("example.com".into()),
            DomainPolicyAction::Proxy,
            100,
            super::super::policy::DomainRuleSource::User,
        ));

        let mut runtime = DomainRuntime::new(policy, TestResolver);

        let outcome = runtime.resolve_domain("WWW.Example.COM.").await.unwrap();

        assert_eq!(outcome, ResolveOutcome::Accepted);
        let entry = runtime.state().get("www.example.com").unwrap();
        assert_eq!(entry.action, DomainPolicyAction::Proxy);
        assert_eq!(entry.generation, 1);
        assert_eq!(entry.intents.len(), 2);
        assert_eq!(runtime.resolver().identity(), "test");
    }

    #[tokio::test]
    async fn no_override_does_not_create_domain_intents() {
        let mut policy = DomainPolicy::new(DomainPolicyAction::Direct);
        policy.add_rule(super::super::policy::DomainRule::new(
            "keep-default",
            super::super::policy::DomainRuleMatcher::Exact("example.com".into()),
            DomainPolicyAction::NoOverride,
            100,
            super::super::policy::DomainRuleSource::User,
        ));

        let mut runtime = DomainRuntime::new(policy, TestResolver);

        let outcome = runtime.resolve_domain("example.com").await.unwrap();

        assert_eq!(outcome, ResolveOutcome::Accepted);
        assert!(runtime.state().get("example.com").unwrap().intents.is_empty());
    }

    #[tokio::test]
    async fn resolver_error_is_recorded_in_state() {
        struct ErrorResolver;

        impl Resolver for ErrorResolver {
            fn identity(&self) -> &str {
                "test-error"
            }

            fn resolve<'a>(
                &'a self,
                _domain: &'a str,
                _generation: u64,
            ) -> ResolverFuture<'a, Result<DomainRecord, ResolveError>> {
                Box::pin(async { Err(ResolveError::Timeout) })
            }
        }

        let policy = DomainPolicy::new(DomainPolicyAction::Direct);
        let mut runtime = DomainRuntime::new(policy, ErrorResolver);

        let error = runtime.resolve_domain("example.com").await.unwrap_err();

        assert_eq!(error, ResolveError::Timeout);
        assert_eq!(
            runtime.state().get("example.com").unwrap().last_error.as_deref(),
            Some("DNS resolution timed out")
        );
    }
}
