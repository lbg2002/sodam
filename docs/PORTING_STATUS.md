# SodaM Windows → Linux 迁移状态

更新时间：2026-11-07（Asia/Tokyo）

## 可核验输入

- Windows 安装包：`SodaMusic-v3.7.0-official-win32_x64.exe`，PE32 GUI、Nullsoft/NSIS 自解压安装包，大小约 144 MiB。
- 安装包字符串中出现 `Qt6` 与 `app_name: SodaMusic`；当前证据支持“Qt 6 原生客户端”，不支持 Electron、CEF 或 WebView2 的判断。
- 工作区已提供 `sodam-main.zip`，解压到 `sodam/`。压缩包没有 `.git` 目录，因此本地 Git 历史从导入提交开始保存；未覆盖上游历史。
- REA MCP/CLI 未在当前 Codex 环境注册，REA 仓库只能作为公开工具说明参考，未执行未提供的伪造命令。没有对 DRM、付费访问控制或安全保护进行绕过。

## SodaM 现有实现核对

| 模块 | 当前实现 | 本次核对结论 |
|---|---|---|
| UI | GPUI 原生窗口、首页/推荐、搜索、歌单、艺人、专辑、设置、登录、歌词、播放队列 | 可保留 |
| 领域层 | `sodam-core`，配置、会话、库、队列、历史、下载、本地库 | 可保留 |
| 网络/账号 | `libresoda` 会话与扫码登录流程 | 依赖真实账号和网络才能运行验证 |
| 播放 | rodio 专属线程，PipeWire/PulseAudio 经 ALSA/CPAL，进度/错误快照 | 已有后台播放与无设备容错 |
| 预加载 | 后台任务、并发上限、请求去重、失败 60 秒冷却、缓存命中优先、任务完成立即补位 | 已覆盖主要卡顿来源；当前缺少显式任务取消令牌 |
| 缓存 | 音频/歌词/封面缓存、LRU 清理、离线模式、缓存索引后台扫描 | 可保留 |
| 桌面集成 | MPRIS、SNI 托盘、桌面歌词、GNOME 顶栏歌词扩展、Wayland/X11 特性开关 | 已实现，需桌面会话实机验证 |

## Windows/Linux 功能差异（当前证据范围）

| 能力 | Windows 官方 Qt 客户端 | SodaM Linux |
|---|---|---|
| 程序框架 | Qt 6 原生（由安装包静态证据判断） | Rust + GPUI 原生 |
| 登录/账号 | 官方账号体系 | libresoda 会话与扫码登录；未使用授权账号验证 |
| 首页/推荐/搜索 | 安装包未能在无 Windows 运行时解包到页面资源 | 已有推荐、场景、综合/垂直搜索 |
| 播放/歌词/队列 | 未完成运行时动态观测 | 已有 rodio 播放、歌词同步、队列、睡眠定时、淡入淡出 |
| 预加载/缓存 | 未取得动态行为证据 | 并发预取、去重、冷却重试、离线缓存和 LRU |
| 下载 | 未取得动态行为证据 | 播放缓存导出、批量下载任务和下载管理 |
| Linux 桌面集成 | 不适用 | MPRIS、Wayland/X11、GNOME 顶栏歌词、SNI |

## 构建与交付

- `cargo check --offline --workspace`：已通过（Rust 1.96，jobs=2）。
- `scripts/check.sh`：已加入 systemd-run 用户总线不可用时的自动回退；当前容器中脚本完整通过（check、fmt、clippy、test）。
- `packaging/make-deb.sh`：新增 Debian 打包脚本，release 构建后输出 `dist/sodam_<version>_<arch>.deb` 及 SHA-256 文件。
- 真实账号登录、在线搜索、播放、歌词和连续切歌尚未在本环境完成：缺少授权账号、网络服务可达性和可启动的 Wayland/音频桌面会话。
