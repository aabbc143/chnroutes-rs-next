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

/// A resolution request allocated by DomainState before the async DNS work.
///
/// The generation is the concurrency fence: only a result carrying the
/// current generation may commit into DomainState.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionRequest {
    pub domain: String,
    pub generation: u64,
    pub action: DomainPolicyAction,
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

    /// Start a resolution without awaiting the resolver.
    ///
    /// This separation is intentional: callers can issue multiple DNS
    /// requests concurrently and later commit results in any completion order.
    pub fn begin_resolution(&mut self, domain: &str) -> ResolutionRequest {
        let normalized = super::policy::normalize_domain(domain);
        let action = self.policy.evaluate(&normalized).action;
        let generation = self
            .state
            .upsert(&normalized, action)
            .begin_resolution();

        ResolutionRequest {
            domain: normalized,
            generation,
            action,
        }
    }

    /// Commit or reject a resolver result against its generation fence.
    pub fn finish_resolution(
        &mut self,
        request: &ResolutionRequest,
        result: Result<super::DomainRecord, ResolveError>,
    ) -> Result<ResolveOutcome, ResolveError> {
        let entry = self
            .state
            .get_mut(&request.domain)
            .expect("state entry must exist after begin_resolution");

        match result {
            Ok(record) => {
                if entry.accept_record(record) {
                    Ok(ResolveOutcome::Accepted)
                } else {
                    Ok(ResolveOutcome::Stale)
                }
            }
            Err(error) => {
                entry.reject_record(
                    request.generation,
                    super::state::ResolveStateError::from(&error),
                );
                Err(error)
            }
        }
    }

    /// Resolve one domain using the action selected by the current policy.
    ///
    /// The public begin/finish pair remains available when callers need
    /// concurrent resolution. This convenience method is the sequential form.
    pub async fn resolve_domain(
        &mut self,
        domain: &str,
    ) -> Result<ResolveOutcome, ResolveError> {
        let request = self.begin_resolution(domain);
        let result = self.resolver.resolve(&request.domain, request.generation).await;
        self.finish_resolution(&request, result)
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

    #[test]
    fn older_resolution_result_is_discarded() {
        let policy = DomainPolicy::new(DomainPolicyAction::Direct);
        let mut runtime = DomainRuntime::new(policy, TestResolver);

        let first = runtime.begin_resolution("example.com");
        let second = runtime.begin_resolution("example.com");

        let old_record = DomainRecord::new(
            "example.com",
            vec!["1.1.1.1".parse::<IpAddr>().unwrap()],
            vec![],
            60,
            100,
            "test",
            first.generation,
        );
        let new_record = DomainRecord::new(
            "example.com",
            vec!["2.2.2.2".parse::<IpAddr>().unwrap()],
            vec![],
            60,
            100,
            "test",
            second.generation,
        );

        assert_eq!(
            runtime.finish_resolution(&first, Ok(old_record)).unwrap(),
            ResolveOutcome::Stale
        );
        assert_eq!(
            runtime.finish_resolution(&second, Ok(new_record)).unwrap(),
            ResolveOutcome::Accepted
        );

        let entry = runtime.state().get("example.com").unwrap();
        assert_eq!(entry.generation, second.generation);
        assert_eq!(
            entry.record.as_ref().unwrap().a,
            vec!["2.2.2.2".parse::<IpAddr>().unwrap()]
        );
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
        assert_eq!(runtime.state().get("example.com").unwrap().action, DomainPolicyAction::Direct);
        assert_eq!(runtime.state().get("example.com").unwrap().intents.len(), 2);
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
