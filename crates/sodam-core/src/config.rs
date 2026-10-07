//! 应用配置（`~/.config/sodam/config.json`）。

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 与 libresoda / libmssdk 对接需要的全部设置。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// 签名服务地址（libmssdk 的 `/sign`，VIP 整曲必需）。
    pub signer_url: String,
    /// 签名服务 token（`Authorization: Bearer`），服务端没开鉴权时留空。
    pub signer_token: String,
    /// 登录 Cookie（扫码登录成功后写入）。
    pub cookie: String,
    /// 设备指纹：必须与签名器一致（签名器 `/config` 可查）。
    pub device_id: String,
    pub iid: String,
    pub fp: String,
    /// 播放音质偏好：`best` / `lossless` / `highest` / `medium` / `low`。
    pub quality: String,
    /// 下载音质：`follow` / `lossless` / `highest` / `medium` / `low`。
    pub download_quality: String,
    /// 下载格式：`source` / `mp3` / `flac`。
    pub download_format: String,
    /// 自定义下载目录；空 = 系统音乐目录下的 `SodaM Downloads`。
    pub download_dir: String,
    /// 歌词字号（px）。
    pub lyrics_font_size: u32,
    /// 歌词行高（px）。
    pub lyrics_line_height: u32,
    /// 歌词时间偏移（毫秒）；正值表示歌词更晚出现。
    pub lyrics_offset_ms: i64,
    /// 智能预加载前方曲目数；0 = 关闭。固定模式使用。
    pub prefetch_count: usize,
    /// 是否启用自适应预加载。
    pub prefetch_adaptive: bool,
    /// 自适应预加载目标时长（分钟）。
    pub prefetch_minutes: u32,
    /// 离线模式：只播放当前音质档位已有的本地缓存。
    pub offline_mode: bool,
    /// 播放缓存上限（GB）；0 = 不限制。
    pub cache_limit_gb: u64,
    /// 切歌系统通知。
    pub system_notifications: bool,
    /// 音频输出设备名；空 = 系统默认。
    pub audio_output_device: String,
    /// 自动响度标准化（ReplayGain-style / AGC）。
    pub normalize_volume: bool,
    /// 无缝衔接：提前准备下一首并尽量缩短曲目边界空白。
    pub gapless_playback: bool,
    /// Crossfade 时长（秒）；0 = 关闭。
    pub crossfade_seconds: u32,
    /// 启动时延迟加载非首屏内容。
    pub lazy_startup: bool,
    /// 播放栏低于该宽度时折叠次要功能。
    pub player_bar_compact_width: u32,
    /// 应用界面基础字号（px）；所有常规界面字号按该值成比例缩放。
    pub ui_font_size: u32,
    /// 桌面歌词：单行模式。
    pub desktop_lyrics_single_line: bool,
    /// 桌面歌词：始终置顶。
    pub desktop_lyrics_always_on_top: bool,
    /// 桌面歌词：锁定位置。
    pub desktop_lyrics_locked: bool,
    /// 桌面歌词：鼠标穿透（Linux 下由窗口后端能力决定）。
    pub desktop_lyrics_click_through: bool,
    /// 桌面歌词背景透明度 0~100。
    pub desktop_lyrics_opacity: u8,
    /// 桌面歌词独立字号（px）。
    pub desktop_lyrics_font_size: u32,
    /// 桌面歌词颜色：accent / text。
    pub desktop_lyrics_color: String,
    /// 桌面歌词对齐：left / center / right。
    pub desktop_lyrics_align: String,
    /// Ubuntu / GNOME 顶栏歌词开关。其他平台会忽略该设置。
    pub panel_lyrics_enabled: bool,
    /// GNOME 顶栏歌词位置：left / center-left / center-right / right。
    pub panel_lyrics_position: String,
    /// GNOME 顶栏歌词颜色：accent / text / white。默认 accent，跟随当前歌曲主题色。
    pub panel_lyrics_color: String,
    /// 迷你播放器记忆窗口坐标/尺寸；0 表示使用默认。
    pub mini_x: f32,
    pub mini_y: f32,
    pub mini_w: f32,
    pub mini_h: f32,
    /// 桌面歌词窗口记忆坐标/尺寸；0 表示使用默认。
    pub desktop_lyrics_x: f32,
    pub desktop_lyrics_y: f32,
    pub desktop_lyrics_w: f32,
    pub desktop_lyrics_h: f32,
    /// 界面主题：`dark` / `light`；空 = 第一次启动跟随系统偏好。
    pub theme: String,
    /// 界面语言：`zh` / `en`；空或 `auto` = 跟随系统语言。
    pub language: String,
}

pub const DEFAULT_SIGNER_URL: &str = "http://222.186.10.201:8921/sign";
pub const DEFAULT_SIGNER_TOKEN: &str = "05f8089b8c5f60c63f2a6dcfe1028d28ee2725a504f3e59b";

impl Default for Settings {
    fn default() -> Self {
        Self {
            signer_url: DEFAULT_SIGNER_URL.to_string(),
            signer_token: DEFAULT_SIGNER_TOKEN.to_string(),
            cookie: String::new(),
            device_id: String::new(),
            iid: String::new(),
            fp: String::new(),
            quality: String::new(),
            download_quality: "follow".to_string(),
            download_format: "source".to_string(),
            download_dir: String::new(),
            lyrics_font_size: 18,
            lyrics_line_height: 32,
            lyrics_offset_ms: 0,
            prefetch_count: 3,
            prefetch_adaptive: true,
            prefetch_minutes: 12,
            offline_mode: false,
            cache_limit_gb: 3,
            system_notifications: true,
            audio_output_device: String::new(),
            normalize_volume: false,
            gapless_playback: true,
            crossfade_seconds: 0,
            lazy_startup: true,
            player_bar_compact_width: 900,
            ui_font_size: 14,
            desktop_lyrics_single_line: false,
            desktop_lyrics_always_on_top: true,
            desktop_lyrics_locked: false,
            desktop_lyrics_click_through: false,
            desktop_lyrics_opacity: 86,
            desktop_lyrics_font_size: 30,
            desktop_lyrics_color: "accent".to_string(),
            desktop_lyrics_align: "center".to_string(),
            panel_lyrics_enabled: true,
            panel_lyrics_position: "center-left".to_string(),
            panel_lyrics_color: "accent".to_string(),
            mini_x: 0.0,
            mini_y: 0.0,
            mini_w: 420.0,
            mini_h: 148.0,
            desktop_lyrics_x: 0.0,
            desktop_lyrics_y: 0.0,
            desktop_lyrics_w: 760.0,
            desktop_lyrics_h: 150.0,
            theme: String::new(),
            language: String::new(),
        }
    }
}

impl Settings {
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("sodam")
            .join("config.json")
    }

    pub fn load() -> Self {
        Self::load_from(&Self::config_path())
    }

    pub fn load_from(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        self.save_to(&Self::config_path())
    }

    pub fn save_to(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_vec_pretty(self)?)?;
        Ok(())
    }

    pub fn is_ready_for_vip(&self) -> bool {
        !self.cookie.trim().is_empty() && !self.signer_url.trim().is_empty()
    }

    pub fn merged_with_env(mut self) -> Self {
        let pairs = [
            ("SODA_COOKIE", &mut self.cookie),
            ("QISHUI_SIGNER_URL", &mut self.signer_url),
            ("QISHUI_SIGNER_TOKEN", &mut self.signer_token),
            ("SODA_DEVICE_ID", &mut self.device_id),
            ("SODA_IID", &mut self.iid),
            ("SODA_FP", &mut self.fp),
            ("SODAM_QUALITY", &mut self.quality),
            ("SODAM_DOWNLOAD_QUALITY", &mut self.download_quality),
            ("SODAM_DOWNLOAD_FORMAT", &mut self.download_format),
            ("SODAM_DOWNLOAD_DIR", &mut self.download_dir),
            ("SODAM_AUDIO_DEVICE", &mut self.audio_output_device),
        ];
        for (key, slot) in pairs {
            if slot.trim().is_empty() {
                if let Ok(value) = std::env::var(key) {
                    if !value.trim().is_empty() {
                        *slot = value.trim().to_string();
                    }
                }
            }
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn older_config_gets_new_playback_defaults() {
        let dir = std::env::temp_dir().join(format!("sodam-old-cfg-{}", std::process::id()));
        let path = dir.join("config.json");
        std::fs::create_dir_all(&dir).expect("mkdir");
        std::fs::write(
            &path,
            r#"{
  "cookie": "sessionid_ss=x",
  "quality": "highest",
  "theme": "dark",
  "language": "zh"
}"#,
        )
        .expect("write");

        let loaded = Settings::load_from(&path);
        assert_eq!(loaded.cookie, "sessionid_ss=x");
        assert_eq!(loaded.quality, "highest");
        assert_eq!(loaded.prefetch_count, 3);
        assert!(loaded.prefetch_adaptive);
        assert_eq!(loaded.prefetch_minutes, 12);
        assert!(!loaded.offline_mode);
        assert_eq!(loaded.cache_limit_gb, 3);
        assert!(loaded.system_notifications);
        assert!(loaded.gapless_playback);
        assert_eq!(loaded.crossfade_seconds, 0);
        assert!(loaded.lazy_startup);
        assert_eq!(loaded.player_bar_compact_width, 900);
        assert_eq!(loaded.desktop_lyrics_opacity, 86);
        assert_eq!(loaded.desktop_lyrics_font_size, 30);
        assert_eq!(loaded.desktop_lyrics_color, "accent");
        assert!(loaded.panel_lyrics_enabled);
        assert_eq!(loaded.panel_lyrics_position, "center-left");
        assert_eq!(loaded.panel_lyrics_color, "accent");
        assert_eq!(loaded.download_quality, "follow");
        assert_eq!(loaded.download_format, "source");
        assert!(loaded.download_dir.is_empty());
        assert_eq!(loaded.lyrics_font_size, 18);
        assert_eq!(loaded.lyrics_line_height, 32);
        assert_eq!(loaded.lyrics_offset_ms, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn round_trip_and_defaults() {
        let dir = std::env::temp_dir().join(format!("sodam-cfg-{}", std::process::id()));
        let path = dir.join("config.json");
        let settings = Settings {
            signer_url: "http://127.0.0.1:8899/sign".into(),
            cookie: "sessionid_ss=x".into(),
            ..Default::default()
        };
        settings.save_to(&path).expect("save");

        let loaded = Settings::load_from(&path);
        assert_eq!(loaded.signer_url, settings.signer_url);
        assert_eq!(loaded.cookie, settings.cookie);
        assert_eq!(loaded.download_quality, "follow");
        assert_eq!(loaded.download_format, "source");
        assert!(loaded.download_dir.is_empty());
        assert_eq!(loaded.lyrics_font_size, 18);
        assert_eq!(loaded.lyrics_line_height, 32);
        assert_eq!(loaded.lyrics_offset_ms, 0);
        assert_eq!(loaded.prefetch_count, 3);
        assert!(loaded.prefetch_adaptive);
        assert_eq!(loaded.prefetch_minutes, 12);
        assert!(!loaded.offline_mode);
        assert_eq!(loaded.cache_limit_gb, 3);
        assert!(loaded.system_notifications);
        assert!(loaded.gapless_playback);
        assert_eq!(loaded.crossfade_seconds, 0);
        assert_eq!(loaded.audio_output_device, "");
        assert!(!loaded.normalize_volume);
        assert!(loaded.panel_lyrics_enabled);
        assert_eq!(loaded.panel_lyrics_position, "center-left");
        assert_eq!(loaded.panel_lyrics_color, "accent");
        assert!(loaded.theme.is_empty());
        assert!(loaded.language.is_empty());
        assert!(loaded.is_ready_for_vip());
        assert!(!Settings::default().is_ready_for_vip());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
