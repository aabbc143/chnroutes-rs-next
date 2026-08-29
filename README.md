# chnroutes-rs-next

`chnroutes-rs-next` 是 `chnroutes-rs` 的增强版本，用于生成和管理中国大陆 IP 路由规则，实现 VPN / 代理环境下国内外流量分流。

相比原项目，`chnroutes-rs-next` 增加了多数据源支持、CIDR 路由优化、BGP / ASN 数据处理，并采用系统原生网络 API 管理路由。


## 核心特性

- **多数据源支持**
  - APNIC 官方 IP 分配数据
  - `chnroutes2` 高精度 CN 路由数据

- **CIDR 路由优化**
  - 自动合并 IP 网段
  - 减少系统路由数量，提高加载效率

- **原生路由管理**
  - 直接写入和删除系统路由表
  - Windows 使用 IP Helper API (`IPHLPAPI`) 操作路由，无需生成 `.bat` 脚本

- **自动路由维护**
  - 路由状态保存
  - 自动恢复路由
  - Windows 服务支持
  - 定时检查并更新路由变化

- **BGP / ASN 支持**
  - 支持自治系统数据分析与处理

- **跨平台支持**
  - Windows
  - Linux
  - macOS


# 安装

推荐从 Releases 下载预编译版本：

https://github.com/aabbc143/chnroutes-rs-next/releases


Windows：


建议将 chnroutes-rs-next.exe 所在目录加入 Windows PATH 环境变量：


# 使用方法

运行路由相关命令需要管理员权限。


## 写入中国大陆路由

默认 APNIC 数据源：

```powershell
chnroutes-rs-next up
```

使用 chnroutes2 数据源：

```powershell
chnroutes-rs-next up --source chnroutes2
```


## 删除路由

删除默认路由：

```powershell
chnroutes-rs-next down
```

删除 chnroutes2 路由：

```powershell
chnroutes-rs-next down --source chnroutes2
```


## 自动恢复

恢复已保存的路由：

```powershell
chnroutes-rs-next auto-restore
```


## Windows 服务

安装自动恢复服务：

```powershell
chnroutes-rs-next install-service
```

安装后：

- Windows 启动自动恢复路由
- 网络就绪后自动加载路由
- 后台自动检查路由状态
- 定时检测路由变化并应用更新


删除服务：

```powershell
chnroutes-rs-next remove-service
```


## 更新路由

检查数据源变化并更新：

```powershell
chnroutes-rs-next update
```

程序会自动处理差异：

- 新增路由 → 添加
- 删除路由 → 移除
- 未变化路由 → 保留


## 查看路由状态

Windows：

```powershell
route print
```

或者：

```powershell
Get-NetRoute -AddressFamily IPv4
```


# 从源码构建

安装 Rust：

https://rustup.rs/


构建命令行程序：

```bash
cargo build --release --features bin
```


构建完成：

Windows：

```
target/release/chnroutes-rs-next.exe
```

Linux/macOS：

```
target/release/chnroutes-rs-next
```


# 项目地址

本项目：

https://github.com/aabbc143/chnroutes-rs-next


原项目：

https://github.com/lxl66566/chnroutes-rs


# License

MIT License
