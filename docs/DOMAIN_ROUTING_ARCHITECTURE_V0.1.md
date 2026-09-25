# Domain Routing Architecture v0.1

## 目标

在不破坏 0.5.0 现有 IP Route Engine 的前提下，引入可扩展的 Domain Routing 架构。

核心原则：

- IP Route Engine 继续负责 CIDR → 系统路由表。
- Domain Routing 负责 Domain → Policy → Resolution/Traffic Intent。
- DNS、SNI、QUIC 等域名发现机制属于 Domain/Traffic 层，不直接写入 route_op。
- OpenVPN / WireSock 继续作为现有兼容 Backend；真正的 Domain Traffic Routing 后续通过成熟 Traffic Backend 实现。
- IPv4 与 IPv6 从数据模型层同时设计，Route Backend 可分阶段实现。
- State、Diff、Reconcile、Self-Healing 保持与 0.5.0 相同的事务思路。
- 0.5.0 不冻结；feature/domain-routing 独立演进。

## 目标分层

\`\`\`text
CLI / Config
    |
    v
Policy Engine
    |
    +--> Domain Rule Source
    |      +-- geosite / v2ray-rules-dat
    |      +-- user rules
    |
    v
Decision
    |
    +--> DNS Resolver ----> A / AAAA / TTL
    |
    +--> Traffic Backend --> mature traffic core
    |
    v
Route Intent
    |
    v
Reconciler
    |
    v
Route Backend
    +-- IPv4
    +-- IPv6
    |
    v
Windows Network
\`\`\`

闭环：

\`\`\`
Desired Policy
 -> Resolve / Observe
 -> Desired State
 -> Apply Backend
 -> Observe Actual State
 -> Reconcile
 -> Desired State
\`\`\`

## 核心领域模型

### DomainPolicy

描述一个域名最终希望采取的动作，不绑定具体 Backend。

候选动作：

- Direct
- Proxy
- Auto
- Block
- Bypass / NoOverride

暂不把 OpenVPN、WireSock、sing-box、Mihomo 等具体实现写进 Policy。

### Rule

Rule 至少包含：

- matcher 类型
- matcher value
- action
- priority
- enabled
- source

Matcher 初期应支持：

- exact domain
- domain suffix
- domain keyword

后续再接：

- regex
- geosite
- rule-set
- IP / ASN
- process
- network / protocol

### Decision

Policy Engine 输出确定性结果：

\`\`\`
Input Domain
 -> matched rule
 -> Decision { action, matched_rule, reason }
\`\`\`

规则优先级必须稳定、可解释，不能依赖 HashMap 遍历顺序。

## DomainRecord

DNS 结果必须保存：

- domain
- A records
- AAAA records
- TTL
- resolved_at
- expires_at
- resolver identity
- generation

Domain → IP 映射不能永久缓存。

TTL 到期后必须允许重新解析。

## Resolver

Resolver 是独立接口，不属于 Route Backend。

职责：

- resolve A / AAAA
- 返回 TTL
- 支持取消 / timeout
- 区分成功、NXDOMAIN、SERVFAIL、timeout
- 为后续 DoH / DoT / system resolver 留扩展空间

## RouteIntent

Domain 层不能直接调用 Windows API。

Domain Resolver 最终只产生类似：

\`\`\`
RouteIntent {
    network: IpNet,
    action: Direct / Proxy / Block,
    owner: Domain(domain),
    generation,
    expires_at
}
\`\`\`

然后由 Reconciler 决定实际 Backend 行为。

## RouteBackend

目标接口：

\`\`\`
trait RouteBackend {
    inspect()
    apply()
    remove()
    reconcile()
}
\`\`\`

当前 \`route_op.rs\` 是第一代实现来源。

后续逐步拆分：

- Windows IPv4
- Windows IPv6
- other OS adapters

现阶段不重写 route_op，只在 feature/domain-routing 中建立边界。

## TrafficBackend

Route Backend 只解决 IP 层。

真正的 Domain Traffic Routing 需要独立 TrafficBackend：

- sing-box
- Mihomo
- Xray

本项目不复制这些项目的代理核心。

TrafficBackend 只负责：

- 接收 Domain Policy
- 接收 Route/Rule intent
- 启停或更新成熟核心
- 查询运行状态

## State

当前 \`state.rs\` 是纯 IP Route State。

不直接把 Domain 字段塞入现有 State。

未来：

\`\`\`
RuntimeState
  +-- RouteState
  +-- DomainState
  +-- ResolverState
  +-- BackendState
\`\`\`

第一阶段可以继续保持现有 \`chnroutes-state.json\` 兼容。

Domain State 使用独立文件或独立 namespace。

## Reconciler

这是整个新架构的核心。

流程：

1. 读取 Desired Policy
2. 解析 Domain
3. 生成 Desired State
4. 获取 Actual State
5. 计算 diff
6. Apply
7. Verify
8. Commit State

失败时：

- 不提交错误 State
- 保留上一份有效 State
- 下一轮继续 reconcile

这直接继承 0.5.0 的 update/state 事务思想。

## NetworkWatcher

监听：

- 网卡变化
- 默认网关变化
- DNS 配置变化
- VPN connect / disconnect
- sleep / resume

发生变化时触发 Reconcile，而不是等待固定周期。

固定周期只作为兜底。

## IPv4 / IPv6

架构从第一天支持：

- A
- AAAA
- IPv4 CIDR
- IPv6 CIDR

Policy 不区分 IPv4/IPv6。

Backend 再根据 address family 执行。

IPv6 Route Backend 可以晚于 IPv4 实现，但不能让上层模型成为 IPv4-only。

## Backend ownership

必须明确：

- IP Route Backend 管系统 CIDR route
- Domain Backend 管 Domain Policy
- Traffic Backend 管代理核心
- Resolver 管 DNS 状态
- Reconciler 管 Desired/Actual convergence

任何模块不能越界直接修改另一个模块的 State。

## 与当前 0.5.0 的对应关系

| v0.5.0 | Domain Routing v0.1 |
|---|---|
| source/* | domain source 独立新增 |
| route_op.rs | RouteBackend 第一实现 |
| state.rs | RouteState |
| update.rs | Reconciler 思想来源 |
| service.rs | Supervisor / Watcher 容器 |
| cache.rs | Resolver / Rule cache 可复用思想 |
| target.rs | Legacy Export Backend |
| error.rs | 扩展 Domain / DNS / Backend errors |
| main.rs | 后续 CLI/config boundary |

## 第一阶段明确不做

- 不重写 route_op.rs
- 不实现自研 WFP traffic core
- 不实现 WinDivert VPN core
- 不实现 Wintun VPN core
- 不把项目变成 sing-box
- 不把 WireSock/OpenVPN 私有逻辑塞进 Policy Engine
- 不强制冻结 0.5.0
- 不一次性实现所有 Domain matcher

## 第一阶段实现顺序

1. 建立 domain 模块边界
2. 建立 DomainPolicy / Rule / Decision
3. 建立 DomainRecord / Resolver trait
4. 建立 RouteIntent
5. 建立 RouteBackend trait
6. 把现有 route_op 封装为兼容实现
7. 建立独立 DomainState
8. 建立最小 Reconciler
9. 加入 exact / suffix / keyword matcher
10. 再接 DNS 实现
11. 最后接成熟 Traffic Backend

第一阶段完成标准：

\`\`\`
domain rule
 -> decision
 -> DNS
 -> A/AAAA
 -> RouteIntent
 -> existing route engine
 -> state
 -> reconcile
\`\`\`

整个链路可独立测试，且不影响现有 \`up/down/update/restore\`。
