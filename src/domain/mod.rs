//! Domain routing domain model and policy evaluation.
//!
//! This module intentionally has no dependency on route tables, VPNs, or
//! proxy implementations. Policy evaluation, DNS state, route intent
//! generation, backend enforcement, reconciliation, and runtime state are
//! kept separate.

pub mod backend;
pub mod intent;
pub mod policy;
pub mod reconcile;
pub mod resolver;
pub mod runtime;
pub mod state;

pub use backend::{
    RouteBackend, RouteBackendCapabilities, RouteBackendError, RouteBackendFuture,
    SystemRouteBackend,
};
pub use intent::{RouteIntent, RouteIntentAction, RouteIntentOwner};
pub use policy::{
    DomainPolicy, DomainPolicyAction, DomainPolicyDecision, DomainRule, DomainRuleMatch,
    DomainRuleMatcher, DomainRuleSource,
};
pub use reconcile::{ReconcileError, ReconcilePlan, Reconciler};
pub use resolver::{DomainRecord, ResolveError, Resolver, ResolverFuture};
pub use runtime::{DomainRuntime, ResolveOutcome};
pub use state::{DomainState, DomainStateEntry, ResolveStateError};
