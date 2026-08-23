# chnroutes-rs-next

`chnroutes` 的 Rust 重构与增强版本，用于生成和写入中国大陆 IP 路由表，适合与 VPN、代理等工具配合实现国内外流量分流。

项目支持多种中国大陆 IP 数据源，并通过 CIDR 聚合、系统原生网络 API 等方式提高路由生成与注入效率。

## 核心特性

* **多数据源支持**：支持 APNIC 官方分配数据与 `chnroutes2` 高精度 CN 路由数据。
* **CIDR 路由优化**：自动对 IP 网段进行聚合，大幅减少路由表项数量。
* **原生系统 API**：直接操作系统路由表（`up` / `down`），Windows 下避免逐条执行传统 `route` 批处理命令。
* **BGP / ASN 支持**：支持自治系统号（ASN）相关数据处理和分类。
* **跨平台**：支持 Windows、Linux、macOS。

## 安装

推荐直接从 [Releases](https://github.com/aabbc143/chnroutes-rs-next/releases) 下载预编译版本：

* **Windows**: `x86_64-pc-windows-msvc.zip`
* **Linux**: `x86_64-unknown-linux-gnu.tar.gz`
* **macOS**: `aarch64-apple-darwin.tar.gz` (Apple Silicon)

解压后即可直接运行（建议将程序所在目录加入系统的 `PATH` 环境变量以便全局调用）。

## 系统要求

* **管理员权限**：修改系统路由表必须具备管理员权限。Windows 请使用**以管理员身份运行**的 PowerShell / Terminal；Linux / macOS 请使用 `sudo`。
* **VC++ 运行库**：若 Windows 启动时提示缺少 `VCRUNTIME140.dll`，请安装 [Microsoft Visual C++ Redistributable (x64)](https://aka.ms/vs/17/release/vc_redist.x64.exe)。

## 快速使用

日常推荐直接使用 `up` / `down` 命令，程序会调用系统原生 API 自动写入或清除，无需导出脚本。

### 1. 写入系统路由表

```powershell
# 使用 APNIC 数据源（默认）
chnroutes-rs-next up

# 使用 chnroutes2 高精度数据源
chnroutes-rs-next up --source chnroutes2
