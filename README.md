# chnroutes-rs-next

`chnroutes-rs` 的增强版本，用于生成和管理中国大陆 IP 路由规则，实现 VPN / 代理环境下的国内外流量分流。

<<<<<<< HEAD
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
```

### 2. 删除已写入的路由

```powershell
chnroutes-rs-next down
```

### 3. 查看路由表状态

```powershell
route print
```

=======
相比原项目，`chnroutes-rs-next` 增加了多数据源支持（`chnroutes2`）、CIDR 路由优化、BGP / ASN 分析能力，并改用系统原生网络 API 进行路由注入。

## 核心特性

* **多数据源支持**：支持 APNIC 官方分配数据及 `chnroutes2` 高精度 CN 路由数据。
* **CIDR 路由优化**：自动对 IP 网段进行合并与聚合，大幅减少系统路由表项数量，提高加载效率。
* **原生路由操作**：
  * `up` / `down` 直接写入或删除系统路由表。
  * **Windows 平台优势**：通过系统原生 IP Helper API (`IPHLPAPI`) 直接注入/删除路由表，无需生成低效的批处理 `.bat` 脚本逐条执行。
* **BGP / ASN 数据支持**：支持自治系统号（ASN）相关数据的提取、分类与处理。
* **跨平台**：支持 Windows、Linux、macOS。

## 与原项目区别

| 特性 / 功能 | chnroutes-rs | chnroutes-rs-next |
| :--- | :---: | :---: |
| Rust 实现与路由规则生成 | ✅ | ✅ |
| APNIC 数据源 | ✅ | ✅ |
| **chnroutes2 数据源支持** | ❌ | **✅** |
| CIDR 路由聚合 | 基础支持 | **增强优化** |
| BGP / ASN 数据处理 | ❌ | **✅** |
| **系统路由注入方式** | 路由脚本导出 | **系统 API 直接操作** |

## 常用命令

> **注意**：直接修改系统路由表（执行 `up` / `down`）需要**管理员 (Admin / root) 权限**。

```bash
# 查看帮助
chnroutes-rs-next --help

# 写入中国大陆路由（默认使用 APNIC 数据源）
chnroutes-rs-next up

# 使用 chnroutes2 数据源写入路由
chnroutes-rs-next up --source chnroutes2

# 清除已写入的中国大陆路由
chnroutes-rs-next down

# 导出路由脚本（支持 Windows / macOS / Linux / Android / OpenVPN）
chnroutes-rs-next export --platform windows

## 安装与构建
## 预编译版本
直接从 Releases 下载适用于 Windows (x86_64)、Linux (x86_64) 或 macOS (ARM64) 的最新二进制文件。
>>>>>>> 3723191 (Update README documentation)

从源码构建
需要 Rust 工具链支持：

<<<<<<< HEAD
需提前安装 Rust 工具链：

=======
>>>>>>> 3723191 (Update README documentation)
```bash
cargo build --release --features bin
```
构建完成后：

<<<<<<< HEAD
构建完成后，可执行文件位于 `target/release/chnroutes-rs-next`（Windows 为 `.exe`）。

## 关联项目

* GitHub：[aabbc143/chnroutes-rs-next](https://github.com/aabbc143/chnroutes-rs-next)
* 原项目：[lxl66566/chnroutes-rs](https://github.com/lxl66566/chnroutes-rs)
=======
Windows:

```text
target/release/chnroutes-rs-next.exe
```

Linux / macOS:

```text
target/release/chnroutes-rs-next
```

## 项目地址

本项目：

https://github.com/aabbc143/chnroutes-rs-next

基于：

https://github.com/lxl66566/chnroutes-rs
>>>>>>> 3723191 (Update README documentation)

## License

MIT License