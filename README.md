# chnroutes-rs-next

高效的中国大陆 IP 路由表及 BGP/ASN 规则生成与直写工具（Rust 重构版）。

支持多源融合、CIDR 聚合优化，并提供原生 Windows API 快速直写能力（1 万条路由注入仅需 30ms）。

## 核心特性

* **原生 API 秒级注入**：摆脱臃肿批处理，直接调用系统 Network API 实时读写路由表。
* **多源策略融合**：支持 APNIC 官方数据与 chnroutes2 高精聚合规则。
* **BGP ASN 精准归类**：支持自治系统号查询与归属地（Country / Registry）标注。
* **全平台原生构建**：提供 Windows、Linux (GNU/MUSL)、macOS 的预编译文件。

## 程序放置 (Windows)

解压得到 `chnroutes-rs-next.exe`，按需选择：

* **全局调用（推荐）**：直接复制到 `C:\Windows\System32\`，即可在任何目录下运行。
* **解压即用**：放在任意文件夹，在当前目录下按 `Shift + 鼠标右键` 打开终端，带上 `.\` 运行（如 `.\chnroutes-rs-next.exe up`）。

## 快速开始 (Windows)

> **注意**：执行路由表写入（`up`）与清理（`down`）必须使用**管理员权限**打开终端。

### 常用命令

```powershell
# 自动获取最新大陆 IP 库并直写系统路由表
chnroutes-rs-next up

# 清理并还原所有注入的路由表项
chnroutes-rs-next down

# 导出 Windows 静态路由脚本
chnroutes-rs-next export -p windows -o routes.bat