use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;

use ipnet::IpNet;
use log::{debug, info};
use net_route::{Handle, Route};
use tokio::sync::OnceCell;

use crate::error::RouteOpError;

type Result<T> = std::result::Result<T, RouteOpError>;

pub static GATEWAY: OnceCell<Option<Ipv4Addr>> = OnceCell::const_new();

/// 包含最优出口网卡与网关的完整结构体
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BestRouteInfo {
    pub interface_index: u32,
    pub next_hop: Ipv4Addr,
}

/// A route identity used for in-memory diffing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct RouteKey {
    destination: Ipv4Addr,
    prefix: u8,
    ifindex: u32,
    gateway: Ipv4Addr,
}

#[derive(Debug, Default)]
pub struct RouteResult {
    pub added: usize,
    pub already_exists: usize,
    pub failed: usize,
    pub success_routes: Vec<IpNet>,
}

#[derive(Debug, Default)]
pub struct RemoveResult {
    pub removed: usize,
    pub not_found: usize,
    pub failed: usize,
}

/// Direct Win32 GetBestRoute2 query with Network Byte Order handling.
#[cfg(target_os = "windows")]
pub fn get_best_route_info(target_ip: Ipv4Addr) -> std::result::Result<BestRouteInfo, String> {
    use windows_sys::Win32::NetworkManagement::IpHelper::GetBestRoute2;
    use windows_sys::Win32::Networking::WinSock::{AF_INET, SOCKADDR_INET};

    unsafe {
        let mut destination: SOCKADDR_INET = std::mem::zeroed();
        destination.Ipv4.sin_family = AF_INET;

        // Winsock sin_addr 要求 Network Byte Order (Big-Endian)
        destination.Ipv4.sin_addr.S_un.S_addr = u32::from(target_ip).to_be();

        let mut best_route = std::mem::zeroed();
        let mut best_source_address = std::mem::zeroed();

        let ret = GetBestRoute2(
            std::ptr::null(),
            0,
            std::ptr::null(),
            &destination,
            0,
            &mut best_route,
            &mut best_source_address,
        );

        if ret != 0 {
            return Err(format!("GetBestRoute2 failed with error code: {ret}"));
        }

        let ifindex = best_route.InterfaceIndex;
        if ifindex == 0 {
            return Err("Invalid interface index returned by system".to_string());
        }

        let raw_next_hop_net_order = best_route.NextHop.Ipv4.sin_addr.S_un.S_addr;
        let next_hop = Ipv4Addr::from(u32::from_be(raw_next_hop_net_order));

        Ok(BestRouteInfo {
            interface_index: ifindex,
            next_hop,
        })
    }
}

/// 纯异步非阻塞网络栈就绪等待函数，支持 SCM 心跳回调刷新
pub async fn wait_for_network_ready<F>(
    target_ip: Ipv4Addr,
    timeout: Duration,
    mut heartbeat_cb: F,
) -> std::result::Result<BestRouteInfo, String>
where
    F: FnMut(),
{
    info!(
        "Waiting for network interface readiness (probe target: {})...",
        target_ip
    );

    let start = tokio::time::Instant::now();
    let poll_interval = Duration::from_millis(500);
    let mut last_heartbeat = tokio::time::Instant::now();

    while start.elapsed() < timeout {
        if last_heartbeat.elapsed() >= Duration::from_secs(1) {
            heartbeat_cb();
            last_heartbeat = tokio::time::Instant::now();
        }

        #[cfg(target_os = "windows")]
        if let Ok(info) = get_best_route_info(target_ip) {
            if info.interface_index > 0 {
                info!(
                    "Network ready (gateway: {}, ifindex: {})",
                    info.next_hop, info.interface_index
                );
                return Ok(info);
            }
        }

        #[cfg(not(target_os = "windows"))]
        if let Ok(ifindex) = interface_index() {
            if ifindex != 0 {
                return Ok(BestRouteInfo {
                    interface_index: ifindex,
                    next_hop: Ipv4Addr::UNSPECIFIED,
                });
            }
        }

        tokio::time::sleep(poll_interval).await;
    }

    Err(format!(
        "Network not ready within {} seconds timeout",
        timeout.as_secs()
    ))
}

/// Get default physical IPv4 interface index.
#[cfg(target_os = "windows")]
fn get_interface_index() -> std::result::Result<u32, String> {
    if let Ok(info) = get_best_route_info(Ipv4Addr::new(223, 5, 5, 5)) {
        return Ok(info.interface_index);
    }

    netdev::get_default_interface()
        .map(|interface| interface.index)
        .map_err(|e| e.to_string())
}

#[cfg(not(target_os = "windows"))]
fn get_interface_index() -> std::result::Result<u32, String> {
    netdev::get_default_interface()
        .map(|interface| interface.index)
        .map_err(|e| e.to_string())
}

pub fn interface_index() -> std::result::Result<u32, String> {
    let index = get_interface_index()?;

    if index == 0 {
        return Err("Default Interface not found".to_string());
    }

    Ok(index)
}

fn find_gateway_in_routes(routes: &[Route]) -> Option<Ipv4Addr> {
    routes
        .iter()
        .filter(|route| route.gateway.is_some())
        .filter(|route| route.destination.is_unspecified())
        .find_map(|route| match route.gateway {
            Some(IpAddr::V4(ipv4)) if !ipv4.is_unspecified() => Some(ipv4),
            _ => None,
        })
}

pub async fn get_gateway(handle: &Handle) -> Result<Option<Ipv4Addr>> {
    #[cfg(target_os = "windows")]
    if let Ok(info) = get_best_route_info(Ipv4Addr::new(223, 5, 5, 5)) {
        if !info.next_hop.is_unspecified() {
            return Ok(Some(info.next_hop));
        }
    }

    let routes = handle.list().await.map_err(RouteOpError::OpError)?;
    Ok(find_gateway_in_routes(&routes))
}

fn resolve_gateway(system_routes: &[Route]) -> Result<Ipv4Addr> {
    #[cfg(target_os = "windows")]
    if let Ok(info) = get_best_route_info(Ipv4Addr::new(223, 5, 5, 5)) {
        if !info.next_hop.is_unspecified() {
            return Ok(info.next_hop);
        }
    }

    find_gateway_in_routes(system_routes).ok_or(RouteOpError::NoGatewayError)
}

fn route_destination_key(route: &IpNet) -> Option<(Ipv4Addr, u8)> {
    match route {
        IpNet::V4(route) => Some((route.network(), route.prefix_len())),
        IpNet::V6(_) => None,
    }
}

fn route_key(route: &IpNet, ifindex: u32, gateway: Ipv4Addr) -> Option<RouteKey> {
    let (destination, prefix) = route_destination_key(route)?;

    Some(RouteKey {
        destination,
        prefix,
        ifindex,
        gateway,
    })
}

fn parse_existing_ipv4_routes(system_routes: &[Route]) -> HashSet<RouteKey> {
    let mut existing = HashSet::with_capacity(system_routes.len());

    for route in system_routes {
        let IpAddr::V4(destination) = route.destination else {
            continue;
        };

        let Some(ifindex) = route.ifindex else {
            continue;
        };

        let Some(IpAddr::V4(gateway)) = route.gateway else {
            continue;
        };

        existing.insert(RouteKey {
            destination,
            prefix: route.prefix,
            ifindex,
            gateway,
        });
    }

    existing
}

/// 批量添加路由表项
pub async fn add_routes(routes: &[IpNet]) -> Result<RouteResult> {
    let handle = Handle::new().map_err(RouteOpError::OpError)?;
    let ifindex = interface_index().map_err(RouteOpError::GetInterfaceError)?;
    let system_routes = handle.list().await.map_err(RouteOpError::OpError)?;
    let gateway = resolve_gateway(&system_routes)?;
    let existing = parse_existing_ipv4_routes(&system_routes);

    let mut result = RouteResult::default();

    for route in routes {
        let Some(key) = route_key(route, ifindex, gateway) else {
            result.failed += 1;
            continue;
        };

        if existing.contains(&key) {
            result.already_exists += 1;
            result.success_routes.push(*route);
            continue;
        }

        let nr_route = Route::new(IpAddr::V4(key.destination), key.prefix)
            .with_ifindex(ifindex)
            .with_gateway(IpAddr::V4(gateway));

        match handle.add(&nr_route).await {
            Ok(_) => {
                result.added += 1;
                result.success_routes.push(*route);
            }
            Err(e) => {
                debug!("Failed to add route {}: {}", route, e);
                result.failed += 1;
            }
        }
    }

    Ok(result)
}

/// 批量删除路由表项
pub async fn del_routes(routes: &[IpNet]) -> Result<RemoveResult> {
    let handle = Handle::new().map_err(RouteOpError::OpError)?;
    let ifindex = interface_index().ok();
    let gateway = get_gateway(&handle).await.ok().flatten();

    let mut result = RemoveResult::default();

    for route in routes {
        let (destination, prefix) = match route {
            IpNet::V4(v4) => (v4.network(), v4.prefix_len()),
            _ => {
                result.failed += 1;
                continue;
            }
        };

        let mut nr_route = Route::new(IpAddr::V4(destination), prefix);
        if let Some(idx) = ifindex {
            nr_route = nr_route.with_ifindex(idx);
        }
        if let Some(gw) = gateway {
            nr_route = nr_route.with_gateway(IpAddr::V4(gw));
        }

        match handle.delete(&nr_route).await {
            Ok(_) => result.removed += 1,
            Err(_) => result.not_found += 1,
        }
    }

    Ok(result)
}
