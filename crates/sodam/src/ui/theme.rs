//! 主题与设计 Token 系统：中式美学主题、颜色、尺寸、字号、圆角、响应式与动效参数。
//!
//! 内置五套可切换的中式美学主题（支持深色、浅色与跟随系统）：
//! 1. **黛墨夜色** (`dark` / `daimo`)：墨黑、黛蓝、暗青、朱砂色，适合夜间听歌与全屏沉浸播放
//! 2. **宋韵青瓷** (`light` / `qingci`)：青瓷绿、米白、浅灰、淡青，留白较多，适合古典、轻音乐与民谣
//! 3. **丹青山水** (`danqing`)：宣纸白、墨灰、松石绿、山水蓝，淡雅水墨层次与低对比背景
//! 4. **朱砂金** (`zhusha`)：深棕、朱砂红、暖金、米色，适合节庆、收藏、古风与戏曲风格
//! 5. **竹影清风** (`zhuying`)：竹青、月白、浅墨、灰绿，清爽克制，适合长时间使用
//!
//! 设计原则：
//! 1. **所有尺寸从同一套基准梯度派生**，行高/控件/缩略图保持稳定比例；
//! 2. **不使用大面积刺眼纯色**，底色 → 面板 → 抬升 → 悬停 → 选中采用温润步进与 1px 柔和描边；
//! 3. **封面环境色轻柔晕染（Ink & Ambient Wash）**：保留中式主题原本的明度与纸墨质感，
//!    仅按受控比例混入专辑封面主色，使切歌与沉浸背景自然过渡。

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};

use gpui::{linear_color_stop, linear_gradient, px, Background, Pixels, Rgba};

pub const fn hex(value: u32) -> Rgba {
    Rgba {
        r: ((value >> 16) & 0xFF) as f32 / 255.0,
        g: ((value >> 8) & 0xFF) as f32 / 255.0,
        b: (value & 0xFF) as f32 / 255.0,
        a: 1.0,
    }
}

pub const fn rgba_alpha(color: Rgba, alpha: f32) -> Rgba {
    Rgba {
        r: color.r,
        g: color.g,
        b: color.b,
        a: alpha,
    }
}

/// 五套中式美学主题。
///
/// 为保持向下兼容：
/// - `Dark` 对应「黛墨夜色」（默认深色，兼容 `"dark"` 与 `"daimo"`）
/// - `Light` 对应「宋韵青瓷」（默认浅色，兼容 `"light"` 与 `"qingci"`）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ThemeKind {
    /// 1. 黛墨夜色：墨黑、黛蓝、暗青、朱砂色
    Dark = 0,
    /// 2. 宋韵青瓷：青瓷绿、米白、浅灰、淡青
    Light = 1,
    /// 3. 丹青山水：宣纸白、墨灰、松石绿、山水蓝
    DanQing = 2,
    /// 4. 朱砂金：深棕、朱砂红、暖金、米色
    ZhuSha = 3,
    /// 5. 竹影清风：竹青、月白、浅墨、灰绿
    ZhuYing = 4,
}

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub bg: Rgba,
    pub bg_deep: Rgba,
    pub surface: Rgba,
    pub surface_elevated: Rgba,
    pub surface_hover: Rgba,
    pub surface_selected: Rgba,
    pub border: Rgba,
    pub text: Rgba,
    pub text_muted: Rgba,
    pub text_faint: Rgba,
    pub accent: Rgba,
    pub accent_secondary: Rgba,
    pub accent_foreground: Rgba,
    pub wash_strength: f32,
}

impl ThemeKind {
    pub const ALL: [Self; 5] = [
        Self::Dark,
        Self::Light,
        Self::DanQing,
        Self::ZhuSha,
        Self::ZhuYing,
    ];

    pub fn from_key(key: &str) -> Option<Self> {
        match key.trim().to_ascii_lowercase().as_str() {
            "dark" | "daimo" => Some(Self::Dark),
            "light" | "qingci" => Some(Self::Light),
            "danqing" => Some(Self::DanQing),
            "zhusha" => Some(Self::ZhuSha),
            "zhuying" => Some(Self::ZhuYing),
            _ => None,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
            Self::DanQing => "danqing",
            Self::ZhuSha => "zhusha",
            Self::ZhuYing => "zhuying",
        }
    }

    pub fn is_dark(self) -> bool {
        matches!(self, Self::Dark | Self::ZhuSha)
    }

    pub fn name_zh(self) -> &'static str {
        match self {
            Self::Dark => "黛墨夜色",
            Self::Light => "宋韵青瓷",
            Self::DanQing => "丹青山水",
            Self::ZhuSha => "朱砂金",
            Self::ZhuYing => "竹影清风",
        }
    }

    pub fn subtitle_zh(self) -> &'static str {
        match self {
            Self::Dark => "墨黑 · 黛蓝 · 暗青 · 朱砂",
            Self::Light => "青瓷绿 · 米白 · 浅灰 · 淡青",
            Self::DanQing => "宣纸白 · 墨灰 · 松石绿 · 山水蓝",
            Self::ZhuSha => "深棕 · 朱砂红 · 暖金 · 米色",
            Self::ZhuYing => "竹青 · 月白 · 浅墨 · 灰绿",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::DanQing,
            Self::DanQing => Self::ZhuSha,
            Self::ZhuSha => Self::ZhuYing,
            Self::ZhuYing => Self::Dark,
        }
    }

    pub fn palette(self) -> Palette {
        match self {
            // 1. 黛墨夜色：墨黑、黛蓝、暗青、朱砂色
            Self::Dark => Palette {
                bg: hex(0x0D1117),
                bg_deep: hex(0x080B0F),
                surface: hex(0x131922),
                surface_elevated: hex(0x1A2330),
                surface_hover: hex(0x222E3E),
                surface_selected: hex(0x2B3B4E),
                border: hex(0x263445),
                text: hex(0xEDEAE4),
                text_muted: hex(0xA6B0BC),
                text_faint: hex(0x768290),
                accent: hex(0xD9483B),
                accent_secondary: hex(0x3B6E8C),
                accent_foreground: hex(0xFAF7F2),
                wash_strength: 0.36,
            },
            // 2. 宋韵青瓷：青瓷绿、米白、浅灰、淡青
            Self::Light => Palette {
                bg: hex(0xF3F0E8),
                bg_deep: hex(0xEAE6DC),
                surface: hex(0xF8F6F0),
                surface_elevated: hex(0xFCFBF7),
                surface_hover: hex(0xE5ECE8),
                surface_selected: hex(0xD5E2DC),
                border: hex(0xD2DBD6),
                text: hex(0x1E2624),
                text_muted: hex(0x5A6864),
                text_faint: hex(0x7E8D88),
                accent: hex(0x3D786E),
                accent_secondary: hex(0x6C9A8B),
                accent_foreground: hex(0xF9F8F4),
                wash_strength: 0.25,
            },
            // 3. 丹青山水：宣纸白、墨灰、松石绿、山水蓝
            Self::DanQing => Palette {
                bg: hex(0xF5F1E6),
                bg_deep: hex(0xECE6D8),
                surface: hex(0xFAF7EF),
                surface_elevated: hex(0xFDFBF5),
                surface_hover: hex(0xE8E4D8),
                surface_selected: hex(0xDAD6C8),
                border: hex(0xD5D0C3),
                text: hex(0x252B30),
                text_muted: hex(0x5C646A),
                text_faint: hex(0x828A90),
                accent: hex(0x2E7468),
                accent_secondary: hex(0x3B6078),
                accent_foreground: hex(0xFAF8F2),
                wash_strength: 0.22,
            },
            // 4. 朱砂金：深棕、朱砂红、暖金、米色
            Self::ZhuSha => Palette {
                bg: hex(0x16100E),
                bg_deep: hex(0x0E0A08),
                surface: hex(0x1F1613),
                surface_elevated: hex(0x291E1A),
                surface_hover: hex(0x352621),
                surface_selected: hex(0x443029),
                border: hex(0x3D2B24),
                text: hex(0xF2E8DC),
                text_muted: hex(0xC2AFA0),
                text_faint: hex(0x8F7C6E),
                accent: hex(0xD6453A),
                accent_secondary: hex(0xD4A359),
                accent_foreground: hex(0xFDF8F2),
                wash_strength: 0.32,
            },
            // 5. 竹影清风：竹青、月白、浅墨、灰绿
            Self::ZhuYing => Palette {
                bg: hex(0xEDF3EF),
                bg_deep: hex(0xE2EBE5),
                surface: hex(0xF4F8F5),
                surface_elevated: hex(0xFAFCFA),
                surface_hover: hex(0xE0EAE3),
                surface_selected: hex(0xCFE0D5),
                border: hex(0xC9D9CE),
                text: hex(0x222C26),
                text_muted: hex(0x55665C),
                text_faint: hex(0x7A8C81),
                accent: hex(0x3A6E57),
                accent_secondary: hex(0x5E8C76),
                accent_foreground: hex(0xF7FAF8),
                wash_strength: 0.24,
            },
        }
    }

    fn from_u8(val: u8) -> Self {
        match val {
            1 => Self::Light,
            2 => Self::DanQing,
            3 => Self::ZhuSha,
            4 => Self::ZhuYing,
            _ => Self::Dark,
        }
    }

    fn from_appearance(appearance: gpui::WindowAppearance) -> Self {
        match appearance {
            gpui::WindowAppearance::Light | gpui::WindowAppearance::VibrantLight => Self::Light,
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark => Self::Dark,
        }
    }
}

/// 沉浸窗口显示模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowMode {
    /// 1. 普通窗口模式：左侧导航 + 中央内容 + 底部播放器
    Normal,
    /// 2. 半屏播放模式：自动收窄侧边栏，增大专辑封面与歌词区域，适合分屏
    HalfScreen,
    /// 3. 全屏沉浸模式：隐藏导航、标题栏与非必要控件，封面+歌词+动态渐变居中
    Fullscreen,
    /// 4. 紧凑小窗口模式：只显示封面、歌曲名、核心播放控制与进度条
    Compact,
}

impl WindowMode {
    pub const ALL: [Self; 4] = [
        Self::Normal,
        Self::HalfScreen,
        Self::Fullscreen,
        Self::Compact,
    ];

    pub fn from_key(key: &str) -> Self {
        match key.trim().to_ascii_lowercase().as_str() {
            "half" | "halfscreen" => Self::HalfScreen,
            "fullscreen" | "immersive" => Self::Fullscreen,
            "compact" | "mini" => Self::Compact,
            _ => Self::Normal,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::HalfScreen => "half",
            Self::Fullscreen => "fullscreen",
            Self::Compact => "compact",
        }
    }

    pub fn label_zh(self) -> &'static str {
        match self {
            Self::Normal => "普通模式",
            Self::HalfScreen => "半屏模式",
            Self::Fullscreen => "全屏沉浸",
            Self::Compact => "紧凑小窗",
        }
    }

    pub fn name_zh(self) -> &'static str {
        self.label_zh()
    }

    pub fn subtitle_zh(self) -> &'static str {
        match self {
            Self::Normal => "左侧导航 · 中央内容 · 底部播放",
            Self::HalfScreen => "窄侧栏 · 大封面与歌词 · 分屏友好",
            Self::Fullscreen => "水墨流光背景 · 封面歌词居中 · Esc 退出",
            Self::Compact => "精简悬浮小窗 · 核心播放控制",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Normal => Self::HalfScreen,
            Self::HalfScreen => Self::Fullscreen,
            Self::Fullscreen => Self::Compact,
            Self::Compact => Self::Normal,
        }
    }
}

/// 播放页内部视图模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackViewMode {
    /// 封面 + 歌词双栏视图
    Split,
    /// 纯享歌词居中视图
    LyricsOnly,
    /// 封面 + 播放队列视图
    QueueOnly,
}

impl PlaybackViewMode {
    pub const ALL: [Self; 3] = [Self::Split, Self::LyricsOnly, Self::QueueOnly];

    pub fn from_key(key: &str) -> Self {
        match key.trim().to_ascii_lowercase().as_str() {
            "lyrics" => Self::LyricsOnly,
            "queue" => Self::QueueOnly,
            _ => Self::Split,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Split => "split",
            Self::LyricsOnly => "lyrics",
            Self::QueueOnly => "queue",
        }
    }

    pub fn name_zh(self) -> &'static str {
        match self {
            Self::Split => "双栏视图（封面 + 歌词）",
            Self::LyricsOnly => "纯享歌词",
            Self::QueueOnly => "内嵌队列",
        }
    }
}

/// 列表与歌单/专辑内容密度。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentDensity {
    /// 大卡片 / 宽松密度
    LargeCard,
    /// 标准列表密度
    List,
    /// 紧凑列表密度
    Compact,
}

impl ContentDensity {
    pub const ALL: [Self; 3] = [Self::LargeCard, Self::List, Self::Compact];

    pub fn from_key(key: &str) -> Self {
        match key.trim().to_ascii_lowercase().as_str() {
            "card" | "large" => Self::LargeCard,
            "compact" => Self::Compact,
            _ => Self::List,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::LargeCard => "card",
            Self::List => "list",
            Self::Compact => "compact",
        }
    }

    pub fn label_zh(self) -> &'static str {
        match self {
            Self::LargeCard => "大卡片",
            Self::List => "标准列表",
            Self::Compact => "紧凑列表",
        }
    }

    pub fn name_zh(self) -> &'static str {
        self.label_zh()
    }

    pub fn row_height(self) -> f32 {
        match self {
            Self::LargeCard => 66.0,
            Self::List => size::LIST_ROW,
            Self::Compact => 44.0,
        }
    }

    pub fn card_size(self) -> f32 {
        match self {
            Self::LargeCard => 176.0,
            Self::List => size::CARD_COVER,
            Self::Compact => 132.0,
        }
    }
}

/// 歌词字体族选项。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LyricsFontFamily {
    /// 系统无衬线 / Noto Sans CJK SC
    Sans,
    /// 宋韵衬线 / Noto Serif CJK SC
    Serif,
    /// 等宽字体
    Mono,
}

impl LyricsFontFamily {
    pub const ALL: [Self; 3] = [Self::Sans, Self::Serif, Self::Mono];

    pub fn from_key(key: &str) -> Self {
        match key.trim().to_ascii_lowercase().as_str() {
            "serif" | "song" => Self::Serif,
            "mono" => Self::Mono,
            _ => Self::Sans,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Sans => "sans",
            Self::Serif => "serif",
            Self::Mono => "mono",
        }
    }

    pub fn label_zh(self) -> &'static str {
        match self {
            Self::Sans => "思源黑体",
            Self::Serif => "宋韵衬线",
            Self::Mono => "等宽韵律",
        }
    }

    pub fn name_zh(self) -> &'static str {
        self.label_zh()
    }

    pub fn font_name(self) -> &'static str {
        match self {
            Self::Sans => "Noto Sans CJK SC",
            Self::Serif => "Noto Serif CJK SC",
            Self::Mono => "Noto Sans Mono CJK SC",
        }
    }
}

/// 歌词对齐方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LyricsAlign {
    Left,
    Center,
    Right,
}

impl LyricsAlign {
    pub const ALL: [Self; 3] = [Self::Center, Self::Left, Self::Right];

    pub fn from_key(key: &str) -> Self {
        match key.trim().to_ascii_lowercase().as_str() {
            "left" => Self::Left,
            "right" => Self::Right,
            _ => Self::Center,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Center => "center",
            Self::Right => "right",
        }
    }

    pub fn label_zh(self) -> &'static str {
        match self {
            Self::Left => "左对齐",
            Self::Center => "居中对齐",
            Self::Right => "右对齐",
        }
    }

    pub fn name_zh(self) -> &'static str {
        self.label_zh()
    }
}

static CURRENT_THEME: AtomicU8 = AtomicU8::new(0);
static UI_FONT_SIZE: AtomicU32 = AtomicU32::new(14);
static REDUCED_MOTION: AtomicBool = AtomicBool::new(false);
// bit31 表示存在主色；低 24 位是 RGB。渲染线程每帧设置，后台只传值不读全局。
static AMBIENT_COLOR: AtomicU32 = AtomicU32::new(0);

const BORDER_TINT: f32 = 0.25;
const TEXT_TINT: f32 = 0.08;
const MAX_WASH_SATURATION: f32 = 0.52;

pub fn set_theme(theme: ThemeKind) {
    CURRENT_THEME.store(theme as u8, Ordering::Relaxed);
}

pub fn current_theme() -> ThemeKind {
    ThemeKind::from_u8(CURRENT_THEME.load(Ordering::Relaxed))
}

pub fn set_ui_font_size(size: u32) {
    UI_FONT_SIZE.store(size.clamp(12, 20), Ordering::Relaxed);
}

pub fn ui_font_size() -> u32 {
    UI_FONT_SIZE.load(Ordering::Relaxed).clamp(12, 20)
}

pub fn set_reduced_motion(reduced: bool) {
    REDUCED_MOTION.store(reduced, Ordering::Relaxed);
}

pub fn reduced_motion() -> bool {
    REDUCED_MOTION.load(Ordering::Relaxed)
}

pub fn motion_enabled() -> bool {
    !reduced_motion()
}

pub fn ui_font_family() -> &'static str {
    "Noto Sans CJK SC"
}

pub fn theme_from_appearance(appearance: gpui::WindowAppearance) -> ThemeKind {
    ThemeKind::from_appearance(appearance)
}

pub fn set_ambient_rgb(color: Option<(u8, u8, u8)>) {
    let packed = color
        .map(|(r, g, b)| 0x8000_0000 | (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b));
    AMBIENT_COLOR.store(packed.unwrap_or(0), Ordering::Relaxed);
}

pub fn ambient_color() -> Option<Rgba> {
    let packed = AMBIENT_COLOR.load(Ordering::Relaxed);
    if packed & 0x8000_0000 == 0 {
        return None;
    }
    Some(Rgba {
        r: ((packed >> 16) & 0xFF) as f32 / 255.0,
        g: ((packed >> 8) & 0xFF) as f32 / 255.0,
        b: (packed & 0xFF) as f32 / 255.0,
        a: 1.0,
    })
}

type Hsl = (f32, f32, f32);

fn rgba_to_hsl(color: Rgba) -> Hsl {
    let max = color.r.max(color.g).max(color.b);
    let min = color.r.min(color.g).min(color.b);
    let delta = max - min;
    let lightness = (max + min) / 2.0;
    let saturation = if delta == 0.0 {
        0.0
    } else {
        delta / (1.0 - (2.0 * lightness - 1.0).abs())
    };
    let hue = if delta == 0.0 {
        0.0
    } else if max == color.r {
        ((color.g - color.b) / delta).rem_euclid(6.0) / 6.0
    } else if max == color.g {
        ((color.b - color.r) / delta + 2.0) / 6.0
    } else {
        ((color.r - color.g) / delta + 4.0) / 6.0
    };
    (hue, saturation, lightness)
}

fn hsl_to_rgba(hue: f32, saturation: f32, lightness: f32, alpha: f32) -> Rgba {
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let hue = (hue * 6.0).rem_euclid(6.0);
    let x = chroma * (1.0 - ((hue % 2.0) - 1.0).abs());
    let (red, green, blue) = match hue as u32 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let shift = lightness - chroma / 2.0;
    Rgba {
        r: (red + shift).clamp(0.0, 1.0),
        g: (green + shift).clamp(0.0, 1.0),
        b: (blue + shift).clamp(0.0, 1.0),
        a: alpha,
    }
}

pub fn mix_rgba(from: Rgba, to: Rgba, delta: f32) -> Rgba {
    let delta = delta.clamp(0.0, 1.0);
    Rgba {
        r: from.r + (to.r - from.r) * delta,
        g: from.g + (to.g - from.g) * delta,
        b: from.b + (to.b - from.b) * delta,
        a: from.a + (to.a - from.a) * delta,
    }
}

/// 保持中式主题固有明度与气韵，适度混入封面环境色。
fn tint(base: Rgba, strength: f32) -> Rgba {
    let Some(ambient) = ambient_color() else {
        return base;
    };
    let (base_hue, base_saturation, base_lightness) = rgba_to_hsl(base);
    let (ambient_hue, ambient_saturation, _) = rgba_to_hsl(ambient);
    let blended_hue = base_hue + (ambient_hue - base_hue) * (strength * 0.55).clamp(0.0, 1.0);
    let saturation = (base_saturation * 0.72 + ambient_saturation * strength * 0.45)
        .clamp(0.0, MAX_WASH_SATURATION);
    let washed = hsl_to_rgba(
        blended_hue.rem_euclid(1.0),
        saturation,
        base_lightness,
        base.a,
    );
    mix_rgba(base, washed, (strength * 0.65).clamp(0.0, 0.55))
}

pub fn is_dark() -> bool {
    current_theme().is_dark()
}

fn current_palette() -> Palette {
    current_theme().palette()
}

// ---------------------------------------------------------------------------
// 颜色 Token
// ---------------------------------------------------------------------------

/// 窗口/侧边/播放条底色。
pub fn bg() -> Rgba {
    let p = current_palette();
    tint(p.bg, p.wash_strength)
}

/// 内容区背景。
pub fn surface() -> Rgba {
    let p = current_palette();
    tint(p.surface, p.wash_strength)
}

/// 抬升面：搜索框、卡片、徽章。
pub fn surface_elevated() -> Rgba {
    let p = current_palette();
    tint(p.surface_elevated, p.wash_strength * 0.85)
}

/// 行悬停。
pub fn surface_hover() -> Rgba {
    let p = current_palette();
    tint(p.surface_hover, p.wash_strength * 0.85)
}

/// 选中行。
pub fn surface_selected() -> Rgba {
    let p = current_palette();
    tint(p.surface_selected, p.wash_strength * 0.9)
}

/// 1px 描边。
pub fn border() -> Rgba {
    let p = current_palette();
    tint(p.border, BORDER_TINT)
}

/// 主文本。
pub fn text() -> Rgba {
    let p = current_palette();
    tint(p.text, TEXT_TINT)
}

/// 次要文本。
pub fn text_muted() -> Rgba {
    let p = current_palette();
    tint(p.text_muted, TEXT_TINT)
}

/// 三级文本。
pub fn text_faint() -> Rgba {
    let p = current_palette();
    tint(p.text_faint, TEXT_TINT)
}

/// 主题主强调色（兼顾封面色彩与中式主题印章/青瓷/松石主色）。
pub fn accent() -> Rgba {
    let p = current_palette();
    let Some(ambient) = ambient_color() else {
        return p.accent;
    };
    let (hue, sat, _) = rgba_to_hsl(ambient);
    let target_l = if p.bg.r < 0.3 { 0.66 } else { 0.38 };
    let cover_accent = hsl_to_rgba(hue, sat.clamp(0.45, 0.78), target_l, 1.0);
    // 保留 65% 主题特征色 + 35% 封面联动色，防止刺眼纯色破坏中式美感
    mix_rgba(p.accent, cover_accent, 0.35)
}

/// 主色上的前景文字/图标颜色。
pub fn accent_foreground() -> Rgba {
    current_palette().accent_foreground
}

/// 强调色的低透明底（印章淡彩 / 胶囊选中背景）。
pub fn accent_soft() -> Rgba {
    let mut color = accent();
    color.a = if is_dark() { 0.18 } else { 0.14 };
    color
}

/// 歌词高亮行/水墨光晕底色。
pub fn ink_glow() -> Rgba {
    let mut color = accent();
    color.a = if is_dark() { 0.12 } else { 0.09 };
    color
}

/// 透明 Rgba（与其它颜色 token 同类型，方便 `if/else` 混用）。
pub const fn transparent() -> Rgba {
    rgba_alpha(hex(0x000000), 0.0)
}

/// 警告/错误朱砂红色。
pub fn danger() -> Rgba {
    if is_dark() {
        hex(0xE06055)
    } else {
        hex(0xB83B32)
    }
}

/// 全局自适应背景：中式底色与封面主色轻柔晕染的线性渐变。
pub fn ambient_background() -> Background {
    let p = current_palette();
    let from = tint(p.bg, p.wash_strength);
    let to = mix_rgba(tint(p.bg_deep, p.wash_strength * 1.15), p.surface, 0.35);
    linear_gradient(
        135.0,
        linear_color_stop(from, 0.0),
        linear_color_stop(to, 1.0),
    )
}

/// 沉浸播放页与全屏模式的深层动态背景：更强的封面色晕染与水墨层次。
pub fn immersive_background() -> Background {
    let p = current_palette();
    let ambient_boost = (p.wash_strength * 1.65).clamp(0.28, 0.62);
    let top = if let Some(ambient) = ambient_color() {
        let (h, s, _) = rgba_to_hsl(ambient);
        let l = if is_dark() { 0.14 } else { 0.88 };
        let cover_tone = hsl_to_rgba(h, s.clamp(0.25, 0.60), l, 1.0);
        mix_rgba(p.bg, cover_tone, ambient_boost)
    } else {
        mix_rgba(
            p.bg,
            p.accent_secondary,
            if is_dark() { 0.16 } else { 0.10 },
        )
    };
    let bottom = tint(p.bg_deep, ambient_boost * 0.8);
    linear_gradient(
        155.0,
        linear_color_stop(top, 0.0),
        linear_color_stop(bottom, 1.0),
    )
}

// ---------------------------------------------------------------------------
// 尺寸与响应式断点
// ---------------------------------------------------------------------------

/// 标准侧边栏宽度。
pub const SIDEBAR_W: f32 = 216.0;
/// 半屏 / 窄窗紧凑侧边栏宽度（图标栏）。
pub const SIDEBAR_COMPACT_W: f32 = 64.0;
/// 顶部工具条 / 抽屉头高度。
pub const TOOLBAR_H: f32 = 48.0;
/// 底部播放条高度。
pub const PLAYER_H: f32 = 76.0;

/// 响应式窗口宽度断点。
pub mod breakpoints {
    /// 超窄/竖屏窗口（隐藏侧边栏，切换顶部/底部紧凑导航，堆叠播放页）。
    pub const NARROW_W: f32 = 680.0;
    /// 半屏/4:3/紧凑窗口（自动收窄侧边栏为图标轨，精简播放栏次要按钮）。
    pub const COMPACT_W: f32 = 980.0;
}

/// 图标尺寸：导航与按钮统一 16，播放控制 20。
pub const ICON_SM: f32 = 14.0;
pub const ICON: f32 = 16.0;
pub const ICON_LG: f32 = 20.0;
pub const ICON_XL: f32 = 24.0;

/// 间距梯度：4 / 8 / 12 / 16 / 24 / 32。
pub mod space {
    pub const XS: f32 = 4.0;
    pub const SM: f32 = 8.0;
    pub const MD: f32 = 12.0;
    pub const LG: f32 = 16.0;
    pub const XL: f32 = 24.0;
    pub const XXL: f32 = 32.0;
}

/// 行高与控件尺寸梯度。
pub mod size {
    /// 导航行。
    pub const NAV_ROW: f32 = 40.0;
    /// 列表行（歌曲/歌单）。
    pub const LIST_ROW: f32 = 56.0;
    /// 小控件（图标按钮）。
    pub const CONTROL_SM: f32 = 32.0;
    /// 常规控件（按钮/输入框）。
    pub const CONTROL: f32 = 36.0;
    /// 列表行封面。
    pub const ROW_ART: f32 = 40.0;
    /// 播放条封面。
    pub const PLAYER_THUMB: f32 = 48.0;
    /// 歌单卡片封面。
    pub const CARD_COVER: f32 = 156.0;
}

/// 圆角梯度：6 / 8 / 12 / 999。
pub mod radius {
    /// 封面（小图）。
    pub const ART: f32 = 6.0;
    /// 列表行 / 输入框。
    pub const ROW: f32 = 8.0;
    /// 卡片。
    pub const CARD: f32 = 12.0;
    /// 胶囊按钮。
    pub const PILL: f32 = 999.0;
}

/// 字号梯度（相对 `BASE` 的比例）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Text {
    Tiny,
    Small,
    Body,
    Large,
    Title,
}

impl Text {
    pub fn ratio(self) -> f32 {
        match self {
            Text::Tiny => 0.77,
            Text::Small => 0.85,
            Text::Body => 1.0,
            Text::Large => 1.38,
            Text::Title => 1.69,
        }
    }

    pub fn size(self) -> Pixels {
        px((ui_font_size() as f32 * self.ratio()).round())
    }
}

pub fn nav_row() -> Pixels {
    px(size::NAV_ROW)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_five_chinese_themes() {
        assert_eq!(ThemeKind::from_key("dark"), Some(ThemeKind::Dark));
        assert_eq!(ThemeKind::from_key("daimo"), Some(ThemeKind::Dark));
        assert_eq!(ThemeKind::from_key("light"), Some(ThemeKind::Light));
        assert_eq!(ThemeKind::from_key("qingci"), Some(ThemeKind::Light));
        assert_eq!(ThemeKind::from_key("danqing"), Some(ThemeKind::DanQing));
        assert_eq!(ThemeKind::from_key("zhusha"), Some(ThemeKind::ZhuSha));
        assert_eq!(ThemeKind::from_key("zhuying"), Some(ThemeKind::ZhuYing));
        assert_eq!(ThemeKind::from_key("system"), None);

        assert!(ThemeKind::Dark.is_dark());
        assert!(ThemeKind::ZhuSha.is_dark());
        assert!(!ThemeKind::Light.is_dark());
        assert!(!ThemeKind::DanQing.is_dark());
        assert!(!ThemeKind::ZhuYing.is_dark());
    }

    #[test]
    fn parses_window_and_playback_modes() {
        assert_eq!(WindowMode::from_key("normal"), WindowMode::Normal);
        assert_eq!(WindowMode::from_key("half"), WindowMode::HalfScreen);
        assert_eq!(WindowMode::from_key("fullscreen"), WindowMode::Fullscreen);
        assert_eq!(WindowMode::from_key("compact"), WindowMode::Compact);

        assert_eq!(PlaybackViewMode::from_key("split"), PlaybackViewMode::Split);
        assert_eq!(
            PlaybackViewMode::from_key("lyrics"),
            PlaybackViewMode::LyricsOnly
        );
        assert_eq!(
            PlaybackViewMode::from_key("queue"),
            PlaybackViewMode::QueueOnly
        );

        assert_eq!(ContentDensity::from_key("card"), ContentDensity::LargeCard);
        assert_eq!(ContentDensity::from_key("list"), ContentDensity::List);
        assert_eq!(ContentDensity::from_key("compact"), ContentDensity::Compact);
    }
}
