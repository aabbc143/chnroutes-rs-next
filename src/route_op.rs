use std::{
    net::{IpAddr, Ipv4Addr},
    sync::OnceLock,
};

#[cfg(target_os = "windows")]
use std::process::Command;

use futures_util::{stream::FuturesOrdered, StreamExt};
use ipnet::IpNet;
use log::info;
use net_route::{Handle, Route};
use tokio::sync::OnceCell;

use crate::error::RouteOpError;

type Result<T> = std::result::Result<T, RouteOpError>;

pub static GATEWAY: OnceCell<Option<Ipv4Addr>> = OnceCell::const_new();
pub static INTERFACE_INDEX: OnceLock<u32> = OnceLock::new();

/// Get the default physical IPv4 interface index.
///
/// On Windows, the VPN interface must not be selected as the default
/// interface for CN routes. Therefore OpenVPN, WSL, vEthernet and
/// loopback interfaces are explicitly excluded.
///
/// On non-Windows platforms, netdev is used to detect the default
/// interface.
#[cfg(target_os = "windows")]
fn get_interface_index() -> std::result::Result<u32, String> {
    let script = r#"
$routes = Get-NetRoute `
    -AddressFamily IPv4 `
    -DestinationPrefix '0.0.0.0/0' `
    -ErrorAction Stop |
    Where-Object {
        $_.NextHop -ne '0.0.0.0' -and
        $_.InterfaceAlias -notmatch 'OpenVPN|WSL|vEthernet|Loopback'
    } |
    Sort-Object RouteMetric, InterfaceMetric

if ($null -eq $routes) {
    throw 'No suitable IPv4 default route found'
}

$routes |
    Select-Object -First 1 -ExpandProperty ifIndex
"#;

    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .output()
        .map_err(|e| format!("Failed to execute PowerShell: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Failed to detect default interface: {}",
            stderr.trim()
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let index = stdout
        .trim()
        .parse::<u32>()
        .map_err(|e| format!("Invalid interface index returned by PowerShell: {e}"))?;

    Ok(index)
}

/// Get the default interface index on non-Windows platforms.
#[cfg(not(target_os = "windows"))]
fn get_interface_index() -> std::result::Result<u32, String> {
    netdev::get_default_interface()
        .map(|interface| interface.index)
        .map_err(|e| e.to_string())
}

/// Get the cached default interface index.
///
/// The interface is detected only once, because all routes in one
/// operation should use the same physical interface.
fn interface_index() -> std::result::Result<u32, String> {
    INTERFACE_INDEX.get_or_init(|| get_interface_index().unwrap_or(0));

    match INTERFACE_INDEX.get() {
        Some(0) => Err("Default Interface not found".to_string()),
        Some(index) => Ok(*index),
        None => Err("Default Interface not found".to_string()),
    }
}

/// Get the default IPv4 gateway.
pub async fn get_gateway(handle: &Handle) -> Result<Option<Ipv4Addr>> {
    let routes = handle
        .list()
        .await?
        .into_iter()
        .filter(|route| route.gateway.is_some())
        .filter(|route| route.destination.is_unspecified())
        .filter_map(|route| route.gateway);

    for gateway in routes {
        if let IpAddr::V4(ipv4) = gateway {
            return Ok(Some(ipv4));
        }
    }

    Ok(None)
}

/// Add one IPv4 route entry.
pub async fn add_route(handle: &Handle, route: &IpNet) -> Result<()> {
    // chnroutes is an IPv4 split-routing tool.
    // IPv6 routes are intentionally ignored.
    if !matches!(route, IpNet::V4(_)) {
        return Ok(());
    }

    let gateway = GATEWAY
        .get_or_try_init(|| async { get_gateway(handle).await })
        .await?
        .ok_or(RouteOpError::NoGatewayError)?;

    let ifindex = interface_index().map_err(RouteOpError::GetInterfaceError)?;

    let route_item = Route::new(route.addr(), route.prefix_len())
        .with_gateway(IpAddr::V4(gateway))
        .with_ifindex(ifindex);

    match handle.add(&route_item).await {
        Ok(_) => Ok(()),

        Err(err)
            if err.kind() == std::io::ErrorKind::Other && err.to_string().contains("exists") =>
        {
            Err(RouteOpError::RouteAlreadyExistsError)
        }

        Err(err) => Err(err.into()),
    }
}

/// Delete one IPv4 route entry.
pub async fn del_route(handle: &Handle, route: &IpNet) -> Result<()> {
    // IPv6 routes are intentionally ignored.
    if !matches!(route, IpNet::V4(_)) {
        return Ok(());
    }

    let ifindex = interface_index().map_err(RouteOpError::GetInterfaceError)?;

    let route_item = Route::new(route.addr(), route.prefix_len()).with_ifindex(ifindex);

    match handle.delete(&route_item).await {
        Ok(_) => Ok(()),

        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            Err(RouteOpError::RouteNotFoundError)
        }

        Err(err) => Err(err.into()),
    }
}

/// Add multiple IPv4 routes.
pub async fn add_routes(routes: &[IpNet]) -> Result<()> {
    let routes: Vec<IpNet> = routes
        .iter()
        .filter(|route| matches!(route, IpNet::V4(_)))
        .cloned()
        .collect();

    info!("Adding {} IPv4 routes...", routes.len());

    if routes.is_empty() {
        info!("No IPv4 routes to add.");
        return Ok(());
    }

    // Detect and cache the interface before adding routes.
    //
    // This ensures all routes in one operation use the same
    // physical network interface.
    let ifindex = interface_index().map_err(RouteOpError::GetInterfaceError)?;

    info!("Using interface index {} for IPv4 routes.", ifindex);

    let handle = Box::leak(Box::new(
        Handle::new().map_err(|_| RouteOpError::HandleInitError)?,
    ));

    let mut futures = routes
        .iter()
        .map(|route| add_route(handle, route))
        .collect::<FuturesOrdered<_>>();

    let mut added = 0;
    let mut existed = 0;
    let mut failed = 0;

    while let Some(result) = futures.next().await {
        match result {
            Ok(()) => {
                added += 1;
            }

            Err(RouteOpError::RouteAlreadyExistsError) => {
                existed += 1;
            }

            Err(_) => {
                failed += 1;
            }
        }
    }

    info!(
        "Routes completed: added={}, already_exists={}, failed={}",
        added, existed, failed
    );

    Ok(())
}

/// Delete multiple IPv4 routes.
pub async fn del_routes(routes: &[IpNet]) -> Result<()> {
    let routes: Vec<IpNet> = routes
        .iter()
        .filter(|route| matches!(route, IpNet::V4(_)))
        .cloned()
        .collect();

    info!("Removing {} IPv4 routes...", routes.len());

    if routes.is_empty() {
        info!("No IPv4 routes to remove.");
        return Ok(());
    }

    let ifindex = interface_index().map_err(RouteOpError::GetInterfaceError)?;

    info!("Using interface index {} for IPv4 routes.", ifindex);

    let handle = Handle::new().map_err(|_| RouteOpError::HandleInitError)?;

    let mut futures = routes
        .iter()
        .map(|route| del_route(&handle, route))
        .collect::<FuturesOrdered<_>>();

    let mut removed = 0;
    let mut missing = 0;
    let mut failed = 0;

    while let Some(result) = futures.next().await {
        match result {
            Ok(()) => {
                removed += 1;
            }

            Err(RouteOpError::RouteNotFoundError) => {
                missing += 1;
            }

            Err(_) => {
                failed += 1;
            }
        }
    }

    info!(
        "Routes removed: removed={}, not_found={}, failed={}",
        removed, missing, failed
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_gateway() {
        let handle = Handle::new().unwrap();

        let result = get_gateway(&handle).await;

        assert!(result.is_ok());

        dbg!(result.unwrap());
    }

    #[tokio::test]
    #[ignore = "Run as Administrator"]
    async fn test_add_remove_route_v4() {
        let handle = Handle::new().unwrap();

        let destination = IpAddr::V4(Ipv4Addr::new(123, 123, 123, 123));

        let route = IpNet::new(destination, 32).unwrap();

        let _ = del_route(&handle, &route).await;

        add_route(&handle, &route).await.unwrap();

        del_route(&handle, &route).await.unwrap();
    }
}
