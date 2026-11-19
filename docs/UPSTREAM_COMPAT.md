# SodaM 定制版上游兼容审计（2026-11-19）

本 Fork 保留 Ubuntu 26/GNOME 顶栏歌词、桌面歌词、MPRIS、迷你播放器、自绘窗口、沉浸主题、独立播放队列、Gapless/Crossfade、缓存与下载管理等独有能力。全量合并 sodahub-org/sodam 不适合直接发布。

## 本分支已修改

- libmssdk v0.2.0: 保持原 libresoda 固定依赖，新增签名适配层，处理 429/503 Retry-After 退避、401/500 类错误、空签名与错误试听缓存。
- 默认公共签名服务由 HTTP 迁移为 HTTPS，已有旧默认地址自动迁移，自定义 URL/Token 不变。
- 设置 → 关于：仅检查自身 Fork 的 GitHub Release，校验包名、SHA256SUMS 和 dpkg 元数据，用户再次点击后由 polkit/apt 安装。仅 Debian/Ubuntu 的 deb 安装支持。
- GitHub Actions: PR/main CI 构建 amd64/arm64 deb 并验证；推送与 Cargo 版本一致的 tag 才发布 Release。修复误把架构当版本号传给打包脚本的问题。
- 版本增加为 0.1.23；本分支不自动打 tag 或发 Release。

## 仍然待办，不可宣称完成

| 上游能力 | 相关来源 | 需要的验收 |
|---|---|---|
| PC 综合搜索兼容 | libresoda #54、SodaM #210 | 现有 libresoda 固定 44df096，搜索接口须在库中移植签名与新解析，验收四类搜索及异常报告 |
| 多轨 MP4/multi-mdat/解密惰性计划 | libresoda #43/#47/#48/#51 | 成组接入，真实文件、损坏样本与音质缓存回归 |
| 纯 HTTP QR 登录/安全验证 | libresoda #50/#53、SodaM #179/#194 | 设备身份迁移、Cookie、二次验证和回退路径 |
| 配置写盘与登出防复活 | SodaM #187/#195/#196/#199/#200 | 审查定制配置的所有字段和异步写盘生命周期 |
| GPUI 窗口和中键滚动 | SodaM #171/#172/#202/#207 等 | Ubuntu Wayland 实机检验歌词、自绘窗口、迷你播放器 |

## 测试发布门槛

1. PR CI 通过 cargo fmt、clippy、test 和 deb 打包校验；失败时不可打 tag。
2. Ubuntu 真机验收 HTTPS 签名、429/503、会员整曲、离线缓存、预取、下载、Gapless/Crossfade、MPRIS、桌面/顶栏歌词、自绘窗口。
3. 用已安装 0.1.22 的 deb 设备检查到 v0.1.23，验证 SHA-256 失败拒绝安装、取消 polkit 后原软件正常、安装成功后重启。
4. 环境不具备时标记 blocked，不应声称所有测试通过。
