use std::future::Future;
use std::net::IpAddr;
use std::pin::Pin;

use super::{RouteIntent, RouteIntentAction};

/// A boxed future used by RouteBackend without requiring async-trait.
pub type RouteBackendFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Backend-level error. The Domain layer does not need to know how the
/// operating system represents routes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteBackendError {
    UnsupportedAction(RouteIntentAction),
    UnsupportedAddressFamily(IpAddr),
    OperationFailed { operation: &'static str, failed: usize },
    Other(String),
}

impl std::fmt::Display for RouteBackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedAction(action) => {
                write!(f, "route backend does not support action: {action:?}")
            }
            Self::UnsupportedAddressFamily(ip) => {
                write!(f, "route backend does not support address family: {ip}")
            }
            Self::OperationFailed { operation, failed } => {
                write!(f, "{operation} route operation failed for {failed} route(s)")
            }
            Self::Other(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for RouteBackendError {}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RouteBackendCapabilities {
    /// Whether the backend can directly manipulate IPv4 routes.
    pub ipv4: bool,
    /// Whether the backend can directly manipulate IPv6 routes.
    pub ipv6: bool,
    /// Whether the backend can represent a direct route.
    pub direct: bool,
    /// Whether the backend can represent proxy routing itself.
    pub proxy: bool,
    /// Whether the backend can decide routing automatically.
    pub auto: bool,
    /// Whether the backend can enforce blocking itself.
    pub block: bool,
}

impl RouteBackendCapabilities {
    pub const SYSTEM_ROUTE: Self = Self {
        ipv4: true,
        ipv6: false,
        direct: true,
        proxy: false,
        auto: false,
        block: false,
    };

    pub fn supports(&self, intent: &RouteIntent) -> bool {
        let family_supported = match intent.destination {
            ipnet::IpNet::V4(_) => self.ipv4,
            ipnet::IpNet::V6(_) => self.ipv6,
        };

        let action_supported = match intent.action {
            RouteIntentAction::Direct => self.direct,
            RouteIntentAction::Proxy => self.proxy,
            RouteIntentAction::Auto => self.auto,
            RouteIntentAction::Block => self.block,
        };

        family_supported && action_supported
    }
}

/// The stable boundary between desired Domain Routing state and an actual
/// network enforcement mechanism.
///
/// A backend receives RouteIntent, not raw domains and not Windows-specific
/// route parameters. This keeps gateway/interface/metric decisions inside the
/// backend.
pub trait RouteBackend: Send + Sync {
    fn name(&self) -> &str;

    fn capabilities(&self) -> RouteBackendCapabilities;

    fn apply<'a>(&'a self, intents: &'a [RouteIntent]) -> RouteBackendFuture<'a, Result<usize, RouteBackendError>>;

    fn remove<'a>(&'a self, intents: &'a [RouteIntent]) -> RouteBackendFuture<'a, Result<usize, RouteBackendError>>;
}

/// Adapter around the existing 0.5.x route engine.
///
/// The current route_op implementation is intentionally left unchanged.
/// This adapter is the first compatibility boundary between the new Domain
/// architecture and the existing Windows route implementation.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemRouteBackend;

impl SystemRouteBackend {
    pub const fn new() -> Self {
        Self
    }

    fn validate_intents(
        &self,
        intents: &[RouteIntent],
    ) -> Result<(), RouteBackendError> {
        let capabilities = self.capabilities();

        for intent in intents {
            if !capabilities.supports(intent) {
                match intent.destination {
                    ipnet::IpNet::V4(ip) if !capabilities.ipv4 => {
                        return Err(RouteBackendError::UnsupportedAddressFamily(
                            ip.addr(),
                        ));
                    }
                    ipnet::IpNet::V6(ip) if !capabilities.ipv6 => {
                        return Err(RouteBackendError::UnsupportedAddressFamily(
                            ip.addr(),
                        ));
                    }
                    _ => return Err(RouteBackendError::UnsupportedAction(intent.action)),
                }
            }
        }

        Ok(())
    }

    fn destinations(intents: &[RouteIntent]) -> Vec<ipnet::IpNet> {
        intents.iter().map(|intent| intent.destination).collect()
    }
}

impl RouteBackend for SystemRouteBackend {
    fn name(&self) -> &str {
        "system-route"
    }

    fn capabilities(&self) -> RouteBackendCapabilities {
        RouteBackendCapabilities::SYSTEM_ROUTE
    }

    fn apply<'a>(
        &'a self,
        intents: &'a [RouteIntent],
    ) -> RouteBackendFuture<'a, Result<usize, RouteBackendError>> {
        Box::pin(async move {
            self.validate_intents(intents)?;

            if intents.is_empty() {
                return Ok(0);
            }

            let routes = Self::destinations(intents);
            let result = crate::route_op::add_routes(&routes)
                .await
                .map_err(|error| RouteBackendError::Other(error.to_string()))?;

            if result.failed > 0 {
                return Err(RouteBackendError::OperationFailed {
                    operation: "add",
                    failed: result.failed,
                });
            }

            Ok(result.added + result.already_exists)
        })
    }

    fn remove<'a>(
        &'a self,
        intents: &'a [RouteIntent],
    ) -> RouteBackendFuture<'a, Result<usize, RouteBackendError>> {
        Box::pin(async move {
            self.validate_intents(intents)?;

            if intents.is_empty() {
                return Ok(0);
            }

            let routes = Self::destinations(intents);
            let result = crate::route_op::del_routes(&routes)
                .await
                .map_err(|error| RouteBackendError::Other(error.to_string()))?;

            if result.failed > 0 {
                return Err(RouteBackendError::OperationFailed {
                    operation: "remove",
                    failed: result.failed,
                });
            }

            Ok(result.removed + result.not_found)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{RouteIntentAction, RouteIntentOwner};

    fn intent(ip: &str, action: RouteIntentAction) -> RouteIntent {
        RouteIntent::from_ip(
            ip.parse().unwrap(),
            action,
            RouteIntentOwner::Domain("example.com".into()),
            1,
            u64::MAX,
        )
    }

    #[test]
    fn system_backend_capabilities_are_explicit() {
        let backend = SystemRouteBackend::new();
        let caps = backend.capabilities();

        assert!(caps.ipv4);
        assert!(!caps.ipv6);
        assert!(caps.direct);
        assert!(!caps.proxy);
        assert!(!caps.auto);
        assert!(!caps.block);
    }

    #[test]
    fn system_backend_accepts_ipv4_direct() {
        let backend = SystemRouteBackend::new();
        assert!(backend.capabilities().supports(&intent(
            "1.2.3.4",
            RouteIntentAction::Direct
        )));
    }

    #[test]
    fn system_backend_rejects_ipv6_until_backend_support_exists() {
        let backend = SystemRouteBackend::new();
        let result = backend.validate_intents(&[intent(
            "2001:db8::1",
            RouteIntentAction::Direct,
        )]);

        assert_eq!(
            result,
            Err(RouteBackendError::UnsupportedAddressFamily(
                "2001:db8::1".parse().unwrap()
            ))
        );
    }

    #[test]
    fn system_backend_rejects_proxy_action() {
        let backend = SystemRouteBackend::new();
        let result = backend.validate_intents(&[intent(
            "1.2.3.4",
            RouteIntentAction::Proxy,
        )]);

        assert_eq!(
            result,
            Err(RouteBackendError::UnsupportedAction(
                RouteIntentAction::Proxy
            ))
        );
    }
}
