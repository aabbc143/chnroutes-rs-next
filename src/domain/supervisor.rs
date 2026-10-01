use super::{DomainRuntime, ReconcileError, ReconcilePlan, ResolveError, ResolveOutcome, Reconciler, Resolver, RouteBackend};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct SupervisorReport {
    pub refreshed: Vec<String>,
    pub accepted: usize,
    pub stale: usize,
    pub dns_errors: Vec<(String, ResolveError)>,
    pub reconcile: ReconcilePlan,
}

pub struct DomainSupervisor<R, B> {
    runtime: DomainRuntime<R>,
    reconciler: Reconciler<B>,
}

impl<R, B> DomainSupervisor<R, B>
where
    R: Resolver,
    B: RouteBackend,
{
    pub fn new(runtime: DomainRuntime<R>, backend: B) -> Self {
        Self { runtime, reconciler: Reconciler::new(backend) }
    }

    pub fn runtime(&self) -> &DomainRuntime<R> { &self.runtime }
    pub fn runtime_mut(&mut self) -> &mut DomainRuntime<R> { &mut self.runtime }
    pub fn reconciler(&self) -> &Reconciler<B> { &self.reconciler }
    pub fn reconciler_mut(&mut self) -> &mut Reconciler<B> { &mut self.reconciler }

    pub async fn tick(&mut self, now: u64) -> Result<SupervisorReport, ReconcileError> {
        let domains = self.runtime.state().domains_needing_refresh(now);
        let mut report = SupervisorReport { refreshed: domains.clone(), ..Default::default() };

        for domain in domains {
            let request = self.runtime.begin_resolution(&domain);
            let result = self.runtime.resolver().resolve(&request.domain, request.generation).await;
            match self.runtime.finish_resolution_at(&request, result, now) {
                Ok(ResolveOutcome::Accepted) => report.accepted += 1,
                Ok(ResolveOutcome::Stale) => report.stale += 1,
                Err(error) => report.dns_errors.push((domain, error)),
            }
        }

        report.reconcile = self.reconciler.reconcile(&self.runtime.desired_intents(now), now).await?;
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{DomainPolicy, DomainPolicyAction, DomainRecord, RouteIntent, ResolverFuture};
    use std::net::IpAddr;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct TestResolver { calls: Arc<Mutex<Vec<String>>> }

    impl Resolver for TestResolver {
        fn identity(&self) -> &str { "test" }
        fn resolve<'a>(&'a self, domain: &'a str, generation: u64) -> ResolverFuture<'a, Result<DomainRecord, ResolveError>> {
            let calls = self.calls.clone();
            Box::pin(async move {
                calls.lock().unwrap().push(domain.to_string());
                Ok(DomainRecord::new_with_stale_grace(domain, vec!["1.2.3.4".parse::<IpAddr>().unwrap()], vec![], 60, 100, 120, "test", generation))
            })
        }
    }

    #[derive(Clone, Default)]
    struct RecordingBackend { applied: Arc<Mutex<Vec<RouteIntent>>>, removed: Arc<Mutex<Vec<RouteIntent>>> }

    impl RouteBackend for RecordingBackend {
        fn name(&self) -> &str { "recording" }
        fn capabilities(&self) -> super::super::RouteBackendCapabilities { super::super::RouteBackendCapabilities { ipv4: true, ipv6: false, direct: true, proxy: false, auto: false, block: false } }
        fn apply<'a>(&'a self, intents: &'a [RouteIntent]) -> super::super::RouteBackendFuture<'a, Result<usize, super::super::RouteBackendError>> {
            let applied = self.applied.clone();
            Box::pin(async move { applied.lock().unwrap().extend_from_slice(intents); Ok(intents.len()) })
        }
        fn remove<'a>(&'a self, intents: &'a [RouteIntent]) -> super::super::RouteBackendFuture<'a, Result<usize, super::super::RouteBackendError>> {
            let removed = self.removed.clone();
            Box::pin(async move { removed.lock().unwrap().extend_from_slice(intents); Ok(intents.len()) })
        }
    }

    #[tokio::test]
    async fn tick_resolves_and_applies() {
        let resolver = TestResolver::default();
        let calls = resolver.calls.clone();
        let runtime = DomainRuntime::new(DomainPolicy::new(DomainPolicyAction::Direct), resolver);
        let backend = RecordingBackend::default();
        let applied = backend.applied.clone();
        let mut supervisor = DomainSupervisor::new(runtime, backend);
        supervisor.runtime_mut().state_mut().upsert("a.example", DomainPolicyAction::Direct);
        supervisor.runtime_mut().state_mut().upsert("b.example", DomainPolicyAction::Direct);
        let report = supervisor.tick(100).await.unwrap();
        assert_eq!(report.refreshed, vec!["a.example", "b.example"]);
        assert_eq!(report.accepted, 2);
        assert!(report.dns_errors.is_empty());
        assert_eq!(report.reconcile.apply.len(), 1);
        assert_eq!(applied.lock().unwrap().len(), 1);
        assert_eq!(calls.lock().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn transient_dns_failure_keeps_stale_route() {
        struct FailingResolver;
        impl Resolver for FailingResolver {
            fn identity(&self) -> &str { "failing" }
            fn resolve<'a>(&'a self, _: &'a str, _: u64) -> ResolverFuture<'a, Result<DomainRecord, ResolveError>> { Box::pin(async { Err(ResolveError::Timeout) }) }
        }
        let runtime = DomainRuntime::new(DomainPolicy::new(DomainPolicyAction::Direct), FailingResolver);
        let backend = RecordingBackend::default();
        let mut supervisor = DomainSupervisor::new(runtime, backend);
        let entry = supervisor.runtime_mut().state_mut().upsert("a.example", DomainPolicyAction::Direct);
        let generation = entry.begin_resolution();
        assert!(entry.accept_record(DomainRecord::new_with_stale_grace("a.example", vec!["1.2.3.4".parse().unwrap()], vec![], 60, 100, 120, "seed", generation)));
        let report = supervisor.tick(160).await.unwrap();
        assert_eq!(report.dns_errors.len(), 1);
        assert_eq!(report.reconcile.apply.len(), 1);
        assert_eq!(supervisor.runtime().desired_intents(160).len(), 1);
    }
}
