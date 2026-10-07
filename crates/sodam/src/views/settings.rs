//! 设置页：按设置语义选择合适控件，并按用户任务重新分类。

use super::*;
use crate::app::SettingsSection;
use crate::ui::i18n::Language;
use gpui::{canvas, MouseButton, MouseDownEvent, MouseMoveEvent};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

static OPEN_SELECT: AtomicU8 = AtomicU8::new(0);

type SliderPreview = fn(&mut Root, f32, &mut Context<Root>);
type SliderCommit = fn(&mut Root, &mut Context<Root>);

fn text(root: &Root, zh: &str, en: &str) -> String {
    if root.language.resolved().is_zh() {
        zh.to_string()
    } else {
        en.to_string()
    }
}

fn human_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let value = bytes as f64;
    if value >= KB * KB * KB {
        format!("{:.2} GB", value / (KB * KB * KB))
    } else if value >= KB * KB {
        format!("{:.1} MB", value / (KB * KB))
    } else if value >= KB {
        format!("{:.0} KB", value / KB)
    } else {
        format!("{bytes} B")
    }
}

/// 秒 → 「小时:分:秒 / 分:秒」。
pub(crate) fn duration_of(seconds: i64) -> String {
    let seconds = seconds.max(0);
    let (hours, minutes, secs) = (seconds / 3600, (seconds % 3600) / 60, seconds % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{secs:02}")
    } else {
        format!("{minutes}:{secs:02}")
    }
}

fn nav_item(
    root: &Root,
    section: SettingsSection,
    label: String,
    icon: &'static str,
    cx: &mut Context<Root>,
) -> AnyElement {
    let selected = root.settings_section == section;
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
            OPEN_SELECT.store(0, Ordering::Relaxed);
            root.settings_section = section;
            if matches!(section, SettingsSection::Storage) {
                root.refresh_cache_stats(cx);
                root.refresh_audio_cache_index(cx);
            }
            cx.notify();
        }))
        .child(
            svg()
                .path(icons::path(icon))
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

fn shell(root: &Root, body: AnyElement, cx: &mut Context<Root>) -> AnyElement {
    let nav = div()
        .flex()
        .flex_col()
        .flex_none()
        .w(px(196.0))
        .gap(px(theme::space::XS))
        .p(px(theme::space::SM))
        .rounded(px(theme::radius::CARD))
        .bg(theme::surface())
        .border_1()
        .border_color(theme::border())
        .child(nav_item(
            root,
            SettingsSection::General,
            text(root, "外观与通知", "Appearance & notifications"),
            "palette",
            cx,
        ))
        .child(nav_item(
            root,
            SettingsSection::Playback,
            text(root, "播放与音频", "Playback & audio"),
            "music",
            cx,
        ))
        .child(nav_item(
            root,
            SettingsSection::Lyrics,
            text(root, "歌词显示", "Lyrics"),
            "captions",
            cx,
        ))
        .child(nav_item(
            root,
            SettingsSection::Downloads,
            text(root, "下载", "Downloads"),
            "download",
            cx,
        ))
        .child(nav_item(
            root,
            SettingsSection::Storage,
            text(root, "缓存与离线", "Cache & offline"),
            "hard-drive",
            cx,
        ))
        .child(nav_item(
            root,
            SettingsSection::Desktop,
            text(root, "桌面与系统", "Desktop & system"),
            "captions",
            cx,
        ))
        .child(nav_item(
            root,
            SettingsSection::Advanced,
            text(root, "性能", "Performance"),
            "settings",
            cx,
        ))
        .child(nav_item(
            root,
            SettingsSection::Account,
            text(root, "账户", "Account"),
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

fn page_header(title: String, _description: String) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .pb(px(theme::space::SM))
        .child(
            div()
                .text_size(theme::Text::Title.size())
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(theme::text())
                .child(title),
        )
        .into_any_element()
}

fn card(title: String, _description: String, children: Vec<AnyElement>) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(theme::space::SM))
        .p(px(theme::space::MD))
        .rounded(px(theme::radius::CARD))
        .bg(theme::surface())
        .border_1()
        .border_color(theme::border())
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .px(px(theme::space::XS))
                .child(
                    div()
                        .text_size(theme::Text::Large.size())
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme::text())
                        .child(title),
                ),
        )
        .children(children)
        .into_any_element()
}

fn toggle_row(
    id: &'static str,
    title: String,
    _description: String,
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
        .px(px(theme::space::SM))
        .py(px(theme::space::SM))
        .rounded(px(theme::radius::ROW))
        .cursor_pointer()
        .hover(|style| style.bg(theme::surface_hover()))
        .on_click(listener)
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w(px(0.0))
                .gap(px(2.0))
                .child(
                    div()
                        .text_size(theme::Text::Body.size())
                        .text_color(theme::text())
                        .child(title),
                ),
        )
        .child(
            div()
                .w(px(42.0))
                .h(px(24.0))
                .flex_none()
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
                .child(
                    div()
                        .size(px(18.0))
                        .rounded_full()
                        .bg(theme::accent_foreground()),
                ),
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
                .text_size(theme::Text::Small.size())
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

fn select_row(
    id: &'static str,
    menu_id: u8,
    title: String,
    _description: String,
    current: String,
    options: Vec<AnyElement>,
    cx: &mut Context<Root>,
) -> AnyElement {
    let expanded = OPEN_SELECT.load(Ordering::Relaxed) == menu_id;
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
                .gap(px(theme::space::LG))
                .px(px(theme::space::SM))
                .py(px(theme::space::SM))
                .rounded(px(theme::radius::ROW))
                .cursor_pointer()
                .hover(|style| style.bg(theme::surface_hover()))
                .on_click(cx.listener(move |_root, _event: &ClickEvent, _window, cx| {
                    let next = if OPEN_SELECT.load(Ordering::Relaxed) == menu_id {
                        0
                    } else {
                        menu_id
                    };
                    OPEN_SELECT.store(next, Ordering::Relaxed);
                    cx.notify();
                }))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w(px(0.0))
                        .gap(px(2.0))
                        .child(
                            div()
                                .text_size(theme::Text::Body.size())
                                .text_color(theme::text())
                                .child(title),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(theme::space::SM))
                        .px(px(theme::space::MD))
                        .h(px(34.0))
                        .max_w(px(360.0))
                        .rounded(px(theme::radius::ROW))
                        .bg(theme::surface_elevated())
                        .border_1()
                        .border_color(if expanded {
                            theme::accent()
                        } else {
                            theme::border()
                        })
                        .child(
                            div()
                                .truncate()
                                .text_size(theme::Text::Small.size())
                                .text_color(theme::text())
                                .child(current),
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
                ),
        )
        .when(expanded, |this| {
            this.child(
                div()
                    .ml(px(theme::space::SM))
                    .mr(px(theme::space::SM))
                    .p(px(theme::space::XS))
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .rounded(px(theme::radius::CARD))
                    .bg(theme::surface_elevated())
                    .border_1()
                    .border_color(theme::border())
                    .children(options),
            )
        })
        .into_any_element()
}

fn slider_value_at(
    bounds: &Arc<Mutex<Option<gpui::Bounds<gpui::Pixels>>>>,
    x: f32,
    min: f32,
    max: f32,
    step: f32,
) -> Option<f32> {
    let (left, width) = bounds
        .lock()
        .ok()
        .and_then(|slot| slot.map(|rect| (f32::from(rect.left()), f32::from(rect.size.width))))?;
    if width <= 1.0 {
        return None;
    }
    let ratio = ((x - left) / width).clamp(0.0, 1.0);
    let raw = min + ratio * (max - min);
    let step = step.max(f32::EPSILON);
    Some(((raw - min) / step).round() * step + min)
}

#[allow(clippy::too_many_arguments)]
fn slider_row(
    id: &'static str,
    title: String,
    _description: String,
    value: f32,
    min: f32,
    max: f32,
    step: f32,
    display: String,
    preview: SliderPreview,
    commit: SliderCommit,
    cx: &mut Context<Root>,
) -> AnyElement {
    let ratio = if max > min {
        ((value - min) / (max - min)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let bounds = Arc::new(Mutex::new(None));
    let bounds_for_canvas = bounds.clone();
    let bounds_for_down = bounds.clone();
    let bounds_for_move = bounds.clone();

    div()
        .flex()
        .flex_col()
        .gap(px(theme::space::SM))
        .px(px(theme::space::SM))
        .py(px(theme::space::SM))
        .rounded(px(theme::radius::ROW))
        .hover(|style| style.bg(theme::surface_hover()))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .gap(px(theme::space::LG))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w(px(0.0))
                        .gap(px(2.0))
                        .child(
                            div()
                                .text_size(theme::Text::Body.size())
                                .text_color(theme::text())
                                .child(title),
                        ),
                )
                .child(
                    div()
                        .px(px(theme::space::SM))
                        .py(px(3.0))
                        .rounded(px(theme::radius::PILL))
                        .bg(theme::surface_elevated())
                        .text_size(theme::Text::Tiny.size())
                        .text_color(theme::text())
                        .child(display),
                ),
        )
        .child(
            div()
                .id(id)
                .relative()
                .h(px(22.0))
                .w_full()
                .cursor_pointer()
                .child(
                    div()
                        .absolute()
                        .left(px(0.0))
                        .right(px(0.0))
                        .top(px(9.0))
                        .h(px(4.0))
                        .rounded(px(2.0))
                        .bg(theme::surface_hover()),
                )
                .child(
                    div()
                        .absolute()
                        .left(px(0.0))
                        .top(px(9.0))
                        .h(px(4.0))
                        .w(gpui::relative(ratio))
                        .rounded(px(2.0))
                        .bg(theme::accent()),
                )
                .child(
                    div()
                        .absolute()
                        .left(gpui::relative(ratio))
                        .top(px(4.0))
                        .size(px(14.0))
                        .rounded_full()
                        .bg(theme::text()),
                )
                .child(
                    canvas(
                        move |rect: gpui::Bounds<gpui::Pixels>, _window, _cx| {
                            if let Ok(mut slot) = bounds_for_canvas.lock() {
                                *slot = Some(rect);
                            }
                        },
                        |_bounds, _state, _window, _cx| {},
                    )
                    .absolute()
                    .size_full(),
                )
                .child(
                    div()
                        .absolute()
                        .size_full()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |root, event: &MouseDownEvent, _window, cx| {
                                if let Some(next) = slider_value_at(
                                    &bounds_for_down,
                                    f32::from(event.position.x),
                                    min,
                                    max,
                                    step,
                                ) {
                                    preview(root, next, cx);
                                }
                            }),
                        )
                        .on_mouse_move(cx.listener(
                            move |root, event: &MouseMoveEvent, _window, cx| {
                                if event.pressed_button != Some(MouseButton::Left) {
                                    return;
                                }
                                if let Some(next) = slider_value_at(
                                    &bounds_for_move,
                                    f32::from(event.position.x),
                                    min,
                                    max,
                                    step,
                                ) {
                                    preview(root, next, cx);
                                }
                            },
                        ))
                        .on_mouse_up(
                            MouseButton::Left,
                            cx.listener(move |root, _event, _window, cx| {
                                commit(root, cx);
                            }),
                        ),
                ),
        )
        .into_any_element()
}

fn action_button(
    id: &'static str,
    label: String,
    primary: bool,
    listener: impl Fn(&ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> AnyElement {
    div()
        .id(id)
        .h(px(34.0))
        .px(px(theme::space::MD))
        .flex()
        .items_center()
        .rounded(px(theme::radius::ROW))
        .bg(if primary {
            theme::accent()
        } else {
            theme::surface_elevated()
        })
        .border_1()
        .border_color(if primary {
            theme::accent()
        } else {
            theme::border()
        })
        .text_size(theme::Text::Small.size())
        .text_color(if primary {
            theme::accent_foreground()
        } else {
            theme::text()
        })
        .cursor_pointer()
        .hover(|style| style.opacity(0.86))
        .on_click(listener)
        .child(label)
        .into_any_element()
}

fn preview_ui_font(root: &mut Root, value: f32, cx: &mut Context<Root>) {
    root.settings.ui_font_size = value.round() as u32;
    theme::set_ui_font_size(root.settings.ui_font_size);
    cx.notify();
}

fn commit_ui_font(root: &mut Root, cx: &mut Context<Root>) {
    root.settings.ui_font_size = root.settings.ui_font_size.clamp(12, 20);
    theme::set_ui_font_size(root.settings.ui_font_size);
    let _ = root.settings.save();
    cx.notify();
}

fn preview_lyrics_font(root: &mut Root, value: f32, cx: &mut Context<Root>) {
    root.settings.lyrics_font_size = value.round() as u32;
    cx.notify();
}

fn commit_lyrics_font(root: &mut Root, cx: &mut Context<Root>) {
    let value = root.settings.lyrics_font_size;
    root.set_lyrics_font_size(value, cx);
}

fn preview_lyrics_line(root: &mut Root, value: f32, cx: &mut Context<Root>) {
    root.settings.lyrics_line_height = value.round() as u32;
    cx.notify();
}

fn commit_lyrics_line(root: &mut Root, cx: &mut Context<Root>) {
    let value = root.settings.lyrics_line_height;
    root.set_lyrics_line_height(value, cx);
}

fn preview_lyrics_offset(root: &mut Root, value: f32, cx: &mut Context<Root>) {
    root.settings.lyrics_offset_ms = value.round() as i64;
    cx.notify();
}

fn commit_lyrics_offset(root: &mut Root, cx: &mut Context<Root>) {
    let value = root.settings.lyrics_offset_ms;
    root.set_lyrics_offset_ms(value, cx);
}

fn preview_crossfade(root: &mut Root, value: f32, cx: &mut Context<Root>) {
    root.settings.crossfade_seconds = value.round() as u32;
    cx.notify();
}

fn commit_crossfade(root: &mut Root, cx: &mut Context<Root>) {
    let value = root.settings.crossfade_seconds;
    root.e3_set_crossfade(value, cx);
}

fn preview_prefetch_minutes(root: &mut Root, value: f32, cx: &mut Context<Root>) {
    root.settings.prefetch_minutes = value.round() as u32;
    cx.notify();
}

fn commit_prefetch_minutes(root: &mut Root, cx: &mut Context<Root>) {
    let value = root.settings.prefetch_minutes;
    root.set_prefetch_minutes(value, cx);
}

fn preview_prefetch_count(root: &mut Root, value: f32, cx: &mut Context<Root>) {
    root.settings.prefetch_count = value.round() as usize;
    cx.notify();
}

fn commit_prefetch_count(root: &mut Root, cx: &mut Context<Root>) {
    let value = root.settings.prefetch_count;
    root.set_prefetch_count(value, cx);
}

fn preview_cache_limit(root: &mut Root, value: f32, cx: &mut Context<Root>) {
    root.settings.cache_limit_gb = value.round() as u64;
    cx.notify();
}

fn commit_cache_limit(root: &mut Root, cx: &mut Context<Root>) {
    let value = root.settings.cache_limit_gb;
    root.set_cache_limit_gb(value, cx);
}

fn preview_desktop_opacity(root: &mut Root, value: f32, cx: &mut Context<Root>) {
    root.settings.desktop_lyrics_opacity = value.round() as u8;
    cx.notify();
}

fn commit_desktop_opacity(root: &mut Root, cx: &mut Context<Root>) {
    let value = root.settings.desktop_lyrics_opacity;
    root.e3_set_desktop_opacity(value, cx);
}

fn preview_desktop_font(root: &mut Root, value: f32, cx: &mut Context<Root>) {
    root.settings.desktop_lyrics_font_size = value.round() as u32;
    cx.notify();
}

fn commit_desktop_font(root: &mut Root, cx: &mut Context<Root>) {
    let value = root.settings.desktop_lyrics_font_size;
    root.e3_set_desktop_font_size(value, cx);
}

fn preview_compact_width(root: &mut Root, value: f32, cx: &mut Context<Root>) {
    root.settings.player_bar_compact_width = value.round() as u32;
    cx.notify();
}

fn commit_compact_width(root: &mut Root, cx: &mut Context<Root>) {
    let value = root.settings.player_bar_compact_width;
    root.e3_set_compact_width(value, cx);
}

fn general_view(root: &Root, cx: &mut Context<Root>) -> AnyElement {
    let theme_value = if root.theme_follows_system {
        text(root, "跟随系统", "Follow system")
    } else if root.theme == ThemeKind::Light {
        text(root, "浅色", "Light")
    } else {
        text(root, "深色", "Dark")
    };
    let theme_options = [
        (None, text(root, "跟随系统", "Follow system")),
        (Some(ThemeKind::Dark), text(root, "深色", "Dark")),
        (Some(ThemeKind::Light), text(root, "浅色", "Light")),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (value, label))| {
        let selected = match value {
            None => root.theme_follows_system,
            Some(value) => !root.theme_follows_system && root.theme == value,
        };
        option_row(
            format!("theme-option-{index}"),
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
                OPEN_SELECT.store(0, Ordering::Relaxed);
            }),
        )
    })
    .collect::<Vec<_>>();

    let language_value = if root.language_follows_system {
        text(root, "跟随系统", "Follow system")
    } else if root.language == Language::English {
        "English".to_string()
    } else {
        "中文".to_string()
    };
    let language_options = [
        (Language::System, text(root, "跟随系统", "Follow system")),
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
            format!("language-option-{index}"),
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
                OPEN_SELECT.store(0, Ordering::Relaxed);
            }),
        )
    })
    .collect::<Vec<_>>();

    let body = div()
        .flex()
        .flex_col()
        .gap(px(theme::space::LG))
        .child(page_header(
            text(root, "外观与通知", "Appearance & notifications"),
            text(
                root,
                "这里只放界面表现和应用级通知，不混入播放或缓存选项。",
                "Visual preferences and app-level notifications only.",
            ),
        ))
        .child(card(
            text(root, "界面", "Interface"),
            text(
                root,
                "主题和语言使用离散下拉选择。",
                "Theme and language use compact selectors.",
            ),
            vec![
                select_row(
                    "theme-select",
                    1,
                    text(root, "主题", "Theme"),
                    text(
                        root,
                        "可跟随系统，也可以固定深色或浅色。",
                        "Follow the system or pin a theme.",
                    ),
                    theme_value,
                    theme_options,
                    cx,
                ),
                select_row(
                    "language-select",
                    2,
                    text(root, "语言", "Language"),
                    text(
                        root,
                        "切换后立即刷新界面文本。",
                        "Updates the interface immediately.",
                    ),
                    language_value,
                    language_options,
                    cx,
                ),
                slider_row(
                    "ui-font-size",
                    text(root, "软件字体大小", "App font size"),
                    text(
                        root,
                        "统一调整侧栏、列表、设置等常规界面的基础字号。",
                        "Adjust the base font size used across the interface.",
                    ),
                    root.settings.ui_font_size as f32,
                    12.0,
                    20.0,
                    1.0,
                    format!("{} px", root.settings.ui_font_size),
                    preview_ui_font,
                    commit_ui_font,
                    cx,
                ),
            ],
        ))
        .child(card(
            text(root, "通知", "Notifications"),
            text(
                root,
                "系统级行为单独放在这里。",
                "System-level app behavior.",
            ),
            vec![toggle_row(
                "system-notifications",
                text(root, "切歌系统通知", "Track-change notifications"),
                text(
                    root,
                    "换歌时使用系统通知显示歌曲信息。",
                    "Show song information in system notifications.",
                ),
                root.settings.system_notifications,
                cx.listener(|root, _event: &ClickEvent, _window, cx| {
                    root.e3_set_notifications(!root.settings.system_notifications, cx);
                }),
            )],
        ))
        .into_any_element();
    shell(root, body, cx)
}

fn playback_view(root: &Root, cx: &mut Context<Root>) -> AnyElement {
    let quality = if root.settings.quality.trim().is_empty() {
        "auto"
    } else {
        root.settings.quality.as_str()
    };
    let quality_items = [
        ("auto", text(root, "自动", "Auto")),
        ("lossless", text(root, "无损", "Lossless")),
        ("highest", text(root, "极高", "Highest")),
        ("medium", text(root, "较高", "Medium")),
        ("low", text(root, "标准", "Standard")),
    ];
    let quality_value = quality_items
        .iter()
        .find(|(value, _)| *value == quality)
        .map(|(_, label)| label.clone())
        .unwrap_or_else(|| text(root, "自动", "Auto"));
    let quality_options = quality_items
        .into_iter()
        .enumerate()
        .map(|(index, (value, label))| {
            option_row(
                format!("quality-option-{index}"),
                label,
                quality == value,
                cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                    root.set_quality(value, cx);
                    OPEN_SELECT.store(0, Ordering::Relaxed);
                }),
            )
        })
        .collect::<Vec<_>>();

    let devices = crate::system_audio::list_output_devices();
    let current_device = devices
        .iter()
        .find(|device| {
            if root.settings.audio_output_device.trim().is_empty() {
                device.default
            } else {
                device.id == root.settings.audio_output_device
            }
        })
        .map(|device| device.name.clone())
        .unwrap_or_else(|| text(root, "系统默认", "System default"));
    let device_options = devices
        .into_iter()
        .enumerate()
        .map(|(index, device)| {
            let id = device.id.clone();
            let selected = if root.settings.audio_output_device.trim().is_empty() {
                device.default
            } else {
                root.settings.audio_output_device == device.id
            };
            option_row(
                format!("audio-device-{index}"),
                device.name,
                selected,
                cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                    root.e3_set_audio_device(id.clone(), cx);
                    OPEN_SELECT.store(0, Ordering::Relaxed);
                }),
            )
        })
        .collect::<Vec<_>>();

    let crossfade_display = if root.settings.crossfade_seconds == 0 {
        text(root, "关闭", "Off")
    } else {
        format!("{} s", root.settings.crossfade_seconds)
    };

    let body = div()
        .flex()
        .flex_col()
        .gap(px(theme::space::LG))
        .child(page_header(
            text(root, "播放与音频", "Playback & audio"),
            text(
                root,
                "与实际播放链路直接相关的选项集中在一起。",
                "Settings that directly affect the audio playback path.",
            ),
        ))
        .child(card(
            text(root, "音质与输出", "Quality & output"),
            text(
                root,
                "离散项目使用下拉框，避免大量平铺选项。",
                "Discrete values use selectors instead of long option lists.",
            ),
            vec![
                select_row(
                    "quality-select",
                    3,
                    text(root, "播放音质", "Playback quality"),
                    text(
                        root,
                        "自动模式会按账号权限和歌曲可用档位择优。",
                        "Auto chooses the best available tier for the account and track.",
                    ),
                    quality_value,
                    quality_options,
                    cx,
                ),
                select_row(
                    "audio-device-select",
                    4,
                    text(root, "音频输出设备", "Audio output device"),
                    text(
                        root,
                        "Ubuntu 下可切换扬声器、耳机、HDMI 或蓝牙输出。",
                        "Switch speakers, headphones, HDMI, or Bluetooth output.",
                    ),
                    current_device,
                    device_options,
                    cx,
                ),
            ],
        ))
        .child(card(
            text(root, "播放处理", "Playback processing"),
            text(
                root,
                "连续参数用滑动条，布尔能力使用开关。",
                "Continuous values use sliders; capabilities use switches.",
            ),
            vec![
                toggle_row(
                    "normalize-volume",
                    text(root, "响度标准化", "Loudness normalization"),
                    text(
                        root,
                        "减少不同歌曲之间的音量突变。",
                        "Reduce loudness jumps between tracks.",
                    ),
                    root.settings.normalize_volume,
                    cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.e3_set_normalize_volume(!root.settings.normalize_volume, cx);
                    }),
                ),
                toggle_row(
                    "gapless-playback",
                    text(root, "无缝衔接", "Gapless playback"),
                    text(
                        root,
                        "提前准备下一首，尽量缩短曲目边界空白。",
                        "Prepare the next track to reduce boundary gaps.",
                    ),
                    root.settings.gapless_playback,
                    cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.e3_set_gapless(!root.settings.gapless_playback, cx);
                    }),
                ),
                slider_row(
                    "crossfade-slider",
                    text(root, "交叉淡化", "Crossfade"),
                    text(
                        root,
                        "0 秒表示关闭；拖动后松手保存。",
                        "0 seconds disables crossfade; release to save.",
                    ),
                    root.settings.crossfade_seconds as f32,
                    0.0,
                    8.0,
                    1.0,
                    crossfade_display,
                    preview_crossfade,
                    commit_crossfade,
                    cx,
                ),
            ],
        ))
        .into_any_element();
    shell(root, body, cx)
}

fn lyrics_view(root: &Root, cx: &mut Context<Root>) -> AnyElement {
    let body = div()
        .flex()
        .flex_col()
        .gap(px(theme::space::LG))
        .child(page_header(
            text(root, "歌词显示", "Lyrics"),
            text(
                root,
                "主播放页歌词的排版与时间同步设置。",
                "Typography and timing for the main lyrics view.",
            ),
        ))
        .child(card(
            text(root, "排版", "Typography"),
            text(
                root,
                "字号和行距都支持连续拖动调节。",
                "Font size and line spacing are adjustable with sliders.",
            ),
            vec![
                slider_row(
                    "lyrics-font-slider",
                    text(root, "歌词字号", "Lyrics font size"),
                    text(
                        root,
                        "影响主播放页滚动歌词。",
                        "Affects scrolling lyrics in the playback view.",
                    ),
                    root.settings.lyrics_font_size as f32,
                    14.0,
                    30.0,
                    1.0,
                    format!("{} px", root.settings.lyrics_font_size),
                    preview_lyrics_font,
                    commit_lyrics_font,
                    cx,
                ),
                slider_row(
                    "lyrics-line-slider",
                    text(root, "歌词行距", "Lyrics line spacing"),
                    text(
                        root,
                        "增大后阅读更舒展，减小后显示更多行。",
                        "Increase for breathing room or reduce to fit more lines.",
                    ),
                    root.settings.lyrics_line_height as f32,
                    24.0,
                    52.0,
                    1.0,
                    format!("{} px", root.settings.lyrics_line_height),
                    preview_lyrics_line,
                    commit_lyrics_line,
                    cx,
                ),
            ],
        ))
        .child(card(
            text(root, "同步", "Timing"),
            text(
                root,
                "针对歌词整体提前或延后，不改变音频进度。",
                "Shift lyrics timing without changing audio position.",
            ),
            vec![slider_row(
                "lyrics-offset-slider",
                text(root, "歌词偏移", "Lyrics offset"),
                text(
                    root,
                    "负值提前，正值延后。",
                    "Negative values show earlier; positive values later.",
                ),
                root.settings.lyrics_offset_ms as f32,
                -3000.0,
                3000.0,
                50.0,
                format!("{} ms", root.settings.lyrics_offset_ms),
                preview_lyrics_offset,
                commit_lyrics_offset,
                cx,
            )],
        ))
        .into_any_element();
    shell(root, body, cx)
}

fn downloads_view(root: &Root, cx: &mut Context<Root>) -> AnyElement {
    let quality_items = [
        (
            "follow",
            text(root, "跟随播放音质", "Follow playback quality"),
        ),
        ("lossless", text(root, "无损", "Lossless")),
        ("highest", text(root, "极高", "Highest")),
        ("medium", text(root, "较高", "Medium")),
        ("low", text(root, "标准", "Standard")),
    ];
    let quality_value = quality_items
        .iter()
        .find(|(value, _)| *value == root.settings.download_quality)
        .map(|(_, label)| label.clone())
        .unwrap_or_else(|| text(root, "跟随播放音质", "Follow playback quality"));
    let quality_options = quality_items
        .into_iter()
        .enumerate()
        .map(|(index, (value, label))| {
            option_row(
                format!("download-quality-{index}"),
                label,
                root.settings.download_quality == value,
                cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                    root.set_download_quality(value, cx);
                    OPEN_SELECT.store(0, Ordering::Relaxed);
                }),
            )
        })
        .collect::<Vec<_>>();

    let format_items = [
        ("source", text(root, "源格式", "Source format")),
        ("mp3", "MP3".to_string()),
        ("flac", "FLAC".to_string()),
    ];
    let format_value = format_items
        .iter()
        .find(|(value, _)| *value == root.settings.download_format)
        .map(|(_, label)| label.clone())
        .unwrap_or_else(|| text(root, "源格式", "Source format"));
    let format_options = format_items
        .into_iter()
        .enumerate()
        .map(|(index, (value, label))| {
            option_row(
                format!("download-format-{index}"),
                label,
                root.settings.download_format == value,
                cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                    root.set_download_format(value, cx);
                    OPEN_SELECT.store(0, Ordering::Relaxed);
                }),
            )
        })
        .collect::<Vec<_>>();
    let download_path = sodam_core::downloads::download_dir().display().to_string();

    let body = div()
        .flex()
        .flex_col()
        .gap(px(theme::space::LG))
        .child(page_header(
            text(root, "下载", "Downloads"),
            text(
                root,
                "只包含导出文件相关设置，缓存策略移动到了“缓存与离线”。",
                "Only exported-file settings live here; cache policy is under Cache & offline.",
            ),
        ))
        .child(card(
            text(root, "文件质量", "File quality"),
            text(
                root,
                "音质和格式都是离散选项，统一使用下拉框。",
                "Quality and format are discrete choices and use selectors.",
            ),
            vec![
                select_row(
                    "download-quality-select",
                    5,
                    text(root, "下载音质", "Download quality"),
                    text(
                        root,
                        "可以独立于播放音质选择。",
                        "Can be chosen independently from playback quality.",
                    ),
                    quality_value,
                    quality_options,
                    cx,
                ),
                select_row(
                    "download-format-select",
                    6,
                    text(root, "下载格式", "Download format"),
                    text(
                        root,
                        "源格式保留服务端提供的原始容器。",
                        "Source keeps the original container when possible.",
                    ),
                    format_value,
                    format_options,
                    cx,
                ),
            ],
        ))
        .child(card(
            text(root, "下载目录", "Download folder"),
            download_path,
            vec![div()
                .flex()
                .flex_row()
                .flex_wrap()
                .gap(px(theme::space::SM))
                .px(px(theme::space::SM))
                .child(action_button(
                    "choose-download-dir",
                    text(root, "更改目录", "Change folder"),
                    true,
                    cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.choose_download_directory(cx);
                    }),
                ))
                .child(action_button(
                    "open-download-dir",
                    text(root, "打开目录", "Open folder"),
                    false,
                    cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.open_download_folder(cx);
                    }),
                ))
                .child(action_button(
                    "reset-download-dir",
                    text(root, "恢复默认", "Reset default"),
                    false,
                    cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.reset_download_directory(cx);
                    }),
                ))
                .into_any_element()],
        ))
        .into_any_element();
    shell(root, body, cx)
}

fn storage_view(root: &Root, cx: &mut Context<Root>) -> AnyElement {
    let (audio_bytes, audio_files, cover_bytes, cover_files) = root.cache_summary;
    let total = audio_bytes + cover_bytes;
    let summary = if root.language.resolved().is_zh() {
        format!(
            "共 {} · 歌曲 {} 首 / {} · 封面 {} 张 / {}",
            human_bytes(total),
            audio_files,
            human_bytes(audio_bytes),
            cover_files,
            human_bytes(cover_bytes)
        )
    } else {
        format!(
            "{} total · {} tracks / {} · {} covers / {}",
            human_bytes(total),
            audio_files,
            human_bytes(audio_bytes),
            cover_files,
            human_bytes(cover_bytes)
        )
    };
    let cache_limit = if root.settings.cache_limit_gb == 0 {
        text(root, "不限制", "Unlimited")
    } else {
        format!("{} GB", root.settings.cache_limit_gb)
    };

    let mut prefetch_controls = vec![toggle_row(
        "adaptive-prefetch",
        text(root, "自适应预加载", "Adaptive prefetch"),
        text(
            root,
            "根据未来播放时长动态决定缓存深度。",
            "Choose prefetch depth based on upcoming playback duration.",
        ),
        root.settings.prefetch_adaptive,
        cx.listener(|root, _event: &ClickEvent, _window, cx| {
            root.set_prefetch_adaptive(!root.settings.prefetch_adaptive, cx);
        }),
    )];
    if root.settings.prefetch_adaptive {
        prefetch_controls.push(slider_row(
            "prefetch-minutes-slider",
            text(root, "预加载目标时长", "Prefetch target duration"),
            text(
                root,
                "希望前方至少准备多少分钟的歌曲。",
                "Minimum duration to keep prepared ahead.",
            ),
            root.settings.prefetch_minutes as f32,
            5.0,
            30.0,
            1.0,
            format!("{} min", root.settings.prefetch_minutes),
            preview_prefetch_minutes,
            commit_prefetch_minutes,
            cx,
        ));
    } else {
        prefetch_controls.push(slider_row(
            "prefetch-count-slider",
            text(root, "固定预加载首数", "Fixed prefetch count"),
            text(root, "0 表示关闭预加载。", "0 disables prefetching."),
            root.settings.prefetch_count as f32,
            0.0,
            8.0,
            1.0,
            root.settings.prefetch_count.to_string(),
            preview_prefetch_count,
            commit_prefetch_count,
            cx,
        ));
    }

    let body = div()
        .flex()
        .flex_col()
        .gap(px(theme::space::LG))
        .child(page_header(
            text(root, "缓存与离线", "Cache & offline"),
            text(
                root,
                "预加载、离线播放和缓存上限统一放在同一个数据策略板块。",
                "Prefetching, offline playback, and cache limits are grouped as one data policy.",
            ),
        ))
        .child(card(
            text(root, "离线播放", "Offline playback"),
            text(
                root,
                "离线模式只使用已有本地音频缓存。",
                "Offline mode only uses audio already cached locally.",
            ),
            vec![toggle_row(
                "offline-mode",
                text(root, "离线模式", "Offline mode"),
                text(
                    root,
                    "开启后禁止新的音频网络请求。",
                    "Block new audio network requests while enabled.",
                ),
                root.settings.offline_mode,
                cx.listener(|root, _event: &ClickEvent, _window, cx| {
                    root.set_offline_mode(!root.settings.offline_mode, cx);
                }),
            )],
        ))
        .child(card(
            text(root, "智能预加载", "Smart prefetch"),
            text(
                root,
                "这是缓存策略，不再放在播放设置里。",
                "This is a cache policy, so it no longer lives under Playback.",
            ),
            prefetch_controls,
        ))
        .child(card(
            text(root, "缓存空间", "Cache storage"),
            summary,
            vec![
                slider_row(
                    "cache-limit-slider",
                    text(root, "缓存上限", "Cache limit"),
                    text(
                        root,
                        "0 GB 表示不限制；达到上限后按最近使用自动清理。",
                        "0 GB means unlimited; old cache is trimmed automatically after the limit.",
                    ),
                    root.settings.cache_limit_gb as f32,
                    0.0,
                    20.0,
                    1.0,
                    cache_limit,
                    preview_cache_limit,
                    commit_cache_limit,
                    cx,
                ),
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .gap(px(theme::space::SM))
                    .px(px(theme::space::SM))
                    .child(action_button(
                        "refresh-cache-stats",
                        text(root, "刷新统计", "Refresh stats"),
                        false,
                        cx.listener(|root, _event: &ClickEvent, _window, cx| {
                            root.refresh_cache_stats(cx);
                            root.refresh_audio_cache_index(cx);
                        }),
                    ))
                    .child(action_button(
                        "clear-audio-cache",
                        text(root, "清除歌曲缓存", "Clear audio cache"),
                        false,
                        cx.listener(|root, _event: &ClickEvent, _window, cx| {
                            let removed = sodam_core::audio::clear_cache();
                            let message =
                                root.localized("已清理缓存：{} 个文件", &[removed.to_string()]);
                            root.refresh_cache_stats(cx);
                            root.refresh_audio_cache_index(cx);
                            root.toast(message, cx);
                        }),
                    ))
                    .into_any_element(),
            ],
        ))
        .into_any_element();
    shell(root, body, cx)
}

fn desktop_view(root: &Root, cx: &mut Context<Root>) -> AnyElement {
    let color_value = if root.settings.desktop_lyrics_color == "text" {
        text(root, "主题文字色", "Theme text color")
    } else {
        text(root, "强调色", "Accent color")
    };
    let color_options = [
        ("accent", text(root, "强调色", "Accent color")),
        ("text", text(root, "主题文字色", "Theme text color")),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (value, label))| {
        option_row(
            format!("desktop-color-{index}"),
            label,
            root.settings.desktop_lyrics_color == value,
            cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                root.e3_set_desktop_color(value, cx);
                OPEN_SELECT.store(0, Ordering::Relaxed);
            }),
        )
    })
    .collect::<Vec<_>>();

    let align_value = match root.settings.desktop_lyrics_align.as_str() {
        "left" => text(root, "左对齐", "Left"),
        "right" => text(root, "右对齐", "Right"),
        _ => text(root, "居中", "Center"),
    };
    let align_options = [
        ("left", text(root, "左对齐", "Left")),
        ("center", text(root, "居中", "Center")),
        ("right", text(root, "右对齐", "Right")),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (value, label))| {
        option_row(
            format!("desktop-align-{index}"),
            label,
            root.settings.desktop_lyrics_align == value,
            cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                root.e3_set_desktop_align(value, cx);
                OPEN_SELECT.store(0, Ordering::Relaxed);
            }),
        )
    })
    .collect::<Vec<_>>();

    let panel_value = match root.settings.panel_lyrics_position.as_str() {
        "left" => text(root, "左侧", "Left"),
        "center-right" => text(root, "日期右侧", "Right of clock"),
        "right" => text(root, "右侧状态区", "Right status area"),
        _ => text(root, "日期左侧", "Left of clock"),
    };
    let panel_options = [
        ("left", text(root, "左侧", "Left")),
        ("center-left", text(root, "日期左侧", "Left of clock")),
        ("center-right", text(root, "日期右侧", "Right of clock")),
        ("right", text(root, "右侧状态区", "Right status area")),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (value, label))| {
        option_row(
            format!("panel-position-{index}"),
            label,
            root.settings.panel_lyrics_position == value,
            cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                root.settings.panel_lyrics_position = value.to_string();
                let _ = root.settings.save();
                OPEN_SELECT.store(0, Ordering::Relaxed);
                cx.notify();
            }),
        )
    })
    .collect::<Vec<_>>();

    let body = div()
        .flex()
        .flex_col()
        .gap(px(theme::space::LG))
        .child(page_header(
            text(root, "桌面与系统", "Desktop & system"),
            text(
                root,
                "桌面歌词窗口和 Ubuntu/GNOME 顶栏歌词统一管理。",
                "Manage the floating lyrics window and Ubuntu/GNOME panel lyrics together.",
            ),
        ))
        .child(card(
            text(root, "桌面歌词窗口", "Floating desktop lyrics"),
            text(
                root,
                "窗口行为使用开关，视觉参数使用滑动条或下拉框。",
                "Window behavior uses switches; visual parameters use sliders or selectors.",
            ),
            vec![
                toggle_row(
                    "desktop-single-line",
                    text(root, "单行歌词", "Single-line lyrics"),
                    text(
                        root,
                        "关闭时显示当前行和下一行。",
                        "Show current and next line when disabled.",
                    ),
                    root.settings.desktop_lyrics_single_line,
                    cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.e3_set_desktop_single_line(
                            !root.settings.desktop_lyrics_single_line,
                            cx,
                        );
                    }),
                ),
                toggle_row(
                    "desktop-always-top",
                    text(root, "始终置顶", "Always on top"),
                    text(
                        root,
                        "让歌词窗口保持在普通窗口上方。",
                        "Keep the lyrics window above normal windows.",
                    ),
                    root.settings.desktop_lyrics_always_on_top,
                    cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.e3_set_desktop_always_on_top(
                            !root.settings.desktop_lyrics_always_on_top,
                            cx,
                        );
                    }),
                ),
                toggle_row(
                    "desktop-lock",
                    text(root, "锁定位置", "Lock position"),
                    text(
                        root,
                        "锁定后禁止拖动和调整窗口大小。",
                        "Disable moving and resizing while locked.",
                    ),
                    root.settings.desktop_lyrics_locked,
                    cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.e3_set_desktop_locked(!root.settings.desktop_lyrics_locked, cx);
                    }),
                ),
                toggle_row(
                    "desktop-click-through",
                    text(root, "鼠标穿透", "Click-through"),
                    text(
                        root,
                        "开启后鼠标事件穿过桌面歌词窗口。",
                        "Pass pointer events through the floating lyrics window.",
                    ),
                    root.settings.desktop_lyrics_click_through,
                    cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.e3_set_desktop_click_through(
                            !root.settings.desktop_lyrics_click_through,
                            cx,
                        );
                    }),
                ),
                slider_row(
                    "desktop-opacity-slider",
                    text(root, "背景透明度", "Background opacity"),
                    text(
                        root,
                        "支持 30%–100% 无级拖动。",
                        "Continuously adjustable from 30% to 100%.",
                    ),
                    root.settings.desktop_lyrics_opacity as f32,
                    30.0,
                    100.0,
                    1.0,
                    format!("{}%", root.settings.desktop_lyrics_opacity),
                    preview_desktop_opacity,
                    commit_desktop_opacity,
                    cx,
                ),
                slider_row(
                    "desktop-font-slider",
                    text(root, "桌面歌词字号", "Desktop lyrics font size"),
                    text(
                        root,
                        "独立于主播放页歌词字号。",
                        "Independent from the main lyrics font size.",
                    ),
                    root.settings.desktop_lyrics_font_size as f32,
                    20.0,
                    44.0,
                    1.0,
                    format!("{} px", root.settings.desktop_lyrics_font_size),
                    preview_desktop_font,
                    commit_desktop_font,
                    cx,
                ),
                select_row(
                    "desktop-color-select",
                    7,
                    text(root, "歌词颜色", "Lyrics color"),
                    text(
                        root,
                        "这是离散模式，使用下拉选择。",
                        "A discrete display mode, so it uses a selector.",
                    ),
                    color_value,
                    color_options,
                    cx,
                ),
                select_row(
                    "desktop-align-select",
                    8,
                    text(root, "歌词对齐", "Lyrics alignment"),
                    text(
                        root,
                        "选择左对齐、居中或右对齐。",
                        "Choose left, center, or right alignment.",
                    ),
                    align_value,
                    align_options,
                    cx,
                ),
            ],
        ))
        .child(card(
            text(
                root,
                "Ubuntu / GNOME 顶栏歌词",
                "Ubuntu / GNOME panel lyrics",
            ),
            text(
                root,
                "顶栏歌词跟随当前播放进度，位置使用离散下拉选择。",
                "Panel lyrics follow playback and use a discrete position selector.",
            ),
            vec![
                toggle_row(
                    "panel-lyrics-enabled",
                    text(root, "显示顶栏歌词", "Show panel lyrics"),
                    text(
                        root,
                        "关闭后 GNOME 扩展会隐藏歌词。",
                        "Hide the GNOME extension lyric label when disabled.",
                    ),
                    root.settings.panel_lyrics_enabled,
                    cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.settings.panel_lyrics_enabled = !root.settings.panel_lyrics_enabled;
                        let _ = root.settings.save();
                        cx.notify();
                    }),
                ),
                select_row(
                    "panel-position-select",
                    9,
                    text(root, "顶栏位置", "Panel position"),
                    text(
                        root,
                        "默认放在日期时间左侧。",
                        "Defaults to the left of the clock.",
                    ),
                    panel_value,
                    panel_options,
                    cx,
                ),
            ],
        ))
        .into_any_element();
    shell(root, body, cx)
}

fn advanced_view(root: &Root, cx: &mut Context<Root>) -> AnyElement {
    let body = div()
        .flex()
        .flex_col()
        .gap(px(theme::space::LG))
        .child(page_header(
            text(root, "性能", "Performance"),
            text(
                root,
                "这里只保留界面性能与启动策略，不再混放音频和缓存选项。",
                "Only startup and interface-performance tuning live here.",
            ),
        ))
        .child(card(
            text(root, "启动", "Startup"),
            text(
                root,
                "减少首次打开时不必要的后台工作。",
                "Reduce unnecessary background work on first launch.",
            ),
            vec![toggle_row(
                "lazy-startup",
                text(root, "延迟加载非首屏内容", "Lazy-load non-primary content"),
                text(
                    root,
                    "优先让主界面可用，再逐步加载次要内容。",
                    "Make the main UI usable first, then load secondary content.",
                ),
                root.settings.lazy_startup,
                cx.listener(|root, _event: &ClickEvent, _window, cx| {
                    root.e3_set_lazy_startup(!root.settings.lazy_startup, cx);
                }),
            )],
        ))
        .child(card(
            text(root, "响应式布局", "Responsive layout"),
            text(
                root,
                "控制底部播放栏何时进入紧凑模式。",
                "Controls when the bottom player switches to compact mode.",
            ),
            vec![slider_row(
                "compact-width-slider",
                text(root, "播放栏折叠阈值", "Player compact threshold"),
                text(
                    root,
                    "窗口低于该宽度时隐藏次要按钮。",
                    "Hide secondary controls below this width.",
                ),
                root.settings.player_bar_compact_width as f32,
                720.0,
                1200.0,
                10.0,
                format!("{} px", root.settings.player_bar_compact_width),
                preview_compact_width,
                commit_compact_width,
                cx,
            )],
        ))
        .into_any_element();
    shell(root, body, cx)
}

fn account_view(root: &Root, cx: &mut Context<Root>) -> AnyElement {
    let logged_in = !root.settings.cookie.trim().is_empty();
    let account_title = if logged_in {
        root.account
            .as_ref()
            .filter(|info| !info.nickname.trim().is_empty())
            .map(|info| info.nickname.clone())
            .unwrap_or_else(|| text(root, "已登录", "Signed in"))
    } else {
        text(root, "未登录", "Not signed in")
    };
    let account_detail = if let Some(info) = root.account.as_ref().filter(|_| logged_in) {
        format!(
            "{} · {}",
            info.user_id,
            if info.vip {
                text(root, "VIP", "VIP")
            } else {
                text(root, "非 VIP", "Non-VIP")
            }
        )
    } else if logged_in {
        text(root, "正在读取账号信息…", "Loading account information…")
    } else {
        text(
            root,
            "登录后才能使用在线搜索、歌单与完整播放能力。",
            "Sign in to use online search, playlists, and full playback features.",
        )
    };
    let signer = if root.settings.signer_url.trim().is_empty() {
        text(root, "未配置", "Not configured")
    } else {
        root.settings.signer_url.trim().to_string()
    };

    let account_action = if logged_in {
        action_button(
            "account-logout",
            text(root, "退出登录", "Sign out"),
            false,
            cx.listener(|root, _event: &ClickEvent, _window, cx| {
                root.logout(cx);
            }),
        )
    } else {
        action_button(
            "account-login",
            text(root, "登录", "Sign in"),
            true,
            cx.listener(|root, _event: &ClickEvent, _window, cx| {
                root.login_modal_open = true;
                root.start_login(cx);
            }),
        )
    };

    let body = div()
        .flex()
        .flex_col()
        .gap(px(theme::space::LG))
        .child(page_header(
            text(root, "账户", "Account"),
            text(
                root,
                "登录状态与服务诊断信息。",
                "Sign-in state and service diagnostics.",
            ),
        ))
        .child(card(
            account_title,
            account_detail,
            vec![div()
                .flex()
                .px(px(theme::space::SM))
                .child(account_action)
                .into_any_element()],
        ))
        .child(card(
            text(root, "服务信息", "Service information"),
            text(
                root,
                "用于排查 VIP 整曲和签名服务问题。",
                "Useful for diagnosing full-track and signing-service issues.",
            ),
            vec![div()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .px(px(theme::space::SM))
                .py(px(theme::space::SM))
                .rounded(px(theme::radius::ROW))
                .bg(theme::surface_elevated())
                .child(
                    div()
                        .text_size(theme::Text::Tiny.size())
                        .text_color(theme::text_muted())
                        .child(text(root, "签名服务", "Signer service")),
                )
                .child(
                    div()
                        .text_size(theme::Text::Small.size())
                        .text_color(theme::text())
                        .child(signer),
                )
                .into_any_element()],
        ))
        .into_any_element();
    shell(root, body, cx)
}

pub(crate) fn settings_view(root: &Root, cx: &mut Context<Root>) -> AnyElement {
    match root.settings_section {
        SettingsSection::General => general_view(root, cx),
        SettingsSection::Playback => playback_view(root, cx),
        SettingsSection::Lyrics => lyrics_view(root, cx),
        SettingsSection::Downloads => downloads_view(root, cx),
        SettingsSection::Storage => storage_view(root, cx),
        SettingsSection::Desktop => desktop_view(root, cx),
        SettingsSection::Advanced => advanced_view(root, cx),
        SettingsSection::Account => account_view(root, cx),
    }
}
