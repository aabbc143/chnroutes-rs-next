# chnroutes-rs-next

`chnroutes-rs-next` 是 `chnroutes-rs` 的增强版本，用于生成和管理中国大陆 IP 路由规则，实现 VPN / 代理环境下的国内外流量分流。

相比原项目，`chnroutes-rs-next` 增加了多数据源支持（`chnroutes2`）、CIDR 路由优化、BGP / ASN 分析能力，并改用系统原生网络 API 进行路由注入。

## 核心特性

* **多数据源支持**：支持 APNIC 官方分配数据及 `chnroutes2` 高精度 CN 路由数据。
* **CIDR 路由优化**：自动对 IP 网段进行合并与聚合，大幅减少系统路由表项数量，提高加载效率。
* **原生路由操作**：
  * `up` / `down` 直接写入或删除系统路由表。
  * **Windows 平台优势**：通过系统原生 IP Helper API (`IPHLPAPI`) 直接注入/删除路由，无需生成低效的批处理 `.bat` 脚本逐条执行。
* **BGP / ASN 支持**：支持自治系统号（ASN）相关数据的提取、分类与处理。
* **跨平台**：支持 Windows、Linux、macOS。

## 与原项目区别

| 特性 / 功能 | chnroutes-rs | chnroutes-rs-next |
| :--- | :---: | :---: |
| Rust 实现与路由规则生成 | ✅ | ✅ |
| APNIC 数据源 | ✅ | ✅ |
| **chnroutes2 数据源支持** | ❌ | **✅** |
| CIDR 路由聚合 | 基础支持 | **增强优化** |
| BGP / ASN 数据处理 | ❌ | **✅** |
| Windows 路由注入 | ❌ 脚本方式 | **✅ 原生路由 API** |

## 系统要求

* **管理员权限**：修改系统路由表必须具备管理员权限。Windows 请使用**以管理员身份运行**的 PowerShell / Terminal；Linux / macOS 请使用 `sudo`。
* **VC++ 运行库**：若 Windows 启动时提示缺少 `VCRUNTIME140.dll`，请安装 [Microsoft Visual C++ Redistributable (x64)](https://aka.ms/vs/17/release/vc_redist.x64.exe)。

## 安装

推荐直接从 [Releases](https://github.com/aabbc143/chnroutes-rs-next/releases) 下载预编译版本。

### Windows PATH 配置（推荐）

建议将 `chnroutes-rs-next.exe` 所在目录加入 Windows PATH 环境变量，以便在任意目录执行：

```powershell
chnroutes-rs-next up
```

若未配置 PATH，需进入程序所在目录执行：

```powershell
.\chnroutes-rs-next.exe up
```

## 使用方法

运行 `up` / `down` 命令需要管理员权限（请使用「管理员身份运行 PowerShell」）。

### 1. 写入系统路由表

* **使用默认 APNIC 数据源**：
  ```powershell
  chnroutes-rs-next up
  ```

* **使用 chnroutes2 高精度数据源**：
  ```powershell
  chnroutes-rs-next up --source chnroutes2
  ```

### 2. 删除已写入的路由

* **删除默认路由**：
  ```powershell
  chnroutes-rs-next down
  ```

* **删除通过 chnroutes2 写入的路由**：
  ```powershell
  chnroutes-rs-next down --source chnroutes2
  ```

### 3. 查看路由表状态

Windows 下可运行：

```powershell
route print
```

## 从源码构建

需要提前安装 Rust 工具链：

```bash
cargo build --release --features bin
```

构建完成后，可执行文件位于 `target/release/chnroutes-rs-next`（Windows 下为 `.exe`）。

## 项目地址

* **本项目**：[aabbc143/chnroutes-rs-next](https://github.com/aabbc143/chnroutes-rs-next)
* **原项目**：[lxl66566/chnroutes-rs](https://github.com/lxl66566/chnroutes-rs)

## License

MIT License