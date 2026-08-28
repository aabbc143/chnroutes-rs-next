use std::collections::HashSet;

use ipnet::IpNet;
use log::{error, info};

use crate::error::Result;
use crate::route_op;
use crate::source::Source;
use crate::state::State;

#[derive(Debug, Clone)]
pub struct DiffResult {
    pub added: Vec<IpNet>,
    pub removed: Vec<IpNet>,
    pub unchanged: usize,
}

fn can_commit_state(add_failed: usize, remove_failed: usize) -> bool {
    add_failed == 0 && remove_failed == 0
}

pub async fn update_diff(source: &Source) -> Result<Option<DiffResult>> {
    // 1. 获取最新路由数据
    let new_routes = source.get_cn_ips()?;

    // 2. 读取已有状态
    let old_state = State::load();

    let (old_routes, if_index, source_name) = match old_state {
        Some(state) => (
            state
                .routes
                .into_iter()
                .filter_map(|route| route.parse::<IpNet>().ok())
                .collect::<HashSet<IpNet>>(),
            state.interface_index,
            state.source,
        ),

        None => (
            HashSet::new(),
            route_op::interface_index().map_err(|e| {
                std::io::Error::other(format!(
                    "No valid interface index available for route update: {e}"
                ))
            })?,
            source.as_str().to_string(),
        ),
    };

    // 3. 新数据转为集合
    let new_set: HashSet<IpNet> = new_routes.into_iter().collect();

    // 4. 计算差分
    let to_add: Vec<IpNet> = new_set.difference(&old_routes).cloned().collect();
    let to_remove: Vec<IpNet> = old_routes.difference(&new_set).cloned().collect();
    let unchanged = new_set.intersection(&old_routes).count();

    info!(
        "Route diff computed: +{} to add, -{} to remove, {} unchanged.",
        to_add.len(),
        to_remove.len(),
        unchanged
    );

    // 5. 完全没有变化
    if to_add.is_empty() && to_remove.is_empty() {
        info!("Route set is already up-to-date.");

        return Ok(Some(DiffResult {
            added: Vec::new(),
            removed: Vec::new(),
            unchanged,
        }));
    }

    // 6. 添加新增路由（校验 failed 数量）
    let mut add_failed = 0;
    if !to_add.is_empty() {
        info!("Adding {} new routes...", to_add.len());

        let add_result = route_op::add_routes(&to_add).await?;
        add_failed = add_result.failed;

        if add_failed > 0 {
            error!(
                "Failed to add {} routes. Keeping previous state.",
                add_failed
            );

            return Err(std::io::Error::other(format!(
                "failed to add {} routes during route refresh",
                add_failed
            ))
            .into());
        }
    }

    // 7. 删除失效旧路由（校验 failed 数量，not_found 视为非失败）
    let mut remove_failed = 0;
    if !to_remove.is_empty() {
        info!("Removing {} obsolete routes...", to_remove.len());

        let remove_result = route_op::del_routes(&to_remove).await?;
        remove_failed = remove_result.failed;

        if remove_failed > 0 {
            error!(
                "Failed to remove {} old routes. Keeping previous state.",
                remove_failed
            );

            return Err(std::io::Error::other(format!(
                "failed to remove {} old routes during route refresh",
                remove_failed
            ))
            .into());
        }
    }

    // 8. 校验事务状态
    if !can_commit_state(add_failed, remove_failed) {
        return Err(std::io::Error::other("route operation failed; state not committed").into());
    }

    // 9. 只有全数成功才写入新 State
    let new_routes_str = new_set.iter().map(ToString::to_string).collect::<Vec<_>>();

    let new_state = State::new(source_name, if_index, new_routes_str);

    if let Err(e) = new_state.save() {
        error!("Failed to save refreshed state: {}", e);
        return Err(std::io::Error::other(e.to_string()).into());
    }

    info!("Route state updated successfully.");

    Ok(Some(DiffResult {
        added: to_add,
        removed: to_remove,
        unchanged,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::str::FromStr;

    #[test]
    fn test_commit_state_only_when_all_route_operations_succeed() {
        assert!(can_commit_state(0, 0));
        assert!(!can_commit_state(1, 0));
        assert!(!can_commit_state(0, 1));
        assert!(!can_commit_state(3, 2));
    }

    #[test]
    fn test_route_diff() {
        let old: HashSet<IpNet> = [
            IpNet::from_str("10.0.0.0/24").unwrap(),
            IpNet::from_str("10.0.1.0/24").unwrap(),
            IpNet::from_str("10.0.2.0/24").unwrap(),
        ]
        .into_iter()
        .collect();

        let new: HashSet<IpNet> = [
            IpNet::from_str("10.0.1.0/24").unwrap(),
            IpNet::from_str("10.0.2.0/24").unwrap(),
            IpNet::from_str("10.0.3.0/24").unwrap(),
        ]
        .into_iter()
        .collect();

        let to_add: HashSet<IpNet> = new.difference(&old).cloned().collect();
        let to_remove: HashSet<IpNet> = old.difference(&new).cloned().collect();

        assert_eq!(to_add.len(), 1);
        assert!(to_add.contains(&IpNet::from_str("10.0.3.0/24").unwrap()));

        assert_eq!(to_remove.len(), 1);
        assert!(to_remove.contains(&IpNet::from_str("10.0.0.0/24").unwrap()));

        assert_eq!(new.intersection(&old).count(), 2);
    }
}
