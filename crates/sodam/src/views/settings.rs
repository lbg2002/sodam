//! 设置中心：商业级三组六栏架构（账户与个性化 / 听歌与视效 / 存储与系统），支持响应式导航与即时预览。

use super::*;
use crate::app::{SettingsSection, UpdateState};
use crate::ui::i18n::Language;
use gpui::{img, relative, Context, FontWeight, MouseMoveEvent};
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

fn switch_section(root: &mut Root, section: SettingsSection, cx: &mut Context<Root>) {
    OPEN_SELECT.store(0, Ordering::Relaxed);
    root.settings_section = section;
    if matches!(section, SettingsSection::Storage) {
        root.trim_cache_if_needed(cx);
        root.refresh_cache_stats(cx);
        root.refresh_audio_cache_index(cx);
    }
    cx.notify();
}

fn nav_group_title(label: String) -> AnyElement {
    div()
        .px(px(theme::space::MD))
        .pt(px(theme::space::MD))
        .pb(px(theme::space::XS))
        .text_size(theme::Text::Tiny.size())
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(theme::text_faint())
        .child(label)
        .into_any_element()
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
        .when(selected, |this| this.font_weight(FontWeight::SEMIBOLD))
        .when(!selected, |this| {
            this.hover(|style| style.bg(theme::surface_hover()))
        })
        .on_click(cx.listener(move |root, _event: &ClickEvent, _window, cx| {
            switch_section(root, section, cx);
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
                .truncate()
                .text_size(theme::Text::Small.size())
                .when(selected, |this| this.font_weight(FontWeight::SEMIBOLD))
                .text_color(if selected {
                    theme::accent()
                } else {
                    theme::text_muted()
                })
                .child(label),
        )
        .into_any_element()
}

fn top_nav_pill(
    root: &Root,
    section: SettingsSection,
    label: String,
    icon: &'static str,
    cx: &mut Context<Root>,
) -> AnyElement {
    let selected = root.settings_section == section;
    div()
        .id(gpui::ElementId::Name(
            format!("settings-pill-{section:?}").into(),
        ))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(theme::space::XS))
        .h(px(theme::size::CONTROL_SM))
        .px(px(theme::space::MD))
        .rounded(px(theme::radius::PILL))
        .cursor_pointer()
        .bg(if selected {
            theme::accent()
        } else {
            theme::surface_elevated()
        })
        .border_1()
        .border_color(if selected {
            theme::accent()
        } else {
            theme::border()
        })
        .when(!selected, |this| {
            this.hover(|style| style.bg(theme::surface_hover()))
        })
        .on_click(cx.listener(move |root, _event: &ClickEvent, _window, cx| {
            switch_section(root, section, cx);
        }))
        .child(
            svg()
                .path(icons::path(icon))
                .size(px(theme::ICON_SM))
                .text_color(if selected {
                    theme::accent_foreground()
                } else {
                    theme::text_muted()
                }),
        )
        .child(
            div()
                .text_size(theme::Text::Small.size())
                .when(selected, |this| this.font_weight(FontWeight::SEMIBOLD))
                .text_color(if selected {
                    theme::accent_foreground()
                } else {
                    theme::text()
                })
                .child(label),
        )
        .into_any_element()
}

fn section_entries(root: &Root) -> [(SettingsSection, String, &'static str); 6] {
    [
        (
            SettingsSection::Account,
            text(root, "账号与会员", "Account & VIP"),
            "user-round",
        ),
        (
            SettingsSection::General,
            text(root, "外观与主题", "Themes & Layout"),
            "palette",
        ),
        (
            SettingsSection::Playback,
            text(root, "播放与音质", "Playback & Audio"),
            "music",
        ),
        (
            SettingsSection::Lyrics,
            text(root, "歌词体验", "Lyrics Experience"),
            "captions",
        ),
        (
            SettingsSection::Storage,
            text(root, "下载与存储", "Downloads & Storage"),
            "hard-drive",
        ),
        (
            SettingsSection::About,
            text(root, "系统与关于", "System & About"),
            "info",
        ),
    ]
}

fn shell(root: &Root, viewport_w: f32, body: AnyElement, cx: &mut Context<Root>) -> AnyElement {
    let entries = section_entries(root);
    let compact = viewport_w < 820.0;

    if compact {
        let pills = div()
            .flex()
            .flex_row()
            .flex_wrap()
            .gap(px(theme::space::XS))
            .p(px(theme::space::SM))
            .rounded(px(theme::radius::CARD))
            .children(
                entries
                    .into_iter()
                    .map(|(section, label, icon)| top_nav_pill(root, section, label, icon, cx)),
            );

        return div()
            .id("settings-layout-compact")
            .flex()
            .flex_col()
            .flex_1()
            .min_h(px(0.0))
            .gap(px(theme::space::MD))
            .child(pills)
            .child(
                div()
                    .id("settings-section-scroll-compact")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w(px(0.0))
                    .min_h(px(0.0))
                    .overflow_y_scroll()
                    .pr(px(theme::space::XS))
                    .child(body),
            )
            .into_any_element();
    }

    let [account, general, playback, lyrics, storage, about] = entries;
    let nav = div()
        .id("settings-nav-scroll")
        .h_full()
        .min_h(px(0.0))
        .flex()
        .flex_col()
        .flex_none()
        .w(px(220.0))
        .gap(px(2.0))
        .p(px(theme::space::SM))
        .overflow_y_scroll()
        .child(
            div()
                .px(px(theme::space::MD))
                .py(px(theme::space::SM))
                .text_size(theme::Text::Large.size())
                .font_weight(FontWeight::BOLD)
                .text_color(theme::text())
                .child(text(root, "设置中心", "Settings")),
        )
        .child(nav_group_title(text(
            root,
            "账户与个性化",
            "Account & Personalization",
        )))
        .child(nav_item(root, account.0, account.1, account.2, cx))
        .child(nav_item(root, general.0, general.1, general.2, cx))
        .child(nav_group_title(text(root, "听歌与视效", "Audio & Lyrics")))
        .child(nav_item(root, playback.0, playback.1, playback.2, cx))
        .child(nav_item(root, lyrics.0, lyrics.1, lyrics.2, cx))
        .child(nav_group_title(text(
            root,
            "存储与系统",
            "Storage & System",
        )))
        .child(nav_item(root, storage.0, storage.1, storage.2, cx))
        .child(nav_item(root, about.0, about.1, about.2, cx));

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
                .child(
                    div()
                        .w_full()
                        .max_w(px(880.0))
                        .mx_auto()
                        .pb(px(theme::space::XL))
                        .child(body),
                ),
        )
        .into_any_element()
}

fn page_header(title: String) -> AnyElement {
    div()
        .pb(px(theme::space::XS))
        .text_size(theme::Text::Title.size())
        .font_weight(FontWeight::BOLD)
        .text_color(theme::text())
        .child(title)
        .into_any_element()
}

fn section_group(title: String, rows: Vec<AnyElement>) -> AnyElement {
    let total = rows.len();
    let mut inner = div()
        .flex()
        .flex_col()
        .p(px(theme::space::XS))
        .rounded(px(14.0))
        .bg(theme::rgba_alpha(theme::surface_elevated(), 0.56))
        .border_1()
        .border_color(theme::rgba_alpha(theme::border(), 0.30));

    for (idx, row) in rows.into_iter().enumerate() {
        inner = inner.child(row);
        if idx + 1 < total {
            inner = inner.child(
                div()
                    .h(px(1.0))
                    .mx(px(theme::space::MD))
                    .bg(theme::rgba_alpha(theme::border(), 0.22)),
            );
        }
    }

    let mut wrap = div().flex().flex_col().gap(px(7.0));
    if !title.trim().is_empty() {
        wrap = wrap.child(
            div()
                .px(px(theme::space::MD))
                .text_size(theme::Text::Tiny.size())
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme::text_muted())
                .child(title),
        );
    }
    wrap.child(inner).into_any_element()
}

type SegmentClick = Box<dyn Fn(&ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static>;

fn segmented_row(
    title: String,
    description: &str,
    segments: Vec<(&'static str, String, bool, SegmentClick)>,
) -> AnyElement {
    let track = div()
        .flex()
        .flex_row()
        .flex_none()
        .items_center()
        .gap(px(2.0))
        .p(px(3.0))
        .rounded(px(10.0))
        .bg(theme::rgba_alpha(theme::surface(), 0.72))
        .border_1()
        .border_color(theme::rgba_alpha(theme::border(), 0.28))
        .children(
            segments
                .into_iter()
                .map(|(seg_id, label, active, listener)| {
                    div()
                        .id(seg_id)
                        .h(px(26.0))
                        .px(px(11.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(7.0))
                        .cursor_pointer()
                        .when(active, |this| {
                            this.bg(theme::accent())
                                .text_color(theme::accent_foreground())
                                .font_weight(FontWeight::SEMIBOLD)
                        })
                        .when(!active, |this| {
                            this.text_color(theme::text_muted()).hover(|style| {
                                style
                                    .bg(theme::rgba_alpha(theme::surface_hover(), 0.85))
                                    .text_color(theme::text())
                            })
                        })
                        .text_size(theme::Text::Tiny.size())
                        .whitespace_nowrap()
                        .on_click(listener)
                        .child(label)
                }),
        );

    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .justify_between()
        .gap(px(theme::space::MD))
        .min_h(px(44.0))
        .px(px(theme::space::MD))
        .py(px(theme::space::SM))
        .rounded(px(10.0))
        .child(row_label(title, description.to_string()))
        .child(track)
        .into_any_element()
}

fn row_label(title: String, description: String) -> AnyElement {
    let has_desc = !description.trim().is_empty();
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
        )
        .when(has_desc, |this| {
            this.child(
                div()
                    .text_size(theme::Text::Tiny.size())
                    .text_color(theme::text_muted())
                    .child(description),
            )
        })
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
        .min_h(px(44.0))
        .px(px(theme::space::MD))
        .py(px(theme::space::SM))
        .rounded(px(10.0))
        .cursor_pointer()
        .hover(|style| style.bg(theme::rgba_alpha(theme::surface_hover(), 0.72)))
        .on_click(listener)
        .child(row_label(title, description))
        .child(
            div()
                .w(px(40.0))
                .h(px(22.0))
                .flex_none()
                .rounded(px(11.0))
                .bg(if enabled {
                    theme::accent()
                } else {
                    theme::rgba_alpha(theme::surface(), 0.85)
                })
                .border_1()
                .border_color(if enabled {
                    theme::accent()
                } else {
                    theme::rgba_alpha(theme::border(), 0.45)
                })
                .flex()
                .items_center()
                .px(px(2.0))
                .justify_end()
                .when(!enabled, |this| this.justify_start())
                .child(div().size(px(16.0)).rounded_full().bg(if enabled {
                    theme::accent_foreground()
                } else {
                    theme::text_muted()
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
                .text_size(theme::Text::Small.size())
                .when(selected, |this| this.font_weight(FontWeight::SEMIBOLD))
                .text_color(if selected {
                    theme::accent()
                } else {
                    theme::text()
                })
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
    description: String,
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
                .min_h(px(44.0))
                .px(px(theme::space::MD))
                .py(px(theme::space::SM))
                .rounded(px(10.0))
                .cursor_pointer()
                .hover(|style| style.bg(theme::rgba_alpha(theme::surface_hover(), 0.72)))
                .on_click(cx.listener(move |_root, _event: &ClickEvent, _window, cx| {
                    let next = if OPEN_SELECT.load(Ordering::Relaxed) == menu_id {
                        0
                    } else {
                        menu_id
                    };
                    OPEN_SELECT.store(next, Ordering::Relaxed);
                    cx.notify();
                }))
                .child(row_label(title, description))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .flex_none()
                        .gap(px(theme::space::SM))
                        .px(px(theme::space::MD))
                        .h(px(32.0))
                        .max_w(px(280.0))
                        .rounded(px(theme::radius::ROW))
                        .bg(theme::rgba_alpha(theme::surface(), 0.72))
                        .border_1()
                        .border_color(if expanded {
                            theme::accent()
                        } else {
                            theme::rgba_alpha(theme::border(), 0.35)
                        })
                        .child(
                            div()
                                .truncate()
                                .text_size(theme::Text::Small.size())
                                .text_color(theme::text())
                                .child(current),
                        )
                        .child(
                            div()
                                .flex_none()
                                .text_size(theme::Text::Tiny.size())
                                .text_color(theme::text_muted())
                                .child(if expanded { "▴" } else { "▾" }),
                        ),
                ),
        )
        .when(expanded, |this| {
            this.child(
                div()
                    .mx(px(theme::space::MD))
                    .mb(px(theme::space::XS))
                    .p(px(theme::space::XS))
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .rounded(px(theme::radius::CARD))
                    .bg(theme::surface_elevated())
                    .border_1()
                    .border_color(theme::rgba_alpha(theme::border(), 0.38))
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
    description: String,
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
        .gap(px(6.0))
        .px(px(theme::space::MD))
        .py(px(theme::space::SM))
        .rounded(px(10.0))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .gap(px(theme::space::LG))
                .child(row_label(title, description))
                .child(
                    div()
                        .flex_none()
                        .text_size(theme::Text::Small.size())
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme::accent())
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
                    // 内缩 7px 轨道盒，确保 14px 滑块在 0% 与 100% 端点都不会溢出裁切
                    div()
                        .relative()
                        .h_full()
                        .mx(px(7.0))
                        .child(
                            div()
                                .absolute()
                                .left(px(0.0))
                                .right(px(0.0))
                                .top(px(9.0))
                                .h(px(4.0))
                                .rounded(px(2.0))
                                .bg(theme::surface_elevated()),
                        )
                        .child(
                            div()
                                .absolute()
                                .left(px(0.0))
                                .top(px(9.0))
                                .h(px(4.0))
                                .w(relative(ratio))
                                .rounded(px(2.0))
                                .bg(theme::accent()),
                        )
                        .child(
                            div()
                                .absolute()
                                .left(relative(ratio))
                                .ml(px(-7.0))
                                .top(px(4.0))
                                .size(px(14.0))
                                .rounded_full()
                                .bg(theme::accent_foreground())
                                .border_2()
                                .border_color(theme::accent()),
                        )
                        .child(
                            canvas(
                                move |rect: Bounds<gpui::Pixels>, _window, _cx| {
                                    if let Ok(mut slot) = bounds_for_canvas.lock() {
                                        *slot = Some(rect);
                                    }
                                },
                                |_bounds, _state, _window, _cx| {},
                            )
                            .absolute()
                            .size_full(),
                        ),
                )
                .child(
                    div()
                        .absolute()
                        .size_full()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |root, event: &MouseDownEvent, _window, cx| {
                                cx.stop_propagation();
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
        .when(primary, |this| this.font_weight(FontWeight::SEMIBOLD))
        .text_color(if primary {
            theme::accent_foreground()
        } else {
            theme::text()
        })
        .cursor_pointer()
        .hover(|style| style.opacity(0.88))
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

fn preview_prefetch_concurrency(root: &mut Root, value: f32, cx: &mut Context<Root>) {
    root.settings.prefetch_concurrency = value.round().clamp(1.0, 6.0) as usize;
    cx.notify();
}

fn commit_prefetch_concurrency(root: &mut Root, cx: &mut Context<Root>) {
    let value = root.settings.prefetch_concurrency;
    root.set_prefetch_concurrency(value, cx);
}

fn preview_audio_timeout(root: &mut Root, value: f32, cx: &mut Context<Root>) {
    root.settings.audio_download_timeout_seconds = value.round().clamp(15.0, 600.0) as u32;
    cx.notify();
}

fn commit_audio_timeout(root: &mut Root, cx: &mut Context<Root>) {
    let value = root.settings.audio_download_timeout_seconds.clamp(15, 600);
    root.settings.audio_download_timeout_seconds = value;
    root.status = match root.settings.save() {
        Ok(()) => root.localized("音频缓存等待时间：{} 秒", &[value.to_string()]),
        Err(err) => root.localized("缓存等待时间保存失败：{err}", &[err.to_string()]),
    };
    root.toast(root.status.clone(), cx);
    cx.notify();
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

fn theme_preview_swatch(
    id: &'static str,
    title: String,
    pal: theme::Palette,
    split_day_night: bool,
    selected: bool,
    listener: impl Fn(&ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> AnyElement {
    let dark_pal = theme::ThemeKind::Dark.palette();
    let light_pal = theme::ThemeKind::Light.palette();

    let preview_box = if split_day_night {
        div()
            .flex()
            .flex_row()
            .w_full()
            .h(px(50.0))
            .rounded(px(theme::radius::ROW))
            .overflow_hidden()
            .border_1()
            .border_color(theme::rgba_alpha(theme::border(), 0.35))
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .bg(light_pal.bg)
                    .p(px(6.0))
                    .flex()
                    .flex_col()
                    .justify_between()
                    .child(
                        div()
                            .h(px(5.0))
                            .w(px(24.0))
                            .rounded_full()
                            .bg(light_pal.accent),
                    )
                    .child(
                        div()
                            .h(px(12.0))
                            .w_full()
                            .rounded(px(4.0))
                            .bg(light_pal.surface_elevated),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .bg(dark_pal.bg)
                    .p(px(6.0))
                    .flex()
                    .flex_col()
                    .justify_between()
                    .child(
                        div()
                            .h(px(5.0))
                            .w(px(24.0))
                            .rounded_full()
                            .bg(dark_pal.accent),
                    )
                    .child(
                        div()
                            .h(px(12.0))
                            .w_full()
                            .rounded(px(4.0))
                            .bg(dark_pal.surface_elevated),
                    ),
            )
            .into_any_element()
    } else {
        div()
            .flex()
            .flex_row()
            .w_full()
            .h(px(50.0))
            .p(px(6.0))
            .gap(px(5.0))
            .rounded(px(theme::radius::ROW))
            .bg(pal.bg)
            .border_1()
            .border_color(pal.border)
            .child(
                div()
                    .w(px(20.0))
                    .h_full()
                    .rounded(px(4.0))
                    .bg(pal.surface)
                    .p(px(3.0))
                    .flex()
                    .flex_col()
                    .gap(px(3.0))
                    .child(div().h(px(4.0)).w_full().rounded_full().bg(pal.accent))
                    .child(
                        div()
                            .h(px(3.0))
                            .w(px(10.0))
                            .rounded_full()
                            .bg(pal.text_faint),
                    )
                    .child(
                        div()
                            .h(px(3.0))
                            .w(px(12.0))
                            .rounded_full()
                            .bg(pal.text_faint),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .justify_between()
                    .p(px(4.0))
                    .rounded(px(4.0))
                    .bg(pal.surface_elevated)
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .child(div().h(px(5.0)).w(px(36.0)).rounded_full().bg(pal.text))
                            .child(div().size(px(6.0)).rounded_full().bg(pal.accent_secondary)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(4.0))
                            .child(div().h(px(7.0)).w(px(24.0)).rounded_full().bg(pal.accent))
                            .child(
                                div()
                                    .h(px(7.0))
                                    .w(px(14.0))
                                    .rounded_full()
                                    .bg(pal.surface_hover),
                            ),
                    ),
            )
            .into_any_element()
    };

    div()
        .id(id)
        .flex()
        .flex_col()
        .gap(px(theme::space::SM))
        .w(px(154.0))
        .p(px(theme::space::SM))
        .rounded(px(12.0))
        .bg(theme::rgba_alpha(theme::surface_elevated(), 0.56))
        .border_1()
        .border_color(if selected {
            theme::accent()
        } else {
            theme::rgba_alpha(theme::border(), 0.28)
        })
        .when(selected, |this| this.bg(theme::surface_selected()))
        .cursor_pointer()
        .hover(|style| style.bg(theme::surface_hover()))
        .on_click(listener)
        .child(preview_box)
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .gap(px(theme::space::XS))
                .child(
                    div()
                        .truncate()
                        .text_size(theme::Text::Small.size())
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(if selected {
                            theme::accent()
                        } else {
                            theme::text()
                        })
                        .child(title),
                )
                .when(selected, |this| {
                    this.child(
                        div()
                            .size(px(16.0))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .bg(theme::accent())
                            .child(
                                svg()
                                    .path(icons::path("check"))
                                    .size(px(10.0))
                                    .text_color(theme::accent_foreground()),
                            ),
                    )
                }),
        )
        .into_any_element()
}

fn account_view(root: &Root, viewport_w: f32, cx: &mut Context<Root>) -> AnyElement {
    let logged_in = !root.settings.cookie.trim().is_empty();
    let pal = theme::current_theme().palette();
    let is_vip = logged_in && root.account.as_ref().is_some_and(|info| info.vip);

    let account_title = if logged_in {
        root.account
            .as_ref()
            .filter(|info| !info.nickname.trim().is_empty())
            .map(|info| info.nickname.clone())
            .unwrap_or_else(|| text(root, "已登录用户", "Signed-in User"))
    } else {
        text(root, "游客模式（未登录）", "Guest Mode (Not Signed In)")
    };

    let membership_badge = if !logged_in {
        text(root, "未登录", "Guest")
    } else if is_vip {
        text(root, "VIP 会员", "VIP Member")
    } else if root.account.is_some() {
        text(root, "普通用户", "Standard")
    } else {
        text(root, "同步中", "Syncing")
    };

    let account_detail = if let Some(info) = root.account.as_ref().filter(|_| logged_in) {
        format!("UID: {}", info.user_id)
    } else if logged_in {
        text(root, "正在读取账号详情…", "Loading account details…")
    } else {
        text(
            root,
            "使用汽水音乐 App 扫码登录",
            "Scan QR code with Qishui Music app",
        )
    };

    let account_avatar = root
        .account
        .as_ref()
        .and_then(|info| root.cover_of(&info.avatar_url));

    let signer = if root.settings.signer_url.trim().is_empty() {
        text(root, "内置签名链路", "Built-in device signing")
    } else {
        root.settings.signer_url.trim().to_string()
    };

    let device_ready = !root.settings.device_id.trim().is_empty()
        && !root.settings.iid.trim().is_empty()
        && !root.settings.fp.trim().is_empty();
    let vip_ready = root.settings.is_ready_for_vip();

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
            text(root, "扫码登录", "QR Sign In"),
            true,
            cx.listener(|root, _event: &ClickEvent, _window, cx| {
                root.login_modal_open = true;
                root.start_login(cx);
            }),
        )
    };

    let qishui_avatar_el = match account_avatar.filter(|p| p.exists()) {
        Some(path) => crate::ui::artwork::cover(Some(path), 60.0, theme::radius::PILL),
        None => div()
            .flex_none()
            .size(px(60.0))
            .rounded(px(16.0))
            .overflow_hidden()
            .child(img("icons/sodam-logo.svg").size(px(60.0)))
            .into_any_element(),
    };

    let hero_card = div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .justify_between()
        .gap(px(theme::space::LG))
        .p(px(theme::space::LG))
        .rounded(px(14.0))
        .bg(theme::rgba_alpha(theme::surface_elevated(), 0.56))
        .border_1()
        .border_color(if is_vip {
            theme::rgba_alpha(pal.accent_secondary, 0.55)
        } else {
            theme::rgba_alpha(theme::border(), 0.30)
        })
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .flex_1()
                .min_w(px(220.0))
                .gap(px(theme::space::LG))
                .child(qishui_avatar_el)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w(px(0.0))
                        .gap(px(4.0))
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .flex_wrap()
                                .items_center()
                                .gap(px(theme::space::SM))
                                .child(
                                    div()
                                        .text_size(theme::Text::Large.size())
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(theme::text())
                                        .child(account_title),
                                )
                                .child(
                                    div()
                                        .px(px(theme::space::SM))
                                        .py(px(2.0))
                                        .rounded(px(theme::radius::PILL))
                                        .bg(if is_vip {
                                            pal.accent_secondary
                                        } else {
                                            theme::surface()
                                        })
                                        .border_1()
                                        .border_color(if is_vip {
                                            pal.accent_secondary
                                        } else {
                                            theme::rgba_alpha(theme::border(), 0.35)
                                        })
                                        .text_size(theme::Text::Tiny.size())
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(if is_vip {
                                            theme::accent_foreground()
                                        } else {
                                            theme::text_muted()
                                        })
                                        .child(membership_badge),
                                ),
                        )
                        .child(
                            div()
                                .text_size(theme::Text::Small.size())
                                .text_color(theme::text_muted())
                                .child(account_detail),
                        ),
                ),
        )
        .child(account_action)
        .into_any_element();

    let status_dot_color = if vip_ready || device_ready {
        pal.accent_secondary
    } else {
        theme::danger()
    };

    let diagnostics_row = div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(px(theme::space::MD))
        .min_h(px(44.0))
        .px(px(theme::space::MD))
        .py(px(theme::space::SM))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(theme::space::SM))
                .child(div().size(px(8.0)).rounded_full().bg(status_dot_color))
                .child(
                    div()
                        .text_size(theme::Text::Body.size())
                        .text_color(theme::text())
                        .child(text(root, "签名服务", "Signing Pipeline")),
                ),
        )
        .child(
            div()
                .truncate()
                .text_size(theme::Text::Tiny.size())
                .text_color(theme::text_muted())
                .child(signer),
        )
        .into_any_element();

    let active_provider = root.active_provider();
    let provider_card = section_group(
        text(root, "音源切换", "Music Source"),
        vec![segmented_row(
            text(root, "当前音乐平台", "Active Platform"),
            "",
            vec![
                (
                    "settings-provider-soda",
                    text(root, "汽水音乐", "Qishui"),
                    active_provider == sodam_core::MusicProvider::Soda,
                    Box::new(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.set_music_provider(sodam_core::MusicProvider::Soda, cx);
                    })),
                ),
                (
                    "settings-provider-migu",
                    text(root, "咪咕音乐", "Migu"),
                    active_provider == sodam_core::MusicProvider::Migu,
                    Box::new(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.set_music_provider(sodam_core::MusicProvider::Migu, cx);
                    })),
                ),
            ],
        )],
    );

    let migu_logged_in = !root.settings.migu_cookie.trim().is_empty();
    let migu_vip = migu_logged_in && root.migu_account.as_ref().is_some_and(|info| info.vip);
    let migu_title = if migu_logged_in {
        root.migu_account
            .as_ref()
            .filter(|info| !info.nickname.trim().is_empty())
            .map(|info| info.nickname.clone())
            .unwrap_or_else(|| text(root, "咪咕音乐会员", "Migu Music User"))
    } else {
        text(root, "咪咕音乐", "Migu Music")
    };
    let migu_badge = if !migu_logged_in {
        text(root, "免登录可用", "Guest Ready")
    } else if migu_vip {
        text(root, "VIP · 24bit", "VIP · Hi-Res")
    } else if root.migu_account.is_some() {
        text(root, "普通用户", "Standard")
    } else {
        text(root, "同步中", "Syncing")
    };
    let migu_detail = if let Some(info) = root.migu_account.as_ref().filter(|_| migu_logged_in) {
        if info.user_id.trim().is_empty() {
            text(root, "咪咕直连就绪", "Migu session ready")
        } else {
            format!("UID: {}", info.user_id)
        }
    } else if migu_logged_in {
        text(
            root,
            "正在读取咪咕账号信息…",
            "Loading Migu account details…",
        )
    } else {
        text(
            root,
            "免登录即可搜索与听歌 · 登录解锁 VIP 与 24bit 无损",
            "Stream free · Sign in for VIP & 24-bit Hi-Res",
        )
    };
    let migu_avatar = root
        .migu_account
        .as_ref()
        .and_then(|info| root.cover_of(&info.avatar_url));
    let migu_avatar_el = match migu_avatar.filter(|p| p.exists()) {
        Some(path) => crate::ui::artwork::cover(Some(path), 60.0, theme::radius::PILL),
        None => div()
            .flex_none()
            .size(px(60.0))
            .rounded(px(16.0))
            .overflow_hidden()
            .child(img("icons/migu-logo.svg").size(px(60.0)))
            .into_any_element(),
    };

    let migu_card = div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .justify_between()
        .gap(px(theme::space::LG))
        .p(px(theme::space::LG))
        .rounded(px(14.0))
        .bg(theme::rgba_alpha(theme::surface_elevated(), 0.56))
        .border_1()
        .border_color(if migu_vip {
            theme::rgba_alpha(pal.accent_secondary, 0.55)
        } else {
            theme::rgba_alpha(theme::border(), 0.30)
        })
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .flex_1()
                .min_w(px(220.0))
                .gap(px(theme::space::LG))
                .child(migu_avatar_el)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w(px(0.0))
                        .gap(px(4.0))
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .flex_wrap()
                                .items_center()
                                .gap(px(theme::space::SM))
                                .child(
                                    div()
                                        .text_size(theme::Text::Large.size())
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(theme::text())
                                        .child(migu_title),
                                )
                                .child(
                                    div()
                                        .px(px(theme::space::SM))
                                        .py(px(2.0))
                                        .rounded(px(theme::radius::PILL))
                                        .bg(if migu_vip {
                                            pal.accent_secondary
                                        } else {
                                            theme::surface()
                                        })
                                        .border_1()
                                        .border_color(if migu_vip {
                                            pal.accent_secondary
                                        } else {
                                            theme::rgba_alpha(theme::border(), 0.35)
                                        })
                                        .text_size(theme::Text::Tiny.size())
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(if migu_vip {
                                            theme::accent_foreground()
                                        } else {
                                            theme::text_muted()
                                        })
                                        .child(migu_badge),
                                ),
                        )
                        .child(
                            div()
                                .text_size(theme::Text::Small.size())
                                .text_color(theme::text_muted())
                                .child(migu_detail),
                        ),
                ),
        )
        .child(if migu_logged_in {
            div()
                .flex()
                .flex_row()
                .gap(px(theme::space::SM))
                .child(action_button(
                    "migu-refresh-account",
                    text(root, "刷新状态", "Refresh"),
                    false,
                    cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.refresh_migu_account(cx);
                    }),
                ))
                .child(action_button(
                    "migu-account-logout",
                    text(root, "退出登录", "Sign out"),
                    false,
                    cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.logout_migu(cx);
                    }),
                ))
                .into_any_element()
        } else {
            action_button(
                "migu-account-login",
                text(root, "登录咪咕会员", "Sign In Migu"),
                true,
                cx.listener(|root, _event: &ClickEvent, _window, cx| {
                    root.open_migu_login_modal(cx);
                }),
            )
        })
        .into_any_element();

    let body = div()
        .flex()
        .flex_col()
        .gap(px(theme::space::XL))
        .child(page_header(text(root, "账号与会员", "Account & VIP")))
        .child(provider_card)
        .child(hero_card)
        .child(migu_card)
        .child(section_group(
            text(root, "连接状态", "Connection"),
            vec![diagnostics_row],
        ))
        .into_any_element();
    shell(root, viewport_w, body, cx)
}

fn general_view(root: &Root, viewport_w: f32, cx: &mut Context<Root>) -> AnyElement {
    let mut theme_cards = vec![theme_preview_swatch(
        "theme-card-system",
        text(root, "跟随系统", "Auto"),
        root.theme.palette(),
        true,
        root.theme_follows_system,
        cx.listener(|root, _event: &ClickEvent, _window, cx| {
            root.set_theme_system(cx);
        }),
    )];
    for kind in theme::ThemeKind::ALL {
        let card_id: &'static str = match kind {
            theme::ThemeKind::Dark => "theme-card-dark",
            theme::ThemeKind::Light => "theme-card-light",
            theme::ThemeKind::DanQing => "theme-card-danqing",
            theme::ThemeKind::ZhuSha => "theme-card-zhusha",
            theme::ThemeKind::ZhuYing => "theme-card-zhuying",
        };
        theme_cards.push(theme_preview_swatch(
            card_id,
            root.tr(kind.name_zh()).to_string(),
            kind.palette(),
            false,
            !root.theme_follows_system && root.theme == kind,
            cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                root.set_ui_theme(kind, cx);
            }),
        ));
    }

    let theme_gallery = div()
        .flex()
        .flex_row()
        .flex_wrap()
        .gap(px(theme::space::MD))
        .px(px(theme::space::MD))
        .py(px(theme::space::SM))
        .children(theme_cards)
        .into_any_element();

    let mode_value = root.tr(root.window_mode.name_zh()).to_string();
    let mode_options = theme::WindowMode::ALL
        .iter()
        .copied()
        .enumerate()
        .map(|(index, mode)| {
            option_row(
                format!("window-mode-option-{index}"),
                format!(
                    "{} · {}",
                    root.tr(mode.name_zh()),
                    root.tr(mode.subtitle_zh())
                ),
                root.window_mode == mode,
                cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                    root.set_window_mode(mode, cx);
                    OPEN_SELECT.store(0, Ordering::Relaxed);
                }),
            )
        })
        .collect::<Vec<_>>();

    let body = div()
        .flex()
        .flex_col()
        .gap(px(theme::space::XL))
        .child(page_header(text(root, "外观与主题", "Themes & Layout")))
        .child(section_group(
            text(root, "视觉主题", "Appearance"),
            vec![
                theme_gallery,
                toggle_row(
                    "dynamic-cover-color",
                    text(root, "封面动态流光染色", "Dynamic Cover Ambient Tint"),
                    text(
                        root,
                        "播放舞台与高亮色随当前专辑封面主色柔和过渡",
                        "Subtly tint stage accents from current album art",
                    ),
                    root.settings.dynamic_cover_color,
                    cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.set_dynamic_cover_color(!root.settings.dynamic_cover_color, cx);
                    }),
                ),
            ],
        ))
        .child(section_group(
            text(root, "界面与布局", "Layout & Language"),
            vec![
                select_row(
                    "window-mode-select",
                    1,
                    text(root, "窗口模式", "Window Mode"),
                    String::new(),
                    mode_value,
                    mode_options,
                    cx,
                ),
                segmented_row(
                    text(root, "播放页视图", "Playback View"),
                    "",
                    vec![
                        (
                            "playback-view-option-0",
                            root.tr(theme::PlaybackViewMode::Split.name_zh())
                                .to_string(),
                            root.playback_view_mode == theme::PlaybackViewMode::Split,
                            Box::new(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                root.set_playback_view_mode(theme::PlaybackViewMode::Split, cx);
                            })),
                        ),
                        (
                            "playback-view-option-1",
                            root.tr(theme::PlaybackViewMode::LyricsOnly.name_zh())
                                .to_string(),
                            root.playback_view_mode == theme::PlaybackViewMode::LyricsOnly,
                            Box::new(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                root.set_playback_view_mode(
                                    theme::PlaybackViewMode::LyricsOnly,
                                    cx,
                                );
                            })),
                        ),
                        (
                            "playback-view-option-2",
                            root.tr(theme::PlaybackViewMode::QueueOnly.name_zh())
                                .to_string(),
                            root.playback_view_mode == theme::PlaybackViewMode::QueueOnly,
                            Box::new(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                root.set_playback_view_mode(theme::PlaybackViewMode::QueueOnly, cx);
                            })),
                        ),
                    ],
                ),
                segmented_row(
                    text(root, "内容视图密度", "Content Density"),
                    "",
                    vec![
                        (
                            "density-option-0",
                            root.tr(theme::ContentDensity::LargeCard.name_zh())
                                .to_string(),
                            root.content_density == theme::ContentDensity::LargeCard,
                            Box::new(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                root.set_content_density(theme::ContentDensity::LargeCard, cx);
                            })),
                        ),
                        (
                            "density-option-1",
                            root.tr(theme::ContentDensity::List.name_zh()).to_string(),
                            root.content_density == theme::ContentDensity::List,
                            Box::new(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                root.set_content_density(theme::ContentDensity::List, cx);
                            })),
                        ),
                        (
                            "density-option-2",
                            root.tr(theme::ContentDensity::Compact.name_zh())
                                .to_string(),
                            root.content_density == theme::ContentDensity::Compact,
                            Box::new(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                root.set_content_density(theme::ContentDensity::Compact, cx);
                            })),
                        ),
                    ],
                ),
                toggle_row(
                    "sidebar-collapsed-toggle",
                    text(root, "收起左侧功能栏", "Compact Navigation Rail"),
                    String::new(),
                    root.settings.sidebar_collapsed,
                    cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.set_sidebar_collapsed(!root.settings.sidebar_collapsed, cx);
                    }),
                ),
                segmented_row(
                    text(root, "界面语言", "Language"),
                    "",
                    vec![
                        (
                            "language-option-0",
                            text(root, "跟随系统", "Auto"),
                            root.language_follows_system,
                            Box::new(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                root.language = Language::system_locale();
                                root.language_follows_system = true;
                                root.settings.language = "auto".to_string();
                                let _ = root.settings.save();
                                cx.notify();
                            })),
                        ),
                        (
                            "language-option-1",
                            "中文".to_string(),
                            !root.language_follows_system && root.language == Language::Chinese,
                            Box::new(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                root.set_language(Language::Chinese, cx);
                            })),
                        ),
                        (
                            "language-option-2",
                            "EN".to_string(),
                            !root.language_follows_system && root.language == Language::English,
                            Box::new(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                root.set_language(Language::English, cx);
                            })),
                        ),
                    ],
                ),
                slider_row(
                    "ui-font-size",
                    text(root, "界面字体大小", "UI Font Size"),
                    String::new(),
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
        .into_any_element();
    shell(root, viewport_w, body, cx)
}

fn playback_view(root: &Root, viewport_w: f32, cx: &mut Context<Root>) -> AnyElement {
    let quality = if root.settings.quality.trim().is_empty() {
        "auto"
    } else {
        root.settings.quality.as_str()
    };
    let quality_items = [
        ("auto", text(root, "自动择优", "Auto")),
        (
            "zq24",
            text(root, "臻品母带 24bit (咪咕)", "Hi-Res 24-bit (Migu)"),
        ),
        ("lossless", text(root, "无损 FLAC", "Lossless")),
        ("highest", text(root, "极高 (≈320k)", "Highest (≈320k)")),
        ("medium", text(root, "较高", "Medium")),
        ("low", text(root, "标准", "Standard")),
    ];
    let quality_value = quality_items
        .iter()
        .find(|(value, _)| *value == quality)
        .map(|(_, label)| label.clone())
        .unwrap_or_else(|| text(root, "自动择优", "Auto"));
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
        .gap(px(theme::space::XL))
        .child(page_header(text(root, "播放与音质", "Playback & Audio")))
        .child(section_group(
            text(root, "音质与输出", "Audio & Output"),
            vec![
                select_row(
                    "quality-select",
                    5,
                    text(root, "在线播放音质", "Streaming Quality"),
                    String::new(),
                    quality_value,
                    quality_options,
                    cx,
                ),
                select_row(
                    "audio-device-select",
                    6,
                    text(root, "音频输出设备", "Output Device"),
                    String::new(),
                    current_device,
                    device_options,
                    cx,
                ),
            ],
        ))
        .child(section_group(
            text(root, "听感调校", "Transitions & DSP"),
            vec![
                toggle_row(
                    "normalize-volume",
                    text(root, "响度标准化", "Loudness Normalization"),
                    text(
                        root,
                        "平衡不同曲目间的音量落差",
                        "Match perceived loudness across tracks",
                    ),
                    root.settings.normalize_volume,
                    cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.e3_set_normalize_volume(!root.settings.normalize_volume, cx);
                    }),
                ),
                toggle_row(
                    "gapless-playback",
                    text(root, "无缝衔接播放", "Gapless Playback"),
                    String::new(),
                    root.settings.gapless_playback,
                    cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.e3_set_gapless(!root.settings.gapless_playback, cx);
                    }),
                ),
                slider_row(
                    "crossfade-slider",
                    text(root, "曲目交叉淡化", "Crossfade"),
                    String::new(),
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
    shell(root, viewport_w, body, cx)
}

fn lyrics_preview_card(root: &Root) -> AnyElement {
    let font_family = theme::LyricsFontFamily::from_key(&root.settings.lyrics_font_family);
    let align = theme::LyricsAlign::from_key(&root.settings.lyrics_align);
    let active_size = root.settings.lyrics_font_size.clamp(14, 30) as f32;
    let surrounding_size = (active_size * 0.78).max(12.0);
    let preview_line_h = (root.settings.lyrics_line_height.clamp(24, 52) as f32).min(46.0);

    let line = |txt: String, size: f32, color: gpui::Rgba, bold: bool| -> AnyElement {
        let base = div()
            .w_full()
            .h(px(preview_line_h))
            .flex()
            .items_center()
            .px(px(theme::space::MD))
            .rounded(px(theme::radius::ROW))
            .font_family(font_family.font_name())
            .text_size(px(size))
            .text_color(color)
            .when(bold, |this| this.font_weight(FontWeight::BOLD));
        let aligned = match align {
            theme::LyricsAlign::Left => base.justify_start().text_left(),
            theme::LyricsAlign::Center => base.justify_center().text_center(),
            theme::LyricsAlign::Right => base.justify_end().text_right(),
        };
        aligned.child(txt).into_any_element()
    };

    section_group(
        text(root, "舞台排版预览", "Stage Preview"),
        vec![div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .mx(px(theme::space::SM))
            .my(px(theme::space::XS))
            .px(px(theme::space::LG))
            .py(px(theme::space::MD))
            .rounded(px(12.0))
            .bg(theme::rgba_alpha(theme::surface(), 0.42))
            .child(line(
                text(
                    root,
                    "青瓷映月色，微风拂竹声",
                    "Celadon reflects moonlight through bamboo",
                ),
                surrounding_size,
                theme::rgba_alpha(theme::text(), 0.42),
                false,
            ))
            .child(line(
                text(
                    root,
                    "一曲听罢意犹长 · 当前高亮演唱行",
                    "A gentle melody lingers · Active highlighted line",
                ),
                active_size,
                theme::stage_accent(),
                true,
            ))
            .child(line(
                text(
                    root,
                    "山河入画卷，清音伴君行",
                    "Ink landscapes unfold along with calm notes",
                ),
                surrounding_size,
                theme::rgba_alpha(theme::text(), 0.42),
                false,
            ))
            .into_any_element()],
    )
}

fn lyrics_view(root: &Root, viewport_w: f32, cx: &mut Context<Root>) -> AnyElement {
    let font_family = theme::LyricsFontFamily::from_key(&root.settings.lyrics_font_family);
    let align = theme::LyricsAlign::from_key(&root.settings.lyrics_align);

    let quick_offset_controls = div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap(px(theme::space::SM))
        .px(px(theme::space::MD))
        .py(px(theme::space::SM))
        .child(action_button(
            "lyrics-offset-earlier",
            text(root, "提前 250 ms", "250 ms Earlier"),
            false,
            cx.listener(|root, _event: &ClickEvent, _window, cx| {
                let next = root.settings.lyrics_offset_ms - 250;
                root.set_lyrics_offset_ms(next, cx);
            }),
        ))
        .child(action_button(
            "lyrics-offset-reset",
            text(root, "偏移归零", "Reset Offset"),
            false,
            cx.listener(|root, _event: &ClickEvent, _window, cx| {
                root.set_lyrics_offset_ms(0, cx);
            }),
        ))
        .child(action_button(
            "lyrics-offset-later",
            text(root, "延后 250 ms", "250 ms Later"),
            false,
            cx.listener(|root, _event: &ClickEvent, _window, cx| {
                let next = root.settings.lyrics_offset_ms + 250;
                root.set_lyrics_offset_ms(next, cx);
            }),
        ))
        .into_any_element();

    let mut panel_rows = vec![toggle_row(
        "panel-lyrics-enabled-toggle",
        text(
            root,
            "向 Liquid Shell 推送实时歌词",
            "Push Live Lyrics to Liquid Shell",
        ),
        text(
            root,
            "通过 D-Bus 推送歌词与主题色，位置与外观请在 Liquid Shell 管理器调整",
            "Send lyrics & accent color over D-Bus; configure layout in Liquid Shell Manager.",
        ),
        root.settings.panel_lyrics_enabled,
        cx.listener(|root, _event: &ClickEvent, _window, cx| {
            root.settings.panel_lyrics_enabled = !root.settings.panel_lyrics_enabled;
            let _ = root.settings.save();
            cx.notify();
        }),
    )];

    if root.settings.panel_lyrics_enabled {
        panel_rows.push(segmented_row(
            text(root, "强调色", "Accent Color"),
            "",
            vec![
                (
                    "panel-color-0",
                    text(root, "主题色", "Accent"),
                    !matches!(root.settings.panel_lyrics_color.as_str(), "text" | "white"),
                    Box::new(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.settings.panel_lyrics_color = "accent".to_string();
                        let _ = root.settings.save();
                        cx.notify();
                    })),
                ),
                (
                    "panel-color-1",
                    text(root, "文字色", "Text"),
                    root.settings.panel_lyrics_color == "text",
                    Box::new(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.settings.panel_lyrics_color = "text".to_string();
                        let _ = root.settings.save();
                        cx.notify();
                    })),
                ),
                (
                    "panel-color-2",
                    text(root, "纯白", "White"),
                    root.settings.panel_lyrics_color == "white",
                    Box::new(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.settings.panel_lyrics_color = "white".to_string();
                        let _ = root.settings.save();
                        cx.notify();
                    })),
                ),
            ],
        ));
    }

    let body = div()
        .flex()
        .flex_col()
        .gap(px(theme::space::XL))
        .child(page_header(text(root, "歌词体验", "Lyrics Experience")))
        .child(lyrics_preview_card(root))
        .child(section_group(
            text(root, "播放页歌词", "Stage Lyrics"),
            vec![
                segmented_row(
                    text(root, "歌词字体", "Font"),
                    "",
                    vec![
                        (
                            "lyrics-font-family-0",
                            root.tr(theme::LyricsFontFamily::Sans.name_zh()).to_string(),
                            font_family == theme::LyricsFontFamily::Sans,
                            Box::new(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                root.set_lyrics_font_family(theme::LyricsFontFamily::Sans, cx);
                            })),
                        ),
                        (
                            "lyrics-font-family-1",
                            root.tr(theme::LyricsFontFamily::Serif.name_zh())
                                .to_string(),
                            font_family == theme::LyricsFontFamily::Serif,
                            Box::new(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                root.set_lyrics_font_family(theme::LyricsFontFamily::Serif, cx);
                            })),
                        ),
                        (
                            "lyrics-font-family-2",
                            root.tr(theme::LyricsFontFamily::Mono.name_zh()).to_string(),
                            font_family == theme::LyricsFontFamily::Mono,
                            Box::new(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                root.set_lyrics_font_family(theme::LyricsFontFamily::Mono, cx);
                            })),
                        ),
                    ],
                ),
                segmented_row(
                    text(root, "对齐方式", "Alignment"),
                    "",
                    vec![
                        (
                            "lyrics-align-0",
                            root.tr(theme::LyricsAlign::Left.name_zh()).to_string(),
                            align == theme::LyricsAlign::Left,
                            Box::new(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                root.set_lyrics_align(theme::LyricsAlign::Left, cx);
                            })),
                        ),
                        (
                            "lyrics-align-1",
                            root.tr(theme::LyricsAlign::Center.name_zh()).to_string(),
                            align == theme::LyricsAlign::Center,
                            Box::new(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                root.set_lyrics_align(theme::LyricsAlign::Center, cx);
                            })),
                        ),
                        (
                            "lyrics-align-2",
                            root.tr(theme::LyricsAlign::Right.name_zh()).to_string(),
                            align == theme::LyricsAlign::Right,
                            Box::new(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                root.set_lyrics_align(theme::LyricsAlign::Right, cx);
                            })),
                        ),
                    ],
                ),
                slider_row(
                    "lyrics-font-slider",
                    text(root, "歌词字号", "Font Size"),
                    String::new(),
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
                    text(root, "歌词行距", "Line Spacing"),
                    String::new(),
                    root.settings.lyrics_line_height as f32,
                    24.0,
                    52.0,
                    1.0,
                    format!("{} px", root.settings.lyrics_line_height),
                    preview_lyrics_line,
                    commit_lyrics_line,
                    cx,
                ),
                slider_row(
                    "lyrics-offset-slider",
                    text(root, "时间偏移", "Timing Offset"),
                    String::new(),
                    root.settings.lyrics_offset_ms as f32,
                    -3000.0,
                    3000.0,
                    50.0,
                    format!("{} ms", root.settings.lyrics_offset_ms),
                    preview_lyrics_offset,
                    commit_lyrics_offset,
                    cx,
                ),
                quick_offset_controls,
                toggle_row(
                    "bottom-lyrics-toggle",
                    text(root, "底部播放栏内联歌词", "Bottom Bar Inline Lyrics"),
                    String::new(),
                    root.settings.bottom_lyrics_enabled,
                    cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.set_bottom_lyrics_enabled(!root.settings.bottom_lyrics_enabled, cx);
                    }),
                ),
            ],
        ))
        .child(section_group(
            text(
                root,
                "Liquid Shell 灵动岛联动",
                "Liquid Shell Dynamic Island",
            ),
            panel_rows,
        ))
        .child(section_group(
            text(root, "桌面悬浮歌词", "Floating Desktop Lyrics"),
            vec![
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .min_h(px(44.0))
                    .px(px(theme::space::MD))
                    .py(px(theme::space::SM))
                    .child(row_label(
                        text(root, "悬浮窗口", "Overlay Window"),
                        String::new(),
                    ))
                    .child(action_button(
                        "toggle-desktop-lyrics-btn",
                        if root.desktop_lyrics_open {
                            text(root, "关闭桌面歌词", "Close")
                        } else {
                            text(root, "开启桌面歌词", "Open")
                        },
                        !root.desktop_lyrics_open,
                        cx.listener(|root, _event: &ClickEvent, _window, cx| {
                            root.open_desktop_lyrics(cx);
                        }),
                    ))
                    .into_any_element(),
                toggle_row(
                    "desktop-single-line",
                    text(root, "单行显示", "Single Line"),
                    String::new(),
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
                    text(root, "始终置顶", "Always on Top"),
                    String::new(),
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
                    text(root, "锁定位置", "Lock Position"),
                    String::new(),
                    root.settings.desktop_lyrics_locked,
                    cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.e3_set_desktop_locked(!root.settings.desktop_lyrics_locked, cx);
                    }),
                ),
                toggle_row(
                    "desktop-click-through",
                    text(root, "鼠标穿透", "Click-Through"),
                    String::new(),
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
                    text(root, "背景透明度", "Opacity"),
                    String::new(),
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
                    text(root, "悬浮字号", "Font Size"),
                    String::new(),
                    root.settings.desktop_lyrics_font_size as f32,
                    20.0,
                    44.0,
                    1.0,
                    format!("{} px", root.settings.desktop_lyrics_font_size),
                    preview_desktop_font,
                    commit_desktop_font,
                    cx,
                ),
                segmented_row(
                    text(root, "文字颜色", "Text Color"),
                    "",
                    vec![
                        (
                            "desktop-color-0",
                            text(root, "强调色", "Accent"),
                            root.settings.desktop_lyrics_color != "text",
                            Box::new(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                root.e3_set_desktop_color("accent", cx);
                            })),
                        ),
                        (
                            "desktop-color-1",
                            text(root, "文字色", "Text"),
                            root.settings.desktop_lyrics_color == "text",
                            Box::new(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                root.e3_set_desktop_color("text", cx);
                            })),
                        ),
                    ],
                ),
                segmented_row(
                    text(root, "水平对齐", "Alignment"),
                    "",
                    vec![
                        (
                            "desktop-align-0",
                            text(root, "左对齐", "Left"),
                            root.settings.desktop_lyrics_align == "left",
                            Box::new(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                root.e3_set_desktop_align("left", cx);
                            })),
                        ),
                        (
                            "desktop-align-1",
                            text(root, "居中", "Center"),
                            !matches!(
                                root.settings.desktop_lyrics_align.as_str(),
                                "left" | "right"
                            ),
                            Box::new(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                root.e3_set_desktop_align("center", cx);
                            })),
                        ),
                        (
                            "desktop-align-2",
                            text(root, "右对齐", "Right"),
                            root.settings.desktop_lyrics_align == "right",
                            Box::new(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                root.e3_set_desktop_align("right", cx);
                            })),
                        ),
                    ],
                ),
            ],
        ))
        .into_any_element();
    shell(root, viewport_w, body, cx)
}

fn cache_stat_tile(label: String, value: String, sub: String, accent: gpui::Rgba) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w(px(150.0))
        .gap(px(4.0))
        .p(px(theme::space::MD))
        .rounded(px(theme::radius::ROW))
        .bg(theme::surface_elevated())
        .border_1()
        .border_color(theme::border())
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(div().size(px(8.0)).rounded_full().bg(accent))
                .child(
                    div()
                        .text_size(theme::Text::Tiny.size())
                        .text_color(theme::text_muted())
                        .child(label),
                ),
        )
        .child(
            div()
                .text_size(theme::Text::Large.size())
                .font_weight(FontWeight::BOLD)
                .text_color(theme::text())
                .child(value),
        )
        .child(
            div()
                .text_size(theme::Text::Tiny.size())
                .text_color(theme::text_faint())
                .child(sub),
        )
        .into_any_element()
}

fn storage_view(root: &Root, viewport_w: f32, cx: &mut Context<Root>) -> AnyElement {
    let quality_items = [
        (
            "follow",
            text(root, "跟随播放音质", "Follow playback quality"),
        ),
        (
            "zq24",
            text(root, "臻品母带 24bit (咪咕)", "Hi-Res 24-bit (Migu)"),
        ),
        ("lossless", text(root, "无损 FLAC", "Lossless")),
        ("highest", text(root, "极高 (≈320k)", "Highest")),
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

    let download_path = sodam_core::downloads::download_dir().display().to_string();

    let (audio_bytes, audio_files, cover_bytes, cover_files) = root.cache_summary;
    let total = audio_bytes.saturating_add(cover_bytes);
    let pal = theme::current_theme().palette();
    let cache_limit = if root.settings.cache_limit_gb == 0 {
        text(root, "不限制", "Unlimited")
    } else {
        format!("{} GB", root.settings.cache_limit_gb)
    };

    let (audio_ratio, cover_ratio) = if root.settings.cache_limit_gb > 0 {
        let denom = (root.settings.cache_limit_gb as f64) * 1024.0 * 1024.0 * 1024.0;
        let a = (audio_bytes as f64 / denom.max(1.0)).clamp(0.0, 1.0) as f32;
        let c = (cover_bytes as f64 / denom.max(1.0)).clamp(0.0, (1.0 - a).max(0.0) as f64) as f32;
        (a, c)
    } else if total > 0 {
        let a = (audio_bytes as f64 / total as f64).clamp(0.0, 1.0) as f32;
        let c = (1.0 - a).max(0.0);
        (a, c)
    } else {
        (0.0, 0.0)
    };

    let storage_meter = div()
        .flex()
        .flex_col()
        .gap(px(theme::space::MD))
        .px(px(theme::space::XS))
        .child(
            div()
                .flex()
                .flex_row()
                .flex_wrap()
                .gap(px(theme::space::SM))
                .child(cache_stat_tile(
                    text(root, "已用总空间", "Total Used"),
                    human_bytes(total),
                    if root.settings.cache_limit_gb > 0 {
                        format!("{} {}", text(root, "上限", "Limit"), cache_limit)
                    } else {
                        text(root, "未限制上限", "No hard cap")
                    },
                    theme::text_muted(),
                ))
                .child(cache_stat_tile(
                    text(root, "歌曲音频缓存", "Audio Cache"),
                    human_bytes(audio_bytes),
                    format!("{audio_files} {}", text(root, "首已缓存", "tracks cached")),
                    theme::accent(),
                ))
                .child(cache_stat_tile(
                    text(root, "专辑封面缓存", "Cover Cache"),
                    human_bytes(cover_bytes),
                    format!("{cover_files} {}", text(root, "张封面", "covers cached")),
                    pal.accent_secondary,
                )),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .w_full()
                .h(px(10.0))
                .rounded_full()
                .overflow_hidden()
                .bg(theme::surface_elevated())
                .border_1()
                .border_color(theme::border())
                .when(audio_ratio > 0.0, |this| {
                    this.child(div().h_full().w(relative(audio_ratio)).bg(theme::accent()))
                })
                .when(cover_ratio > 0.0, |this| {
                    this.child(
                        div()
                            .h_full()
                            .w(relative(cover_ratio))
                            .bg(pal.accent_secondary),
                    )
                }),
        )
        .into_any_element();

    let mut prefetch_controls = vec![
        toggle_row(
            "offline-mode",
            text(root, "离线模式", "Offline Mode"),
            text(
                root,
                "开启后仅播放本地已缓存歌曲，完全禁止发起新的音频网络请求。",
                "Only play locally cached songs and block all new audio network requests.",
            ),
            root.settings.offline_mode,
            cx.listener(|root, _event: &ClickEvent, _window, cx| {
                root.set_offline_mode(!root.settings.offline_mode, cx);
            }),
        ),
        toggle_row(
            "adaptive-prefetch",
            text(root, "自适应预加载", "Adaptive Prefetch"),
            text(
                root,
                "根据队列后续歌曲时长自动估算缓存深度，保障长途听歌无感切歌。",
                "Dynamically size prefetch count from upcoming track durations.",
            ),
            root.settings.prefetch_adaptive,
            cx.listener(|root, _event: &ClickEvent, _window, cx| {
                root.set_prefetch_adaptive(!root.settings.prefetch_adaptive, cx);
            }),
        ),
    ];
    if root.settings.prefetch_adaptive {
        prefetch_controls.push(slider_row(
            "prefetch-minutes-slider",
            text(root, "预加载目标时长", "Prefetch Target Duration"),
            text(
                root,
                "希望前方队列始终提前准备好的播放时长（5–30 分钟）。",
                "Minimum duration of upcoming audio to keep prepared ahead.",
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
    }
    prefetch_controls.push(slider_row(
        "prefetch-count-slider",
        text(root, "缓存歌曲数量", "Prefetch Track Count"),
        text(
            root,
            "自适应模式下为最低保底曲目数；固定模式下为提前缓存首数（0 关闭）。",
            "Minimum floor in adaptive mode, or fixed track count ahead (0 disables).",
        ),
        root.settings.prefetch_count as f32,
        0.0,
        24.0,
        1.0,
        root.settings.prefetch_count.to_string(),
        preview_prefetch_count,
        commit_prefetch_count,
        cx,
    ));
    prefetch_controls.push(slider_row(
        "prefetch-concurrency-slider",
        text(root, "缓存并行数", "Prefetch Concurrency"),
        text(
            root,
            "后台同时缓存的音频任务数量（1–6），弱网环境可调低提升当前曲目优先级。",
            "Concurrent background prefetch tasks (1–6); lower on slower networks.",
        ),
        root.settings.prefetch_concurrency as f32,
        1.0,
        6.0,
        1.0,
        root.settings.prefetch_concurrency.to_string(),
        preview_prefetch_concurrency,
        commit_prefetch_concurrency,
        cx,
    ));
    prefetch_controls.push(slider_row(
        "audio-timeout-slider",
        text(root, "缓存等待时间", "Audio Fetch Timeout"),
        text(
            root,
            "单首歌曲连接与读取的最长等待时间（15–600 秒）。",
            "Maximum connection and read timeout per track (15–600 seconds).",
        ),
        root.settings.audio_download_timeout_seconds as f32,
        15.0,
        600.0,
        15.0,
        format!("{} s", root.settings.audio_download_timeout_seconds),
        preview_audio_timeout,
        commit_audio_timeout,
        cx,
    ));

    let body = div()
        .flex()
        .flex_col()
        .gap(px(theme::space::XL))
        .child(page_header(text(root, "下载与存储", "Downloads & Storage")))
        .child(section_group(
            text(root, "下载与导出", "Downloads"),
            vec![
                select_row(
                    "download-quality-select",
                    13,
                    text(root, "下载音质", "Download Quality"),
                    String::new(),
                    quality_value,
                    quality_options,
                    cx,
                ),
                segmented_row(
                    text(root, "下载格式", "Download Format"),
                    "",
                    vec![
                        (
                            "download-format-0",
                            text(root, "原始容器", "Original"),
                            !matches!(root.settings.download_format.as_str(), "mp3" | "flac"),
                            Box::new(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                root.set_download_format("source", cx);
                            })),
                        ),
                        (
                            "download-format-1",
                            "MP3".to_string(),
                            root.settings.download_format == "mp3",
                            Box::new(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                root.set_download_format("mp3", cx);
                            })),
                        ),
                        (
                            "download-format-2",
                            "FLAC".to_string(),
                            root.settings.download_format == "flac",
                            Box::new(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                root.set_download_format("flac", cx);
                            })),
                        ),
                    ],
                ),
                div()
                    .flex()
                    .flex_col()
                    .gap(px(theme::space::SM))
                    .p(px(theme::space::MD))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .flex_wrap()
                            .items_center()
                            .justify_between()
                            .gap(px(theme::space::MD))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .min_w(px(180.0))
                                    .gap(px(2.0))
                                    .child(
                                        div()
                                            .text_size(theme::Text::Body.size())
                                            .text_color(theme::text())
                                            .child(text(root, "保存目录", "Save Directory")),
                                    )
                                    .child(
                                        div()
                                            .truncate()
                                            .text_size(theme::Text::Tiny.size())
                                            .text_color(theme::text_muted())
                                            .child(download_path),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .flex_wrap()
                                    .gap(px(theme::space::SM))
                                    .child(action_button(
                                        "choose-download-dir",
                                        text(root, "更改", "Change"),
                                        true,
                                        cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                            root.choose_download_directory(cx);
                                        }),
                                    ))
                                    .child(action_button(
                                        "open-download-dir",
                                        text(root, "打开", "Open"),
                                        false,
                                        cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                            root.open_download_folder(cx);
                                        }),
                                    ))
                                    .child(action_button(
                                        "reset-download-dir",
                                        text(root, "默认", "Reset"),
                                        false,
                                        cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                            root.reset_download_directory(cx);
                                        }),
                                    )),
                            ),
                    )
                    .into_any_element(),
            ],
        ))
        .child(section_group(
            text(root, "缓存空间", "Cache Storage"),
            vec![
                div()
                    .px(px(theme::space::MD))
                    .py(px(theme::space::SM))
                    .child(storage_meter)
                    .into_any_element(),
                slider_row(
                    "cache-limit-slider",
                    text(root, "缓存上限", "Cache Limit"),
                    text(
                        root,
                        "超出上限后按最近使用顺序自动滚动清理，0 GB 不限制",
                        "Rolling LRU eviction above limit (0 GB for unlimited)",
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
                    .px(px(theme::space::MD))
                    .py(px(theme::space::SM))
                    .child(action_button(
                        "refresh-cache-stats",
                        text(root, "刷新统计", "Refresh Stats"),
                        false,
                        cx.listener(|root, _event: &ClickEvent, _window, cx| {
                            root.refresh_cache_stats(cx);
                            root.refresh_audio_cache_index(cx);
                        }),
                    ))
                    .child(action_button(
                        "clear-audio-cache",
                        text(root, "清除歌曲缓存", "Clear Audio Cache"),
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
        .child(section_group(
            text(root, "离线与预加载", "Offline & Prefetch"),
            prefetch_controls,
        ))
        .into_any_element();
    shell(root, viewport_w, body, cx)
}

fn about_view(root: &Root, viewport_w: f32, cx: &mut Context<Root>) -> AnyElement {
    let version = env!("CARGO_PKG_VERSION");
    let can_install = matches!(&root.update_state, UpdateState::Available(_));
    let in_progress = matches!(
        &root.update_state,
        UpdateState::Checking | UpdateState::Installing
    );
    let update_status = match &root.update_state {
        UpdateState::Idle => text(
            root,
            "仅检查当前定制分叉的 GitHub Release（SHA-256 校验安装）。",
            "Checks GitHub Releases for this customized fork with SHA-256 verification.",
        ),
        UpdateState::Checking => text(root, "正在检查 GitHub 更新…", "Checking GitHub Releases…"),
        UpdateState::Available(release) => format!(
            "{} {}",
            text(root, "发现新版本：", "New version available:"),
            release.tag
        ),
        UpdateState::Installing => text(
            root,
            "正在下载、校验并请求系统授权安装…",
            "Downloading, verifying SHA-256, and requesting installation authorization…",
        ),
        UpdateState::UpToDate => text(root, "当前已是最新版本", "Already up to date"),
        UpdateState::Installed => text(
            root,
            "更新安装成功，请退出并重新打开流光音乐。",
            "Installed successfully. Please quit and reopen Lumina.",
        ),
        UpdateState::Error(err) => format!("{} {err}", text(root, "更新失败：", "Update failed:")),
    };

    let brand_card = section_group(
        text(root, "关于应用", "About"),
        vec![div()
            .flex()
            .flex_row()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(theme::space::MD))
            .p(px(theme::space::MD))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(theme::space::MD))
                    .child(img("icons/sodam-logo.svg").size(px(44.0)))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .gap(px(theme::space::SM))
                                    .child(
                                        div()
                                            .text_size(theme::Text::Large.size())
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(theme::text())
                                            .child(text(root, "流光音乐 · Lumina", "Lumina")),
                                    )
                                    .child(
                                        div()
                                            .px(px(theme::space::SM))
                                            .py(px(1.0))
                                            .rounded(px(theme::radius::PILL))
                                            .bg(theme::accent_soft())
                                            .text_size(theme::Text::Tiny.size())
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(theme::accent())
                                            .child(format!("v{version}")),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(theme::Text::Tiny.size())
                                    .text_color(theme::text_muted())
                                    .child("https://github.com/lbg2002/sodam · AGPL-3.0"),
                            ),
                    ),
            )
            .child(action_button(
                "about-open-github",
                text(root, "访问 GitHub", "GitHub"),
                false,
                cx.listener(|root, _event: &ClickEvent, _window, cx| {
                    root.open_github_repo(cx);
                }),
            ))
            .into_any_element()],
    );

    let body = div()
        .flex()
        .flex_col()
        .gap(px(theme::space::XL))
        .child(page_header(text(root, "系统与关于", "System & About")))
        .child(section_group(
            text(root, "系统与性能", "System & Performance"),
            vec![
                toggle_row(
                    "system-notifications",
                    text(root, "切歌系统通知", "Track-Change Notifications"),
                    String::new(),
                    root.settings.system_notifications,
                    cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.e3_set_notifications(!root.settings.system_notifications, cx);
                    }),
                ),
                toggle_row(
                    "lazy-startup",
                    text(root, "延迟加载非首屏内容", "Lazy-Load Startup"),
                    String::new(),
                    root.settings.lazy_startup,
                    cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.e3_set_lazy_startup(!root.settings.lazy_startup, cx);
                    }),
                ),
                toggle_row(
                    "reduced-motion",
                    text(root, "减弱动效（低性能模式）", "Reduced Motion"),
                    String::new(),
                    root.settings.reduced_motion,
                    cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.set_reduced_motion(!root.settings.reduced_motion, cx);
                    }),
                ),
                slider_row(
                    "compact-width-slider",
                    text(root, "播放栏折叠阈值", "Player Compact Threshold"),
                    String::new(),
                    root.settings.player_bar_compact_width as f32,
                    720.0,
                    1200.0,
                    10.0,
                    format!("{} px", root.settings.player_bar_compact_width),
                    preview_compact_width,
                    commit_compact_width,
                    cx,
                ),
            ],
        ))
        .child(section_group(
            text(root, "软件更新", "Software Updates"),
            vec![div()
                .flex()
                .flex_row()
                .flex_wrap()
                .items_center()
                .justify_between()
                .gap(px(theme::space::MD))
                .px(px(theme::space::MD))
                .py(px(theme::space::SM))
                .child(
                    div()
                        .text_size(theme::Text::Small.size())
                        .text_color(theme::text_muted())
                        .child(update_status),
                )
                .when(!in_progress, |this| {
                    this.child(action_button(
                        "about-check-update",
                        if can_install {
                            text(root, "下载并安装更新", "Download & Install")
                        } else {
                            text(root, "检查更新", "Check for Updates")
                        },
                        can_install,
                        cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                            if can_install {
                                root.install_available_update(cx);
                            } else {
                                root.check_for_updates(cx);
                            }
                        }),
                    ))
                })
                .into_any_element()],
        ))
        .child(brand_card)
        .into_any_element();
    shell(root, viewport_w, body, cx)
}

pub(crate) fn settings_view(root: &Root, window: &Window, cx: &mut Context<Root>) -> AnyElement {
    let viewport_w = f32::from(window.viewport_size().width).max(320.0);
    match root.settings_section {
        SettingsSection::Account => account_view(root, viewport_w, cx),
        SettingsSection::General => general_view(root, viewport_w, cx),
        SettingsSection::Playback => playback_view(root, viewport_w, cx),
        SettingsSection::Lyrics => lyrics_view(root, viewport_w, cx),
        SettingsSection::Storage => storage_view(root, viewport_w, cx),
        SettingsSection::About => about_view(root, viewport_w, cx),
    }
}
