# chnroutes-rs-next

简体中文 | [English](./README_en.md)

[chnroutes-rs](https://github.com/aabbc143/chnroutes-rs-next) 的现代化分支与重构版本。专注于高效生成中国大陆 IP 路由表及 BGP ASN 规则，具备多源融合、CIDR 聚合优化与系统路由表快速注入能力。

## 核心特性

* **多数据源支持**：兼容 APNIC 官方分配数据与 [chnroutes2](https://github.com/misakaio/chnroutes2) 高精聚合规则。
* **APNIC + BGP 混合策略**：融合实时 BGP 路由表信息，兼顾网络覆盖率与大陆路由判定精准度。
* **BGP ASN 精准分类**：支持根据自治系统号（ASN）对 IP 段进行归类，并自动标注国家与注册机构（Country / Registry）。
* **高性能原生 API 写入**：直接调用系统 API 直写路由表（Windows 1w+ 条目写入仅需 30ms），摆脱传统臃肿脚本。
* **全平台原生支持**：提供 Windows、Linux (GNU/MUSL) 及 macOS (Intel/Apple Silicon) 的全架构单文件二进制执行包。

## 安装

### 1. 下载预编译文件（推荐）
直接前往本仓库的 [Releases](../../releases) 页面下载对应系统的二进制压缩包：
* **Windows**: 解压 `chnroutes-rs-next-x86_64-pc-windows-msvc.zip`，将 `chnroutes-rs-next.exe` 放入 `C:\Windows\System32` 或任意 `PATH` 环境变量目录下。
* **Linux / macOS**: 下载对应的 `.tar.gz` 压缩包解压，将二进制文件赋予执行权限并移至 `/usr/local/bin/`。

### 2. 从源码编译
```sh
cargo install --git [https://github.com/aabbc143/chnroutes-rs-next](https://github.com/aabbc143/chnroutes-rs-next)