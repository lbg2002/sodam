# SodaM Windows → Linux 迁移状态

更新时间：2026-11-07（Asia/Tokyo）

## 可核验输入

- Windows 安装包：`SodaMusic-v3.7.0-official-win32_x64.exe`，PE32 GUI、Nullsoft/NSIS 自解压安装包，大小约 144 MiB。
- 安装包是 NSIS stub，导入表仅包含 Windows 系统 DLL；压缩 payload 中出现 `app_name: SodaMusic` 以及零散 `Qt6`/`CEF` 字节，但未能从 stub 可靠确定最终 UI 框架，因此 Qt/CEF 结论暂记为未定。
- 工作区已提供 `sodam-main.zip`，解压到 `sodam/`。压缩包没有 `.git` 目录，因此本地 Git 历史从导入提交开始保存；未覆盖上游历史。
- REA MCP/CLI 未在当前 Codex 环境注册，REA 仓库只能作为公开工具说明参考，未执行未提供的伪造命令。没有对 DRM、付费访问控制或安全保护进行绕过。

## SodaM 现有实现核对

| 模块 | 当前实现 | 本次核对结论 |
|---|---|---|
| UI | GPUI 原生窗口、首页/推荐、搜索、歌单、艺人、专辑、设置、登录、歌词、播放队列 | 可保留 |
| 领域层 | `sodam-core`，配置、会话、库、队列、历史、下载、本地库 | 可保留 |
| 网络/账号 | `libresoda` 会话与扫码登录流程 | 依赖真实账号和网络才能运行验证 |
| 播放 | rodio 专属线程，PipeWire/PulseAudio 经 ALSA/CPAL，进度/错误快照 | 已有后台播放与无设备容错 |
| 预加载 | 后台任务、并发上限、请求去重、失败 60 秒冷却、缓存命中优先、任务完成立即补位 | 已覆盖主要卡顿来源；队列/账号切换使用代际令牌使旧结果失效，底层阻塞请求自然结束 |
| 缓存 | 音频/歌词/封面缓存、LRU 清理、离线模式、缓存索引后台扫描 | 可保留 |
| 桌面集成 | MPRIS、SNI 托盘、桌面歌词、GNOME 顶栏歌词扩展、Wayland/X11 特性开关 | 已实现，需桌面会话实机验证 |

## Windows/Linux 功能差异（当前证据范围）

| 能力 | Windows 官方 Qt 客户端 | SodaM Linux |
|---|---|---|
| 程序框架 | NSIS 安装 stub；最终 UI 框架尚未由静态证据确定 | Rust + GPUI 原生 |
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
- `packaging/verify-deb.sh`：验证包元数据、依赖、二进制、desktop 文件、GNOME 扩展和 SHA-256；当前产物验证通过。
- 真实账号登录、在线搜索、播放、歌词和连续切歌尚未在本环境完成：缺少授权账号、网络服务可达性和可启动的 Wayland/音频桌面会话。

## 启动烟测

- 当前容器设置了 `WAYLAND_DISPLAY=wayland-0`，但没有可用 Wayland compositor；已加入启动预检，release 现在会输出明确错误并退出，不再触发 GPUI `NoCompositor` panic。
- 去掉 `WAYLAND_DISPLAY` 后，X11 socket `/tmp/.X11-unix/X0` 也不可用；启动预检会在进入 GPUI 前报告 X11 不可用。
- 因此已验证二进制能够进入 GPUI 平台初始化路径，但无法在本容器完成窗口绘制、登录、播放和桌面媒体控制验证。

## REA 可用性复核

- 当前主机有 Node.js 24.16.0 / npm 11.13.0。
- 官方 REA 文档给出的安装入口是 `npx rea-agents setup`，CLI 入口是 `rea`；当前 npm 缓存没有 `rea-agents`，外部 registry DNS/网络不可达，`npx --offline --yes rea-agents --version` 返回 `ENOTCACHED`。
- Codex 当前工具清单也未注册 REA MCP，因此本轮继续保持“未执行动态 REA 分析”的证据边界。

## 安装包 PE 证据补充

对 `SodaMusic-v3.7.0-official-win32_x64.exe` 的 `objdump -x` 结果：

- PE 为 GUI 可执行文件，导入表位于 `.rdata`，可见导入主要来自 `KERNEL32.dll`、`USER32.dll`、`GDI32.dll`、`SHELL32.dll`、`ADVAPI32.dll`、`COMCTL32.dll`、`ole32.dll`。
- CLR Runtime Header 为零，未显示 .NET 程序特征。
- stub 中没有 `Electron`、`WebView2`、`Qt6Core`、`Qt6Gui`、`Qt6Widgets`、`libcef.dll` 或 `node.dll` 的明确导入；这些结论只适用于安装器 stub，不能代表解压后的客户端 payload。
