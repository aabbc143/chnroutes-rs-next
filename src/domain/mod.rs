//! Domain routing domain model and policy evaluation.
//!
//! This module intentionally has no dependency on route tables, VPNs, or
//! proxy implementations. Policy evaluation and DNS state are kept separate
//! so the later Reconciler can combine them without coupling either side.

pub mod policy;
pub mod resolver;

pub use policy::{
    DomainPolicy, DomainPolicyAction, DomainPolicyDecision, DomainRule, DomainRuleMatch,
    DomainRuleMatcher, DomainRuleSource,
};
pub use resolver::{DomainRecord, ResolveError, Resolver, ResolverFuture};
