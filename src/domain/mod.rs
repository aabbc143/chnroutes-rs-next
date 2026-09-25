//! Domain routing domain model and policy evaluation.
//!
//! This module intentionally has no dependency on DNS, route tables, VPNs,
//! or proxy implementations. It converts a domain name into a deterministic
//! policy decision only.

pub mod policy;

pub use policy::{
    DomainPolicy, DomainRule, DomainRuleMatcher, DomainRuleMatch, DomainRuleSource,
    DomainPolicyAction, DomainPolicyDecision,
};
