//! 桌面系统音频集成：输出设备枚举/切换与切歌通知。

use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioOutputDevice {
    pub id: String,
    pub name: String,
    pub default: bool,
}

/// Ubuntu / PipeWire-Pulse 下通过 pactl 枚举输出设备。
/// 其他平台至少返回“系统默认”，不会让设置页失效。
pub fn list_output_devices() -> Vec<AudioOutputDevice> {
    #[cfg(target_os = "linux")]
    {
        let default_name = Command::new("pactl")
            .args(["get-default-sink"])
            .output()
            .ok()
            .filter(|out| out.status.success())
            .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
            .unwrap_or_default();
        if let Ok(out) = Command::new("pactl")
            .args(["list", "short", "sinks"])
            .output()
        {
            if out.status.success() {
                let mut devices = Vec::new();
                for line in String::from_utf8_lossy(&out.stdout).lines() {
                    let mut fields = line.split('\t');
                    let _index = fields.next();
                    let Some(id) = fields.next() else { continue };
                    let driver = fields.next().unwrap_or_default();
                    let state = fields.nth(1).unwrap_or_default();
                    devices.push(AudioOutputDevice {
                        id: id.to_string(),
                        name: if driver.is_empty() {
                            id.to_string()
                        } else {
                            format!("{id} · {driver} · {state}")
                        },
                        default: id == default_name,
                    });
                }
                if !devices.is_empty() {
                    return devices;
                }
            }
        }
    }
    vec![AudioOutputDevice {
        id: String::new(),
        name: "系统默认".to_string(),
        default: true,
    }]
}

pub fn set_output_device(id: &str) -> anyhow::Result<()> {
    let requested = id.trim();
    #[cfg(target_os = "linux")]
    {
        let target = if requested.is_empty() {
            let output = Command::new("pactl").args(["get-default-sink"]).output()?;
            if !output.status.success() {
                anyhow::bail!("读取系统默认音频输出失败");
            }
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        } else {
            let status = Command::new("pactl")
                .args(["set-default-sink", requested])
                .status()?;
            if !status.success() {
                anyhow::bail!("pactl 切换输出设备失败");
            }
            requested.to_string()
        };

        if target.is_empty() {
            anyhow::bail!("系统没有可用的默认音频输出");
        }

        // 把现有播放流一起迁移到目标 sink；失败不影响默认设备设置。
        if let Ok(out) = Command::new("pactl")
            .args(["list", "short", "sink-inputs"])
            .output()
        {
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                if let Some(index) = line.split('\t').next() {
                    let _ = Command::new("pactl")
                        .args(["move-sink-input", index, target.as_str()])
                        .status();
                }
            }
        }
        return Ok(());
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = requested;
        anyhow::bail!("当前平台暂不支持应用内切换音频输出设备")
    }
}

pub fn notify_track(title: &str, artist: &str, cover: Option<&std::path::Path>) {
    #[cfg(target_os = "linux")]
    {
        let mut command = Command::new("notify-send");
        command.args([
            "-a",
            "SodaM",
            "-h",
            "string:x-canonical-private-synchronous:sodam-track",
        ]);
        if let Some(path) = cover {
            command.arg("-i").arg(path);
        }
        let _ = command.arg(title).arg(artist).spawn();
    }
}
