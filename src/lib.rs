pub mod cache;
pub mod error;
pub mod route_op;
pub mod service;
pub mod source;
pub mod state;
pub mod target;
pub mod update;

pub use error::{Error, Result};
pub use source::Source;
pub use state::State;
pub use target::Target;

pub async fn up(source: &Source) -> Result<()> {
    let ips = source.get_cn_ips()?;

    // 执行路由添加，获取实际成功的 RouteResult 对象
    let result = route_op::add_routes(&ips).await?;

    // 校验接口 index，拒绝保存无效的 0 接口
    let ifindex = route_op::interface_index().map_err(error::RouteOpError::GetInterfaceError)?;

    // 仅收集实际写入成功/已存在的网段
    let routes_str = result
        .success_routes
        .iter()
        .map(|ip| ip.to_string())
        .collect();

    // 直接使用 Source 原生实现的 as_str()
    let state = State::new(source.as_str().to_string(), ifindex, routes_str);

    if let Err(e) = state.save() {
        log::warn!("Failed to save state file: {}", e);
    }

    Ok(())
}

pub async fn down(source: &Source) -> Result<()> {
    // 优先加载持久化 State 进行精确清理；若 State 不存在，回退到重新拉取 Source
    let routes_to_del = if let Some(state) = State::load() {
        state
            .routes
            .iter()
            .filter_map(|r| r.parse::<ipnet::IpNet>().ok())
            .collect()
    } else {
        source.get_cn_ips()?
    };

    route_op::del_routes(&routes_to_del).await?;

    if let Err(e) = State::remove() {
        log::warn!("Failed to remove state file: {}", e);
    }

    Ok(())
}

pub async fn restore() -> Result<()> {
    let state = match State::load() {
        Some(state) => state,
        None => {
            log::warn!("No state file found");
            return Ok(());
        }
    };

    let routes: Vec<ipnet::IpNet> = state.routes.iter().filter_map(|x| x.parse().ok()).collect();

    log::info!(
        "Restoring {} routes from state using interface index {}...",
        routes.len(),
        state.interface_index
    );

    let result = route_op::add_routes(&routes).await?;

    log::info!(
        "Restore result: added={}, already_exists={}, failed={}",
        result.added,
        result.already_exists,
        result.failed
    );

    if result.failed > 0 {
        return Err(error::RouteOpError::OpError(std::io::Error::other(format!(
            "Failed to restore {} of {} routes",
            result.failed,
            routes.len()
        )))
        .into());
    }

    if result.added + result.already_exists != routes.len() {
        return Err(error::RouteOpError::OpError(std::io::Error::other(format!(
            "Route restore incomplete: {} successful out of {}",
            result.added + result.already_exists,
            routes.len()
        )))
        .into());
    }

    Ok(())
}

pub async fn auto_restore() -> Result<()> {
    let max_retries = 6;
    let mut delay_secs = 10;

    for attempt in 1..=max_retries {
        log::info!("Auto-restore attempt {}/{}", attempt, max_retries);

        match restore().await {
            Ok(_) => {
                log::info!("Auto-restore completed successfully.");
                return Ok(());
            }
            Err(e) => {
                log::warn!("Auto-restore attempt {} failed: {}", attempt, e);
                if attempt < max_retries {
                    log::info!("Waiting {} seconds before retrying...", delay_secs);
                    tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
                    delay_secs += 10;
                } else {
                    return Err(e);
                }
            }
        }
    }

    Ok(())
}

pub async fn update(source: &Source) -> Result<Option<update::DiffResult>> {
    update::update_diff(source).await
}
