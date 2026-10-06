//! 轻量响度标准化：用 ffmpeg volumedetect 计算曲目平均响度并缓存增益。
//!
//! 这不是完整 EBU R128 扫描器，但行为与 ReplayGain 的目标一致：
//! 让不同来源歌曲的主观响度更接近，避免切歌时忽大忽小。

use std::path::{Path, PathBuf};
use std::process::Command;

const TARGET_DB: f32 = -16.0;

fn sidecar(path: &Path) -> PathBuf {
    path.with_extension("gain")
}

pub fn cached_gain(path: &Path) -> Option<f32> {
    std::fs::read_to_string(sidecar(path))
        .ok()?
        .trim()
        .parse::<f32>()
        .ok()
        .map(|gain| gain.clamp(0.5, 2.0))
}

pub fn analyze_gain(path: &Path) -> anyhow::Result<f32> {
    if let Some(gain) = cached_gain(path) {
        return Ok(gain);
    }
    let output = Command::new("ffmpeg")
        .args(["-hide_banner", "-nostats", "-i"])
        .arg(path)
        .args(["-af", "volumedetect", "-f", "null", "-"])
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    let mean = stderr
        .lines()
        .find_map(|line| {
            let marker = "mean_volume:";
            let index = line.find(marker)?;
            line[index + marker.len()..]
                .trim()
                .trim_end_matches(" dB")
                .parse::<f32>()
                .ok()
        })
        .ok_or_else(|| anyhow::anyhow!("ffmpeg 未返回 mean_volume"))?;
    let db_delta = (TARGET_DB - mean).clamp(-6.0, 6.0);
    let gain = 10.0_f32.powf(db_delta / 20.0).clamp(0.5, 2.0);
    let _ = std::fs::write(sidecar(path), format!("{gain:.6}"));
    Ok(gain)
}

#[cfg(test)]
mod tests {
    #[test]
    fn db_gain_bounds_are_sane() {
        let quiet = 10.0_f32.powf(6.0 / 20.0);
        let loud = 10.0_f32.powf(-6.0 / 20.0);
        assert!(quiet <= 2.0);
        assert!(loud >= 0.5);
    }
}
