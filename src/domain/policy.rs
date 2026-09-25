use serde::{Deserialize, Serialize};

/// The action requested by a domain policy.
///
/// These actions are deliberately backend-agnostic. A later TrafficBackend or
/// RouteBackend decides how an action is actually enforced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DomainPolicyAction {
    Direct,
    Proxy,
    Auto,
    Block,
    /// Keep the result produced by a lower-priority/default policy unchanged.
    NoOverride,
}

/// The matcher used by a domain rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DomainRuleMatcher {
    Exact(String),
    Suffix(String),
    Keyword(String),
}

impl DomainRuleMatcher {
    fn normalized_value(&self) -> String {
        let value = match self {
            Self::Exact(value) | Self::Suffix(value) | Self::Keyword(value) => value,
        };
        normalize_domain(value)
    }

    fn matches(&self, domain: &str) -> bool {
        let domain = normalize_domain(domain);
        let value = self.normalized_value();

        if value.is_empty() || domain.is_empty() {
            return false;
        }

        match self {
            Self::Exact(_) => domain == value,
            Self::Suffix(_) => domain == value || domain.ends_with(&format!(".{value}")),
            Self::Keyword(_) => domain.contains(&value),
        }
    }

    fn specificity(&self) -> u8 {
        match self {
            Self::Exact(_) => 3,
            Self::Suffix(_) => 2,
            Self::Keyword(_) => 1,
        }
    }
}

/// Identifies where a rule came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DomainRuleSource {
    User,
    BuiltIn,
    RuleSet(String),
}

/// A single deterministic domain policy rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainRule {
    /// Stable rule identifier used as the final deterministic tie-breaker.
    pub id: String,
    pub matcher: DomainRuleMatcher,
    pub action: DomainPolicyAction,
    /// Larger values are evaluated first.
    pub priority: i32,
    pub enabled: bool,
    pub source: DomainRuleSource,
}

impl DomainRule {
    pub fn new(
        id: impl Into<String>,
        matcher: DomainRuleMatcher,
        action: DomainPolicyAction,
        priority: i32,
        source: DomainRuleSource,
    ) -> Self {
        Self {
            id: id.into(),
            matcher,
            action,
            priority,
            enabled: true,
            source,
        }
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    pub fn matches(&self, domain: &str) -> bool {
        self.enabled && self.matcher.matches(domain)
    }
}

/// A matched rule and the reason it won.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainRuleMatch<'a> {
    pub rule: &'a DomainRule,
    pub normalized_domain: String,
}

/// The policy-engine result. No network operation happens here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainPolicyDecision<'a> {
    pub action: DomainPolicyAction,
    pub matched_rule: Option<DomainRuleMatch<'a>>,
    pub reason: String,
}

/// Collection of domain rules with deterministic evaluation order.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DomainPolicy {
    rules: Vec<DomainRule>,
    default_action: DomainPolicyAction,
}

impl DomainPolicy {
    pub fn new(default_action: DomainPolicyAction) -> Self {
        Self {
            rules: Vec::new(),
            default_action,
        }
    }

    pub fn rules(&self) -> &[DomainRule] {
        &self.rules
    }

    pub fn default_action(&self) -> DomainPolicyAction {
        self.default_action
    }

    pub fn add_rule(&mut self, rule: DomainRule) {
        self.rules.push(rule);
    }

    pub fn evaluate(&self, domain: &str) -> DomainPolicyDecision<'_> {
        let normalized_domain = normalize_domain(domain);

        let matched = self
            .rules
            .iter()
            .filter(|rule| rule.matches(&normalized_domain))
            .max_by(|left, right| {
                left.priority
                    .cmp(&right.priority)
                    // For equal priority, prefer a more specific matcher.
                    .then_with(|| left.matcher.specificity().cmp(&right.matcher.specificity()))
                    // Finally make the result independent of Vec insertion order.
                    .then_with(|| right.id.cmp(&left.id))
            });

        match matched {
            Some(rule) if rule.action == DomainPolicyAction::NoOverride => DomainPolicyDecision {
                action: self.default_action,
                matched_rule: Some(DomainRuleMatch {
                    rule,
                    normalized_domain,
                }),
                reason: format!(
                    "rule '{}' matched with NoOverride; using default action",
                    rule.id
                ),
            },
            Some(rule) => DomainPolicyDecision {
                action: rule.action,
                matched_rule: Some(DomainRuleMatch {
                    rule,
                    normalized_domain,
                }),
                reason: format!("rule '{}' matched", rule.id),
            },
            None => DomainPolicyDecision {
                action: self.default_action,
                matched_rule: None,
                reason: "no rule matched; using default action".to_string(),
            },
        }
    }
}

/// Normalize a DNS-style domain name for policy matching.
///
/// This deliberately does not perform IDNA conversion yet. That belongs to a
/// later resolver/rule-source layer once the Unicode/ASCII boundary is defined.
pub fn normalize_domain(domain: &str) -> String {
    domain.trim().trim_end_matches('.').to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(
        id: &str,
        matcher: DomainRuleMatcher,
        action: DomainPolicyAction,
        priority: i32,
    ) -> DomainRule {
        DomainRule::new(id, matcher, action, priority, DomainRuleSource::User)
    }

    #[test]
    fn normalizes_domain_case_and_trailing_dot() {
        assert_eq!(normalize_domain("  WWW.Example.COM. "), "www.example.com");
    }

    #[test]
    fn exact_match_is_case_insensitive() {
        let matcher = DomainRuleMatcher::Exact("Example.COM".into());
        assert!(matcher.matches("example.com."));
        assert!(!matcher.matches("www.example.com"));
    }

    #[test]
    fn suffix_match_respects_dns_label_boundary() {
        let matcher = DomainRuleMatcher::Suffix("example.com".into());
        assert!(matcher.matches("example.com"));
        assert!(matcher.matches("www.example.com"));
        assert!(!matcher.matches("notexample.com"));
    }

    #[test]
    fn keyword_match_is_substring_match() {
        let matcher = DomainRuleMatcher::Keyword("google".into());
        assert!(matcher.matches("www.google.com"));
        assert!(matcher.matches("googlevideo.example"));
        assert!(!matcher.matches("example.com"));
    }

    #[test]
    fn disabled_rule_is_ignored() {
        let rule = rule(
            "blocked",
            DomainRuleMatcher::Exact("example.com".into()),
            DomainPolicyAction::Block,
            100,
        )
        .disabled();
        assert!(!rule.matches("example.com"));
    }

    #[test]
    fn higher_priority_wins() {
        let mut policy = DomainPolicy::new(DomainPolicyAction::Direct);
        policy.add_rule(rule(
            "low",
            DomainRuleMatcher::Suffix("example.com".into()),
            DomainPolicyAction::Proxy,
            10,
        ));
        policy.add_rule(rule(
            "high",
            DomainRuleMatcher::Exact("www.example.com".into()),
            DomainPolicyAction::Direct,
            20,
        ));

        let decision = policy.evaluate("WWW.EXAMPLE.COM.");
        assert_eq!(decision.action, DomainPolicyAction::Direct);
        assert_eq!(decision.matched_rule.unwrap().rule.id, "high");
    }

    #[test]
    fn specificity_breaks_priority_ties() {
        let mut policy = DomainPolicy::new(DomainPolicyAction::Direct);
        policy.add_rule(rule(
            "keyword",
            DomainRuleMatcher::Keyword("example".into()),
            DomainPolicyAction::Block,
            10,
        ));
        policy.add_rule(rule(
            "suffix",
            DomainRuleMatcher::Suffix("example.com".into()),
            DomainPolicyAction::Proxy,
            10,
        ));

        let decision = policy.evaluate("www.example.com");
        assert_eq!(decision.action, DomainPolicyAction::Proxy);
        assert_eq!(decision.matched_rule.unwrap().rule.id, "suffix");
    }

    #[test]
    fn rule_id_breaks_complete_ties() {
        let mut policy = DomainPolicy::new(DomainPolicyAction::Direct);
        policy.add_rule(rule(
            "z-rule",
            DomainRuleMatcher::Suffix("example.com".into()),
            DomainPolicyAction::Block,
            10,
        ));
        policy.add_rule(rule(
            "a-rule",
            DomainRuleMatcher::Suffix("example.com".into()),
            DomainPolicyAction::Proxy,
            10,
        ));

        let decision = policy.evaluate("example.com");
        assert_eq!(decision.action, DomainPolicyAction::Block);
        assert_eq!(decision.matched_rule.unwrap().rule.id, "z-rule");
    }

    #[test]
    fn no_override_falls_back_to_default() {
        let mut policy = DomainPolicy::new(DomainPolicyAction::Proxy);
        policy.add_rule(rule(
            "keep-default",
            DomainRuleMatcher::Exact("example.com".into()),
            DomainPolicyAction::NoOverride,
            100,
        ));

        let decision = policy.evaluate("example.com");
        assert_eq!(decision.action, DomainPolicyAction::Proxy);
        assert!(decision.reason.contains("NoOverride"));
    }

    #[test]
    fn no_match_uses_default() {
        let policy = DomainPolicy::new(DomainPolicyAction::Direct);
        let decision = policy.evaluate("example.com");
        assert_eq!(decision.action, DomainPolicyAction::Direct);
        assert!(decision.matched_rule.is_none());
    }
}
