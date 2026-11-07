# QQ 音乐 Ubuntu 26 启动失败

## 现象

`qqmusic` 启动后立即退出，日志类似：

```text
FATAL:sandbox_host_linux.cc(39): Operation not permitted
```

本机 `qqmusic` 1.1.8 的 `chrome-sandbox` 文件具备 setuid 位，但 Ubuntu 26 默认启用了 AppArmor 对非特权 user namespace 的限制。内核日志会出现：

```text
apparmor="AUDIT" operation="userns_create" ... comm="qqmusic"
apparmor="DENIED" operation="capable" ... profile="unprivileged_userns"
```

## 推荐修复

允许本机用户创建 Electron/Chromium 所需的 user namespace，然后重新启动 QQ 音乐：

```bash
sudo sysctl -w kernel.apparmor_restrict_unprivileged_userns=0
printf 'kernel.apparmor_restrict_unprivileged_userns=0\n' | sudo tee /etc/sysctl.d/90-qqmusic-userns.conf
sudo sysctl --system
qqmusic
```

确认状态：

```bash
sysctl kernel.apparmor_restrict_unprivileged_userns
```

应该显示 `= 0`。这只放宽了 AppArmor 对用户命名空间的限制，QQ 音乐自身的 Chromium sandbox 仍然启用。

## 临时诊断

如果只想确认问题是否来自 sandbox，可以运行：

```bash
qqmusic --no-sandbox
```

这会降低 Electron 的隔离强度，只用于诊断，不建议作为日常启动方式。本机测试中由于 AppArmor 仍然拦截 user namespace，单独加 `--no-sandbox` 不能解决问题，因此应优先使用上面的 sysctl 修复。

## 前端参考边界

本机安装包的公开资源表明 QQ 音乐 Linux 是 Electron + React 客户端，使用深色主题资源、推荐卡片、封面悬停播放、播放全部按钮、歌词窗口和紧凑底部播放器。SodaM 只参考这些可观察的交互模式和布局关系，不复制 QQ 音乐的代码、私有接口或受保护内容。
