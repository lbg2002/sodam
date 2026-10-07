//! 设置页入口。
//!
//! 大部分既有设置继续由 legacy 实现；常规与桌面两个板块在这里使用更紧凑的
//! 下拉选择器，并加入 Ubuntu/GNOME 顶栏歌词设置。这样避免为了几个交互改动
//! 重写整个成熟设置页。

use super::*;
use crate::app::SettingsSection;
use crate::ui::i18n::Language;
use std::sync::atomic::{AtomicBool, Ordering};

#[path = "settings_legacy.rs"]
mod legacy;

static THEME_OPEN: AtomicBool = AtomicBool::new(false);
static LANGUAGE_OPEN: AtomicBool = AtomicBool::new(false);
static DESKTOP_OPACITY_OPEN: AtomicBool = AtomicBool::new(false);
static DESKTOP_FONT_OPEN: AtomicBool = AtomicBool::new(false);
static DESKTOP_COLOR_OPEN: AtomicBool = AtomicBool::new(false);
static DESKTOP_ALIGN_OPEN: AtomicBool = AtomicBool::new(false);
static PANEL_POSITION_OPEN: AtomicBool = AtomicBool::new(false);

fn nav_item(
    root: &Root,
    section: SettingsSection,
    label: &str,
    icon: &str,
    cx: &mut Context<Root>,
) -> AnyElement {
    let selected = root.settings_section == section;
    let label = label.to_string();
    let icon = icon.to_string();
    div()
        .id(gpui::ElementId::Name(
            format!("settings-section-{section:?}").into(),
        ))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(theme::space::MD))
        .h(px(theme::size::CONTROL))
        .px(px(theme::space::MD))
        .rounded(px(theme::radius::ROW))
        .cursor_pointer()
        .when(selected, |this| this.bg(theme::surface_selected()))
        .hover(|style| style.bg(theme::surface_hover()))
        .on_click(cx.listener(move |root, _event: &ClickEvent, _window, cx| {
            root.settings_section = section;
            if matches!(section, SettingsSection::Storage) {
                root.refresh_cache_stats(cx);
                root.refresh_audio_cache_index(cx);
            }
            cx.notify();
        }))
        .child(
            svg()
                .path(icons::path(&icon))
                .size(px(theme::ICON_SM))
                .text_color(if selected {
                    theme::accent()
                } else {
                    theme::text_muted()
                }),
        )
        .child(
            div()
                .text_size(theme::Text::Small.size())
                .when(selected, |this| {
                    this.font_weight(gpui::FontWeight::SEMIBOLD)
                })
                .text_color(if selected {
                    theme::text()
                } else {
                    theme::text_muted()
                })
                .child(label),
        )
        .into_any_element()
}

fn heading(title: impl Into<String>, description: impl Into<String>) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(theme::space::XS))
        .child(
            div()
                .text_size(theme::Text::Large.size())
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme::text())
                .child(title.into()),
        )
        .child(
            div()
                .text_size(theme::Text::Small.size())
                .text_color(theme::text_muted())
                .child(description.into()),
        )
        .into_any_element()
}

fn toggle_row(
    id: &'static str,
    title: String,
    description: String,
    enabled: bool,
    listener: impl Fn(&ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> AnyElement {
    div()
        .id(id)
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(px(theme::space::LG))
        .px(px(theme::space::MD))
        .py(px(theme::space::SM))
        .rounded(px(theme::radius::ROW))
        .cursor_pointer()
        .hover(|style| style.bg(theme::surface_hover()))
        .on_click(listener)
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(theme::space::XS))
                .child(
                    div()
                        .text_size(theme::Text::Body.size())
                        .text_color(theme::text())
                        .child(title),
                )
                .child(
                    div()
                        .text_size(theme::Text::Tiny.size())
                        .text_color(theme::text_muted())
                        .child(description),
                ),
        )
        .child(
            div()
                .w(px(40.0))
                .h(px(22.0))
                .rounded(px(theme::radius::PILL))
                .bg(if enabled {
                    theme::accent()
                } else {
                    theme::surface_hover()
                })
                .flex()
                .items_center()
                .px(px(3.0))
                .justify_end()
                .when(!enabled, |this| this.justify_start())
                .child(div().size(px(16.0)).rounded_full().bg(if enabled {
                    theme::accent_foreground()
                } else {
                    theme::text_faint()
                })),
        )
        .into_any_element()
}

fn option_row(
    id: String,
    title: String,
    selected: bool,
    listener: impl Fn(&ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> AnyElement {
    div()
        .id(gpui::ElementId::Name(id.into()))
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .px(px(theme::space::MD))
        .py(px(theme::space::SM))
        .rounded(px(theme::radius::ROW))
        .cursor_pointer()
        .when(selected, |this| this.bg(theme::surface_selected()))
        .hover(|style| style.bg(theme::surface_hover()))
        .on_click(listener)
        .child(
            div()
                .text_size(theme::Text::Body.size())
                .text_color(theme::text())
                .child(title),
        )
        .when(selected, |this| {
            this.child(
                svg()
                    .path(icons::path("check"))
                    .size(px(theme::ICON_SM))
                    .text_color(theme::accent()),
            )
        })
        .into_any_element()
}

fn dropdown(
    id: &'static str,
    value: String,
    open: &'static AtomicBool,
    options: Vec<AnyElement>,
    cx: &mut Context<Root>,
) -> AnyElement {
    let expanded = open.load(Ordering::Relaxed);
    div()
        .flex()
        .flex_col()
        .gap(px(theme::space::XS))
        .child(
            div()
                .id(id)
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .h(px(theme::size::CONTROL))
                .px(px(theme::space::MD))
                .rounded(px(theme::radius::ROW))
                .bg(theme::surface_elevated())
                .border_1()
                .border_color(if expanded {
                    theme::accent()
                } else {
                    theme::border()
                })
                .cursor_pointer()
                .hover(|style| style.bg(theme::surface_hover()))
                .on_click(cx.listener(move |_root, _event: &ClickEvent, _window, cx| {
                    open.store(!open.load(Ordering::Relaxed), Ordering::Relaxed);
                    cx.notify();
                }))
                .child(
                    div()
                        .text_size(theme::Text::Body.size())
                        .text_color(theme::text())
                        .child(value),
                )
                .child(
                    svg()
                        .path(icons::path(if expanded {
                            "chevron-left"
                        } else {
                            "chevron-right"
                        }))
                        .size(px(theme::ICON_SM))
                        .text_color(theme::text_muted()),
                ),
        )
        .when(expanded, |this| {
            this.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .p(px(theme::space::XS))
                    .rounded(px(theme::radius::CARD))
                    .bg(theme::surface())
                    .border_1()
                    .border_color(theme::border())
                    .children(options),
            )
        })
        .into_any_element()
}

fn shell(root: &Root, body: AnyElement, cx: &mut Context<Root>) -> AnyElement {
    let nav = div()
        .flex()
        .flex_col()
        .flex_none()
        .w(px(180.0))
        .gap(px(theme::space::XS))
        .p(px(theme::space::SM))
        .rounded(px(theme::radius::CARD))
        .bg(theme::surface())
        .border_1()
        .border_color(theme::border())
        .child(nav_item(
            root,
            SettingsSection::General,
            root.tr("常规"),
            "palette",
            cx,
        ))
        .child(nav_item(
            root,
            SettingsSection::Playback,
            root.tr("播放"),
            "music",
            cx,
        ))
        .child(nav_item(
            root,
            SettingsSection::Lyrics,
            root.tr("歌词"),
            "captions",
            cx,
        ))
        .child(nav_item(
            root,
            SettingsSection::Downloads,
            root.tr("下载"),
            "download",
            cx,
        ))
        .child(nav_item(
            root,
            SettingsSection::Storage,
            root.tr("存储"),
            "hard-drive",
            cx,
        ))
        .child(nav_item(
            root,
            SettingsSection::Desktop,
            root.tr("桌面"),
            "captions",
            cx,
        ))
        .child(nav_item(
            root,
            SettingsSection::Advanced,
            root.tr("高级"),
            "settings",
            cx,
        ))
        .child(nav_item(
            root,
            SettingsSection::Account,
            root.tr("账户"),
            "user-round",
            cx,
        ));

    div()
        .id("settings-layout")
        .flex()
        .flex_row()
        .flex_1()
        .min_h(px(0.0))
        .gap(px(theme::space::LG))
        .child(nav)
        .child(
            div()
                .id("settings-section-scroll")
                .flex()
                .flex_col()
                .flex_1()
                .min_w(px(0.0))
                .min_h(px(0.0))
                .overflow_y_scroll()
                .pr(px(theme::space::SM))
                .child(body),
        )
        .into_any_element()
}

fn general_view(root: &Root, cx: &mut Context<Root>) -> AnyElement {
    let theme_value = if root.theme_follows_system {
        root.tr("跟随系统").to_string()
    } else if root.theme == ThemeKind::Light {
        root.tr("浅色").to_string()
    } else {
        root.tr("深色").to_string()
    };
    let theme_options = [
        (None, root.tr("跟随系统").to_string()),
        (Some(ThemeKind::Dark), root.tr("深色").to_string()),
        (Some(ThemeKind::Light), root.tr("浅色").to_string()),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (value, label))| {
        let selected = match value {
            None => root.theme_follows_system,
            Some(value) => !root.theme_follows_system && root.theme == value,
        };
        option_row(
            format!("theme-dropdown-{index}"),
            label,
            selected,
            cx.listener(move |root, _event: &ClickEvent, window, cx| {
                match value {
                    Some(value) => root.set_ui_theme(value, cx),
                    None => {
                        root.theme_follows_system = true;
                        root.settings.theme.clear();
                        root.theme = theme::theme_from_appearance(window.appearance());
                        theme::set_theme(root.theme);
                        let _ = root.settings.save();
                        cx.notify();
                    }
                }
                THEME_OPEN.store(false, Ordering::Relaxed);
            }),
        )
    })
    .collect::<Vec<_>>();

    let language_value = if root.language_follows_system {
        root.tr("跟随系统").to_string()
    } else if root.language == Language::English {
        "English".to_string()
    } else {
        "中文".to_string()
    };
    let language_options = [
        (Language::System, root.tr("跟随系统").to_string()),
        (Language::Chinese, "中文".to_string()),
        (Language::English, "English".to_string()),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (value, label))| {
        let selected = if root.language_follows_system {
            value == Language::System
        } else {
            value != Language::System && root.language == value
        };
        option_row(
            format!("language-dropdown-{index}"),
            label,
            selected,
            cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                if value == Language::System {
                    root.language = Language::system_locale();
                    root.language_follows_system = true;
                    root.settings.language = "auto".to_string();
                    let _ = root.settings.save();
                    cx.notify();
                } else {
                    root.set_language(value, cx);
                }
                LANGUAGE_OPEN.store(false, Ordering::Relaxed);
            }),
        )
    })
    .collect::<Vec<_>>();

    let body = div()
        .flex()
        .flex_col()
        .gap(px(theme::space::XL))
        .child(heading(
            root.tr("主题"),
            if root.theme_follows_system {
                root.tr("当前跟随系统偏好，手动选择后会固定主题")
            } else {
                root.tr("已固定主题；可随时切换深色或浅色")
            },
        ))
        .child(dropdown(
            "theme-dropdown",
            theme_value,
            &THEME_OPEN,
            theme_options,
            cx,
        ))
        .child(heading(
            root.tr("语言"),
            if root.language_follows_system {
                root.tr("当前跟随系统语言；手动选择后会固定语言")
            } else {
                root.tr("已固定语言；可随时切换中文或 English")
            },
        ))
        .child(dropdown(
            "language-dropdown",
            language_value,
            &LANGUAGE_OPEN,
            language_options,
            cx,
        ))
        .into_any_element();
    shell(root, body, cx)
}

fn desktop_view(root: &Root, cx: &mut Context<Root>) -> AnyElement {
    let opacity = root.settings.desktop_lyrics_opacity;
    let opacity_options = [60u8, 75, 86, 100]
        .into_iter()
        .map(|value| {
            option_row(
                format!("desktop-opacity-{value}"),
                format!("{value}%"),
                opacity == value,
                cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                    root.e3_set_desktop_opacity(value, cx);
                    DESKTOP_OPACITY_OPEN.store(false, Ordering::Relaxed);
                }),
            )
        })
        .collect();

    let font_size = root.settings.desktop_lyrics_font_size;
    let font_options = [24u32, 30, 36, 42]
        .into_iter()
        .map(|value| {
            option_row(
                format!("desktop-font-{value}"),
                format!("{value} px"),
                font_size == value,
                cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                    root.e3_set_desktop_font_size(value, cx);
                    DESKTOP_FONT_OPEN.store(false, Ordering::Relaxed);
                }),
            )
        })
        .collect();

    let color_value = if root.settings.desktop_lyrics_color == "text" {
        root.tr("跟随主题文字色").to_string()
    } else {
        root.tr("跟随强调色").to_string()
    };
    let color_options = [
        ("accent", root.tr("跟随强调色")),
        ("text", root.tr("跟随主题文字色")),
    ]
    .into_iter()
    .map(|(value, label)| {
        option_row(
            format!("desktop-color-{value}"),
            label.to_string(),
            root.settings.desktop_lyrics_color == value,
            cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                root.e3_set_desktop_color(value, cx);
                DESKTOP_COLOR_OPEN.store(false, Ordering::Relaxed);
            }),
        )
    })
    .collect();

    let align_value = match root.settings.desktop_lyrics_align.as_str() {
        "left" => root.tr("左对齐"),
        "right" => root.tr("右对齐"),
        _ => root.tr("居中"),
    }
    .to_string();
    let align_options = [
        ("left", root.tr("左对齐")),
        ("center", root.tr("居中")),
        ("right", root.tr("右对齐")),
    ]
    .into_iter()
    .map(|(value, label)| {
        option_row(
            format!("desktop-align-{value}"),
            label.to_string(),
            root.settings.desktop_lyrics_align == value,
            cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                root.e3_set_desktop_align(value, cx);
                DESKTOP_ALIGN_OPEN.store(false, Ordering::Relaxed);
            }),
        )
    })
    .collect();

    let panel_position = match root.settings.panel_lyrics_position.as_str() {
        "left" => root.tr("左对齐").to_string(),
        "center-right" => format!("{} · {}", root.tr("居中"), root.tr("右对齐")),
        "right" => root.tr("右对齐").to_string(),
        _ => format!("{} · {}", root.tr("居中"), root.tr("左对齐")),
    };
    let panel_options = [
        ("left", root.tr("左对齐").to_string()),
        (
            "center-left",
            format!("{} · {}", root.tr("居中"), root.tr("左对齐")),
        ),
        (
            "center-right",
            format!("{} · {}", root.tr("居中"), root.tr("右对齐")),
        ),
        ("right", root.tr("右对齐").to_string()),
    ]
    .into_iter()
    .map(|(value, label)| {
        option_row(
            format!("panel-position-{value}"),
            label,
            root.settings.panel_lyrics_position == value,
            cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                root.settings.panel_lyrics_position = value.to_string();
                let _ = root.settings.save();
                PANEL_POSITION_OPEN.store(false, Ordering::Relaxed);
                cx.notify();
            }),
        )
    })
    .collect();

    let body = div()
        .flex()
        .flex_col()
        .gap(px(theme::space::XL))
        .child(heading(
            root.tr("桌面歌词"),
            root.tr("调整独立歌词窗口的显示方式、透明度和交互"),
        ))
        .child(toggle_row(
            "desktop-single-line",
            root.tr("单行歌词").to_string(),
            root.tr("关闭时显示当前行和下一行").to_string(),
            root.settings.desktop_lyrics_single_line,
            cx.listener(|root, _event: &ClickEvent, _window, cx| {
                root.e3_set_desktop_single_line(!root.settings.desktop_lyrics_single_line, cx);
            }),
        ))
        .child(toggle_row(
            "desktop-always-top",
            root.tr("始终置顶").to_string(),
            root.tr("桌面歌词窗口保持在其他窗口上方").to_string(),
            root.settings.desktop_lyrics_always_on_top,
            cx.listener(|root, _event: &ClickEvent, _window, cx| {
                root.e3_set_desktop_always_on_top(!root.settings.desktop_lyrics_always_on_top, cx);
            }),
        ))
        .child(toggle_row(
            "desktop-lock",
            root.tr("锁定位置").to_string(),
            root.tr("锁定后禁止拖动和调整窗口尺寸").to_string(),
            root.settings.desktop_lyrics_locked,
            cx.listener(|root, _event: &ClickEvent, _window, cx| {
                root.e3_set_desktop_locked(!root.settings.desktop_lyrics_locked, cx);
            }),
        ))
        .child(toggle_row(
            "desktop-click-through",
            root.tr("鼠标穿透").to_string(),
            root.tr("受当前 Linux/窗口后端能力限制；不支持时保持普通窗口交互")
                .to_string(),
            root.settings.desktop_lyrics_click_through,
            cx.listener(|root, _event: &ClickEvent, _window, cx| {
                root.e3_set_desktop_click_through(!root.settings.desktop_lyrics_click_through, cx);
            }),
        ))
        .child(heading(
            root.tr("背景透明度"),
            root.tr("降低背景存在感，让歌词更适合悬浮在桌面"),
        ))
        .child(dropdown(
            "desktop-opacity-dropdown",
            format!("{opacity}%"),
            &DESKTOP_OPACITY_OPEN,
            opacity_options,
            cx,
        ))
        .child(heading(
            root.tr("桌面歌词字号"),
            root.tr("独立于主播放页歌词字号，适合远距离查看"),
        ))
        .child(dropdown(
            "desktop-font-dropdown",
            format!("{font_size} px"),
            &DESKTOP_FONT_OPEN,
            font_options,
            cx,
        ))
        .child(heading(
            root.tr("桌面歌词颜色"),
            root.tr("使用动态强调色，或跟随当前主题文字颜色"),
        ))
        .child(dropdown(
            "desktop-color-dropdown",
            color_value,
            &DESKTOP_COLOR_OPEN,
            color_options,
            cx,
        ))
        .child(heading(
            root.tr("歌词对齐"),
            root.tr("设置桌面歌词文字的水平对齐方式"),
        ))
        .child(dropdown(
            "desktop-align-dropdown",
            align_value,
            &DESKTOP_ALIGN_OPEN,
            align_options,
            cx,
        ))
        .child(heading(
            format!("Ubuntu / GNOME · {}", root.tr("桌面歌词")),
            root.tr("设置桌面歌词文字的水平对齐方式"),
        ))
        .child(toggle_row(
            "panel-lyrics-enabled",
            format!("GNOME · {}", root.tr("桌面歌词")),
            root.tr("桌面歌词窗口保持在其他窗口上方").to_string(),
            root.settings.panel_lyrics_enabled,
            cx.listener(|root, _event: &ClickEvent, _window, cx| {
                root.settings.panel_lyrics_enabled = !root.settings.panel_lyrics_enabled;
                let _ = root.settings.save();
                cx.notify();
            }),
        ))
        .child(dropdown(
            "panel-position-dropdown",
            panel_position,
            &PANEL_POSITION_OPEN,
            panel_options,
            cx,
        ))
        .child(heading(
            root.tr("歌词偏移快捷调整"),
            root.tr("快速提前或延后桌面歌词；与播放页歌词偏移共用"),
        ))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(theme::space::SM))
                .child(
                    div()
                        .id("desktop-offset-earlier")
                        .px(px(theme::space::MD))
                        .py(px(theme::space::SM))
                        .rounded(px(theme::radius::ROW))
                        .bg(theme::surface_elevated())
                        .cursor_pointer()
                        .hover(|style| style.bg(theme::surface_hover()))
                        .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                            root.e3_adjust_lyrics_offset(-250, cx);
                        }))
                        .child(root.tr("提前 250 ms")),
                )
                .child(
                    div()
                        .id("desktop-offset-zero")
                        .px(px(theme::space::MD))
                        .py(px(theme::space::SM))
                        .rounded(px(theme::radius::ROW))
                        .bg(theme::surface_elevated())
                        .cursor_pointer()
                        .hover(|style| style.bg(theme::surface_hover()))
                        .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                            root.set_lyrics_offset_ms(0, cx);
                        }))
                        .child(root.tr("偏移归零")),
                )
                .child(
                    div()
                        .id("desktop-offset-later")
                        .px(px(theme::space::MD))
                        .py(px(theme::space::SM))
                        .rounded(px(theme::radius::ROW))
                        .bg(theme::surface_elevated())
                        .cursor_pointer()
                        .hover(|style| style.bg(theme::surface_hover()))
                        .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                            root.e3_adjust_lyrics_offset(250, cx);
                        }))
                        .child(root.tr("延后 250 ms")),
                ),
        )
        .into_any_element();
    shell(root, body, cx)
}

pub(crate) fn duration_of(seconds: i64) -> String {
    legacy::duration_of(seconds)
}

pub(crate) fn settings_view(root: &Root, cx: &mut Context<Root>) -> AnyElement {
    match root.settings_section {
        SettingsSection::General => general_view(root, cx),
        SettingsSection::Desktop => desktop_view(root, cx),
        _ => legacy::settings_view(root, cx),
    }
}
