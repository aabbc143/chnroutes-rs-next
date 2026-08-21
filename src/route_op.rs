use std::{
    net::{IpAddr, Ipv4Addr},
    sync::OnceLock,
};

use futures_util::{stream::FuturesOrdered, TryStreamExt};
use ipnet::IpNet;
use net_route::{Handle, Route};
use tokio::sync::OnceCell;

use crate::error::RouteOpError;

use log::{info, warn};

pub static GATEWAY: OnceCell<Option<Ipv4Addr>> = OnceCell::const_new();
pub static INTERFACE_INDEX: OnceLock<u32> = OnceLock::new();

type Result<T> = std::result::Result<T, RouteOpError>;


/// Get default interface index.
///
/// Windows: use default IPv4 route table.
/// Other platforms: use netdev.
#[cfg(target_os = "windows")]
fn get_interface_index() -> std::result::Result<u32, String> {
    use std::process::Command;

    let output = Command::new("powershell")
        .args([
            "-Command",
            "(Get-NetRoute -DestinationPrefix '0.0.0.0/0' | Sort-Object RouteMetric | Select-Object -First 1).ifIndex"
        ])
        .output()
        .map_err(|e| e.to_string())?;

    let index = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u32>()
        .map_err(|e| e.to_string())?;

    Ok(index)
}


#[cfg(not(target_os = "windows"))]
fn get_interface_index() -> std::result::Result<u32, String> {
    netdev::get_default_interface()
        .map(|x| x.index)
}


/// Get default IPv4 gateway.
pub async fn get_gateway(handle: &Handle) -> Result<Option<Ipv4Addr>> {
    let routes = handle
        .list()
        .await?
        .into_iter()
        .filter(|r| r.gateway.is_some())
        .filter(|r| r.destination.is_unspecified())
        .filter_map(|r| r.gateway);


    for ip in routes {
        if let IpAddr::V4(ipv4) = ip {
            return Ok(Some(ipv4));
        }
    }

    Ok(None)
}


/// Add one IPv4 route entry.
pub async fn add_route(handle: &Handle, route: &IpNet) -> Result<()> {

    // Ignore IPv6 completely
    if !route.addr().is_ipv4() {
        return Ok(());
    }


    let gateway = GATEWAY
        .get_or_try_init(|| async {
            get_gateway(handle).await
        })
        .await?
        .ok_or(RouteOpError::NoGatewayError)?;


    let route_item = Route::new(route.addr(), route.prefix_len())
        .with_gateway(IpAddr::from(gateway))
        .with_ifindex(
            get_interface_index()
                .map_err(RouteOpError::GetInterfaceError)?
        );


    let result = handle.add(&route_item).await;


    if let Err(err) = result {

        if err.kind() == std::io::ErrorKind::Other
            && err.to_string().contains("exists")
        {
            return Err(RouteOpError::RouteAlreadyExistsError);
        }

        return Err(err.into());
    }


    Ok(())
}



/// Delete one IPv4 route entry.
pub async fn del_route(handle: &Handle, route: &IpNet) -> Result<()> {

    if !route.addr().is_ipv4() {
        return Ok(());
    }


    let route_item = Route::new(route.addr(), route.prefix_len())
        .with_ifindex(
            get_interface_index()
                .map_err(RouteOpError::GetInterfaceError)?
        );


    let result = handle.delete(&route_item).await;


    if let Err(err) = result {

        if err.kind() == std::io::ErrorKind::NotFound {
            return Err(RouteOpError::RouteNotFoundError);
        }

        return Err(err.into());
    }


    Ok(())
}



pub async fn add_routes(routes: &[IpNet]) -> Result<()> {

    let routes: Vec<IpNet> = routes
        .iter()
        .filter(|r| r.addr().is_ipv4())
        .cloned()
        .collect();


    info!("Adding {} IPv4 routes...", routes.len());


    let handle = Box::leak(Box::new(
        Handle::new()
            .map_err(|_| RouteOpError::HandleInitError)?
    ));


    let mut futures = routes
        .iter()
        .map(|r| add_route(handle, r))
        .collect::<FuturesOrdered<_>>();


    let mut index = 1;
    let mut ignored = 0;


    while let Some(result) = futures.try_next().await? {

        index += 1;

        if result.is_err() {
            ignored += 1;
        }
    }


    info!(
        "Routes added success, ignored: {}.",
        ignored
    );


    Ok(())
}



pub async fn del_routes(routes: &[IpNet]) -> Result<()> {


    let routes: Vec<IpNet> = routes
        .iter()
        .filter(|r| r.addr().is_ipv4())
        .cloned()
        .collect();


    info!("Removing {} IPv4 routes...", routes.len());


    let handle = Handle::new()
        .map_err(|_| RouteOpError::HandleInitError)?;


    let mut futures = routes
        .iter()
        .map(|r| del_route(&handle, r))
        .collect::<FuturesOrdered<_>>();


    while let Some(_) = futures.try_next().await? {}


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


    async fn test_add_remove_route(dest: IpAddr) {

        let handle = Handle::new().unwrap();


        let route = IpNet::new(dest, 32).unwrap();


        let _ = del_route(&handle, &route).await;


        add_route(&handle, &route)
            .await
            .unwrap();


        del_route(&handle, &route)
            .await
            .unwrap();
    }


    #[tokio::test]
    #[ignore = "Run as administrator"]
    async fn test_add_remove_route_v4() {

        test_add_remove_route(
            IpAddr::from([123,123,123,123])
        )
        .await;
    }
}
