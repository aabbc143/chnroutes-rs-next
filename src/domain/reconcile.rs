use std::collections::{HashMap, HashSet};
use ipnet::IpNet;

use super::{RouteBackend, RouteBackendError, RouteIntent, RouteIntentAction, RouteIntentOwner};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReconcilePlan {
    pub remove: Vec<RouteIntent>,
    pub apply: Vec<RouteIntent>,
}

impl ReconcilePlan {
    pub fn is_empty(&self) -> bool {
        self.remove.is_empty() && self.apply.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconcileError {
    ConflictingActions {
        destination: IpNet,
        first: RouteIntentAction,
        second: RouteIntentAction,
    },
    Backend(RouteBackendError),
}

impl std::fmt::Display for ReconcileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConflictingActions { destination, first, second } => {
                write!(f, "conflicting route actions for {destination}: {first:?} vs {second:?}")
            }
            Self::Backend(error) => write!(f, "route backend error: {error}"),
        }
    }
}

impl std::error::Error for ReconcileError {}

impl From<RouteBackendError> for ReconcileError {
    fn from(error: RouteBackendError) -> Self {
        Self::Backend(error)
    }
}

#[derive(Debug)]
pub struct Reconciler<B> {
    backend: B,
    applied: Vec<RouteIntent>,
}

impl<B: RouteBackend> Reconciler<B> {
    pub fn new(backend: B) -> Self {
        Self { backend, applied: Vec::new() }
    }

    pub fn backend(&self) -> &B {
        &self.backend
    }

    pub fn applied(&self) -> &[RouteIntent] {
        &self.applied
    }

    pub fn plan(
        &self,
        desired: &[RouteIntent],
        now: u64,
    ) -> Result<ReconcilePlan, ReconcileError> {
        build_plan(desired, &self.applied, now)
    }

    pub async fn reconcile(
        &mut self,
        desired: &[RouteIntent],
        now: u64,
    ) -> Result<ReconcilePlan, ReconcileError> {
        let plan = self.plan(desired, now)?;

        if !plan.remove.is_empty() {
            self.backend.remove(&plan.remove).await?;
            remove_applied_keys(&mut self.applied, &plan.remove);
        }

        if !plan.apply.is_empty() {
            self.backend.apply(&plan.apply).await?;
        }

        self.applied = effective_intents(desired, now)?;
        Ok(plan)
    }
}

fn effective_intents(
    desired: &[RouteIntent],
    now: u64,
) -> Result<Vec<RouteIntent>, ReconcileError> {
    let mut by_destination: HashMap<IpNet, HashMap<RouteIntentAction, RouteIntent>> =
        HashMap::new();

    for intent in desired.iter().filter(|intent| !intent.is_expired_at(now)) {
        let actions = by_destination.entry(intent.destination).or_default();

        if !actions.is_empty() && !actions.contains_key(&intent.action) {
            let first = *actions.keys().next().expect("non-empty action map");
            return Err(ReconcileError::ConflictingActions {
                destination: intent.destination,
                first,
                second: intent.action,
            });
        }

        match actions.get_mut(&intent.action) {
            Some(existing) => {
                existing.generation = existing.generation.max(intent.generation);
                existing.expires_at = existing.expires_at.max(intent.expires_at);
                if owner_key(&intent.owner) < owner_key(&existing.owner) {
                    existing.owner = intent.owner.clone();
                }
            }
            None => {
                actions.insert(intent.action, intent.clone());
            }
        }
    }

    let mut result = Vec::new();
    for actions in by_destination.into_values() {
        result.extend(actions.into_values());
    }

    result.sort_by(|left, right| {
        left.destination
            .to_string()
            .cmp(&right.destination.to_string())
            .then_with(|| action_key(left.action).cmp(action_key(right.action)))
    });

    Ok(result)
}

fn build_plan(
    desired: &[RouteIntent],
    applied: &[RouteIntent],
    now: u64,
) -> Result<ReconcilePlan, ReconcileError> {
    let desired = effective_intents(desired, now)?;
    let applied = effective_intents(applied, now)?;

    let desired_keys: HashSet<_> = desired.iter().map(intent_key).collect();
    let applied_keys: HashSet<_> = applied.iter().map(intent_key).collect();

    let remove = applied
        .into_iter()
        .filter(|intent| !desired_keys.contains(&intent_key(intent)))
        .collect();

    let apply = desired
        .into_iter()
        .filter(|intent| !applied_keys.contains(&intent_key(intent)))
        .collect();

    Ok(ReconcilePlan { remove, apply })
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct IntentKey {
    destination: IpNet,
    action: RouteIntentAction,
}

fn intent_key(intent: &RouteIntent) -> IntentKey {
    IntentKey {
        destination: intent.destination,
        action: intent.action,
    }
}

fn action_key(action: RouteIntentAction) -> &'static str {
    match action {
        RouteIntentAction::Direct => "direct",
        RouteIntentAction::Proxy => "proxy",
        RouteIntentAction::Auto => "auto",
        RouteIntentAction::Block => "block",
    }
}

fn owner_key(owner: &RouteIntentOwner) -> String {
    match owner {
        RouteIntentOwner::Domain(domain) => domain.clone(),
    }
}

fn remove_applied_keys(applied: &mut Vec<RouteIntent>, removed: &[RouteIntent]) {
    let removed_keys: HashSet<_> = removed.iter().map(intent_key).collect();
    applied.retain(|intent| !removed_keys.contains(&intent_key(intent)));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intent(
        ip: &str,
        action: RouteIntentAction,
        owner: &str,
        generation: u64,
        expires_at: u64,
    ) -> RouteIntent {
        RouteIntent::from_ip(
            ip.parse().unwrap(),
            action,
            RouteIntentOwner::Domain(owner.into()),
            generation,
            expires_at,
        )
    }

    #[test]
    fn empty_state_applies_desired_routes() {
        let plan = build_plan(
            &[intent("1.2.3.4", RouteIntentAction::Direct, "a.example", 1, 100)],
            &[],
            50,
        ).unwrap();
        assert!(plan.remove.is_empty());
        assert_eq!(plan.apply.len(), 1);
    }

    #[test]
    fn unchanged_route_produces_empty_plan() {
        let desired = vec![intent("1.2.3.4", RouteIntentAction::Direct, "a.example", 2, 200)];
        let applied = vec![intent("1.2.3.4", RouteIntentAction::Direct, "a.example", 1, 100)];
        let plan = build_plan(&desired, &applied, 50).unwrap();
        assert!(plan.is_empty());
    }

    #[test]
    fn expired_desired_intent_is_not_applied() {
        let plan = build_plan(
            &[intent("1.2.3.4", RouteIntentAction::Direct, "a.example", 1, 100)],
            &[],
            100,
        ).unwrap();
        assert!(plan.is_empty());
    }

    #[test]
    fn expired_applied_intent_is_removed() {
        let plan = build_plan(
            &[],
            &[intent("1.2.3.4", RouteIntentAction::Direct, "a.example", 1, 100)],
            100,
        ).unwrap();
        assert_eq!(plan.remove.len(), 1);
    }

    #[test]
    fn two_domains_sharing_ip_are_one_backend_route() {
        let desired = vec![
            intent("1.2.3.4", RouteIntentAction::Direct, "a.example", 1, 100),
            intent("1.2.3.4", RouteIntentAction::Direct, "b.example", 2, 200),
        ];
        let effective = effective_intents(&desired, 50).unwrap();
        assert_eq!(effective.len(), 1);
        assert_eq!(effective[0].generation, 2);
        assert_eq!(effective[0].expires_at, 200);
    }

    #[test]
    fn conflicting_actions_are_rejected() {
        let desired = vec![
            intent("1.2.3.4", RouteIntentAction::Direct, "a.example", 1, 100),
            intent("1.2.3.4", RouteIntentAction::Proxy, "b.example", 1, 100),
        ];
        assert!(matches!(
            effective_intents(&desired, 50),
            Err(ReconcileError::ConflictingActions { .. })
        ));
    }

    #[test]
    fn different_destinations_are_independent() {
        let desired = vec![
            intent("1.2.3.4", RouteIntentAction::Direct, "a.example", 1, 100),
            intent("5.6.7.8", RouteIntentAction::Direct, "b.example", 1, 100),
        ];
        let plan = build_plan(&desired, &[], 50).unwrap();
        assert_eq!(plan.apply.len(), 2);
    }

    #[test]
    fn remove_and_apply_are_calculated_as_sets() {
        let desired = vec![intent("5.6.7.8", RouteIntentAction::Direct, "new.example", 2, 200)];
        let applied = vec![intent("1.2.3.4", RouteIntentAction::Direct, "old.example", 1, 100)];
        let plan = build_plan(&desired, &applied, 50).unwrap();
        assert_eq!(plan.remove.len(), 1);
        assert_eq!(plan.apply.len(), 1);
    }
}
