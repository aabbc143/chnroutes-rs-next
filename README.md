# chnroutes-rs-next

`chnroutes` 的 Rust 重构与增强版本，用于生成和写入中国大陆 IP 路由表，适合与 VPN、代理等工具配合实现国内外流量分流。

项目支持多种中国大陆 IP 数据源，并通过 CIDR 聚合、系统原生网络 API 等方式提高路由生成和注入效率。

## 核心特性

* **多数据源支持**

  * APNIC 官方分配数据
  * chnroutes2 高精度 CN 路由数据

* **CIDR 路由优化**

  * 对 IP 网段进行聚合，减少路由表项数量。

* **原生系统 API**

  * `up` / `down` 直接操作系统路由表。
  * Windows 使用系统网络 API 进行路由注入，避免逐条执行传统 `route` 批处理命令。

* **BGP / ASN 数据支持**

  * 支持自治系统号（ASN）相关数据处理和分类。

* **跨平台**

  * Windows
  * Linux
  * macOS

## 工作方式

`chnroutes-rs-next` 的基本工作流程：

```text
IP 数据源
   │
   ├── APNIC
   │
   └── chnroutes2
          │
          ▼
     CN IP 网段
          │
          ▼
      CIDR 聚合
          │
          ▼
    系统路由表注入
          │
          ▼
       VPN 分流
```

例如在 Windows 上配合 OpenVPN 使用时，可以先将中国大陆 IP 路由写入本机路由表，再启动 VPN。

这样可以实现：

```text
中国大陆 IP
    ↓
本地网络直接访问

非中国大陆 IP
    ↓
VPN / 代理访问
```

## 安装

目前推荐直接从 GitHub Releases 下载预编译版本。

[Releases](https://github.com/aabbc143/chnroutes-rs-next/releases?utm_source=chatgpt.com)

当前提供：

* Windows x86_64
* Linux x86_64 GNU
* macOS ARM64（Apple Silicon）

下载对应平台的压缩包后解压即可。

### Windows

下载：

```text
chnroutes-rs-next-x86_64-pc-windows-msvc.zip
```

解压后得到：

```text
chnroutes-rs-next.exe
```

有两种使用方式。

#### 方式一：直接运行

将 `chnroutes-rs-next.exe` 放在任意目录，然后在该目录打开 PowerShell：

```powershell
.\chnroutes-rs-next.exe --help
```

例如：

```powershell
.\chnroutes-rs-next.exe up
```

#### 方式二：加入 PATH

将程序所在目录加入 Windows 的 `PATH` 环境变量。

之后可以直接：

```powershell
chnroutes-rs-next --help
```

如果希望简单地让程序在任意目录都可以调用，也可以将程序放入 Windows PATH 已包含的目录。

## 系统要求

### Windows 管理员权限

修改 Windows 系统路由表需要管理员权限。

因此执行以下命令时，请使用**以管理员身份运行的 PowerShell 或 Windows Terminal**：

```powershell
chnroutes-rs-next up
```

否则可能无法写入系统路由表。

### VC++ 运行库

如果程序启动时提示缺少：

```text
VCRUNTIME140.dll
```

请安装官方 Microsoft Visual C++ Redistributable x64：

[Microsoft Visual C++ Redistributable (x64)](https://aka.ms/vs/17/release/vc_redist.x64.exe?utm_source=chatgpt.com)

## 使用

### 查看帮助

```powershell
chnroutes-rs-next --help
```

查看版本：

```powershell
chnroutes-rs-next --version
```

查看 `up`：

```powershell
chnroutes-rs-next up --help
```

查看 `export`：

```powershell
chnroutes-rs-next export --help
```

## 直接写入系统路由表

这是推荐的使用方式。

### 使用 APNIC 数据

```powershell
chnroutes-rs-next up --source apnic
```

APNIC 是默认数据源，因此以下命令与上面的命令等价：

```powershell
chnroutes-rs-next up
```

### 使用 chnroutes2 数据

```powershell
chnroutes-rs-next up --source chnroutes2
```

如果网络环境可以正常访问 chnroutes2 数据源，推荐根据实际需求选择 `chnroutes2` 数据。

## 删除已写入的路由

使用：

```powershell
chnroutes-rs-next down
```

该命令用于清理程序写入的中国大陆 IP 路由。

## 导出路由脚本

如果不希望直接通过系统 API 写入路由，也可以使用 `export` 导出路由操作内容。

例如 Windows：

```powershell
chnroutes-rs-next export --platform windows
```

使用 chnroutes2：

```powershell
chnroutes-rs-next export --platform windows --source chnroutes2
```

`export` 更适合需要查看、保存或进一步处理路由脚本的场景。

对于日常使用，推荐直接使用：

```powershell
chnroutes-rs-next up
```

或者：

```powershell
chnroutes-rs-next up --source chnroutes2
```

因为 `up` / `down` 直接调用系统 API 操作路由表，不需要再生成脚本并逐条执行。

## 与 VPN 配合使用

一个典型的 Windows 分流流程：

### 第一步：以管理员身份打开 PowerShell

进入程序所在目录，例如：

```powershell
cd C:\Tools\chnroutes-rs-next
```

### 第二步：写入中国大陆路由

使用 APNIC：

```powershell
.\chnroutes-rs-next.exe up --source apnic
```

或者使用 chnroutes2：

```powershell
.\chnroutes-rs-next.exe up --source chnroutes2
```

### 第三步：确认路由表

可以使用：

```powershell
route print
```

查看已经写入的中国大陆 IP 路由。

### 第四步：启动 VPN

完成中国大陆路由写入后，再启动 OpenVPN 等 VPN 客户端。

这样中国大陆 IP 可以继续通过本地网络访问，而其他流量可以通过 VPN。

## 数据源

### APNIC

APNIC 是默认数据源。

```powershell
chnroutes-rs-next up --source apnic
```

也可以显式指定：

```powershell
chnroutes-rs-next export --platform windows --source apnic
```

### chnroutes2

使用 chnroutes2：

```powershell
chnroutes-rs-next up --source chnroutes2
```

或者：

```powershell
chnroutes-rs-next export --platform windows --source chnroutes2
```

如果当前网络无法直接访问 chnroutes2 数据源，程序可能会出现网络连接或 TLS 错误。

例如：

```text
tls handshake eof
```

这种情况首先应检查当前网络是否能够访问对应的数据源。

如果用户本身正在使用 Clash、VPN 或其他网络代理，应确保该网络环境能够访问 chnroutes2 数据源。

## 网络环境说明

程序需要访问对应的数据源获取路由数据。

例如 chnroutes2：

```text
https://chnroutes2.cdn.skk.moe/chnroutes.txt
```

正常情况下，如果系统网络可以访问该地址，程序会自动获取数据。

程序运行时不要求用户手动设置：

```powershell
$env:HTTP_PROXY
$env:HTTPS_PROXY
```

只有在当前网络环境无法正常访问数据源时，才需要根据实际代理环境进行额外配置。

## 性能

`up` / `down` 采用系统原生网络 API 操作路由表，相比生成批处理脚本后逐条执行，可以显著减少路由注入所需时间。

Windows 路由注入性能在测试环境中可达到较高速度。

具体耗时会受到 Windows 版本、CPU、当前路由表规模以及网络设备等因素影响。

## 命令速查

```powershell
# 查看帮助
chnroutes-rs-next --help

# 查看版本
chnroutes-rs-next --version

# 使用默认 APNIC 数据源写入路由
chnroutes-rs-next up

# 明确使用 APNIC
chnroutes-rs-next up --source apnic

# 使用 chnroutes2 数据源
chnroutes-rs-next up --source chnroutes2

# 删除程序写入的路由
chnroutes-rs-next down

# 导出 Windows 路由操作内容
chnroutes-rs-next export --platform windows

# 使用 chnroutes2 导出
chnroutes-rs-next export --platform windows --source chnroutes2
```

## 从源码构建

需要安装 Rust 工具链。

构建 CLI：

```powershell
cargo build --release --features bin
```

构建完成后，Windows 可执行文件位于：

```text
target\release\chnroutes-rs-next.exe
```

检查：

```powershell
.\target\release\chnroutes-rs-next.exe --version
```

## 项目

GitHub：

[aabbc143/chnroutes-rs-next](https://github.com/aabbc143/chnroutes-rs-next?utm_source=chatgpt.com)

原项目：

[lxl66566/chnroutes-rs](https://github.com/lxl66566/chnroutes-rs?utm_source=chatgpt.com)

## License

MIT
