//! Domain routing domain model and policy evaluation.
//!
//! This module intentionally has no dependency on route tables, VPNs, or
//! proxy implementations. Policy evaluation, DNS state, route intent
//! generation, and backend enforcement are kept separate so the later
//! Reconciler can combine them without coupling either side.

pub mod backend;
pub mod intent;
pub mod policy;
pub mod resolver;

pub use backend::{
    RouteBackend, RouteBackendCapabilities, RouteBackendError, RouteBackendFuture,
    SystemRouteBackend,
};
pub use intent::{RouteIntent, RouteIntentAction, RouteIntentOwner};
pub use policy::{
    DomainPolicy, DomainPolicyAction, DomainPolicyDecision, DomainRule, DomainRuleMatch,
    DomainRuleMatcher, DomainRuleSource,
};
pub use resolver::{DomainRecord, ResolveError, Resolver, ResolverFuture};
