//! 右侧内容区路由与共享列表组件。

use super::*;
use crate::app::SettingsSection;
use crate::ui::i18n::Language;

/// 字节 → 人类可读（KB / MB / GB）。
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

fn settings_nav_item(
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

fn settings_heading(title: &str, description: &str) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(theme::space::XS))
        .child(
            div()
                .text_size(theme::Text::Large.size())
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme::text())
                .child(title.to_string()),
        )
        .child(
            div()
                .text_size(theme::Text::Small.size())
                .text_color(theme::text_muted())
                .child(description.to_string()),
        )
        .into_any_element()
}


fn toggle_setting_row(
    id: &'static str,
    title: &str,
    description: &str,
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
                        .child(title.to_string()),
                )
                .child(
                    div()
                        .text_size(theme::Text::Tiny.size())
                        .text_color(theme::text_muted())
                        .child(description.to_string()),
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
                .child(
                    div()
                        .size(px(16.0))
                        .rounded_full()
                        .bg(if enabled {
                            theme::accent_foreground()
                        } else {
                            theme::text_faint()
                        }),
                ),
        )
        .into_any_element()
}

fn choice_setting_row(
    id: String,
    title: String,
    description: String,
    selected: bool,
    listener: impl Fn(&ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> AnyElement {
    div()
        .id(gpui::ElementId::Name(id.into()))
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(px(theme::space::LG))
        .px(px(theme::space::MD))
        .py(px(theme::space::SM))
        .rounded(px(theme::radius::ROW))
        .cursor_pointer()
        .when(selected, |this| this.bg(theme::surface_selected()))
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
                .when(!description.is_empty(), |this| {
                    this.child(
                        div()
                            .text_size(theme::Text::Tiny.size())
                            .text_color(theme::text_muted())
                            .child(description),
                    )
                }),
        )
        .when(selected, |this| {
            this.child(
                svg()
                    .path(icons::path("check"))
                    .size(px(theme::ICON))
                    .text_color(theme::accent()),
            )
        })
        .into_any_element()
}

/// 设置页：音质偏好选择 + 账号信息 + 其他。
/// 设置页 → 账户板块：头像 + 昵称/ID/VIP；未登录给「去登录」并弹二维码 modal。
fn account_section(root: &Root, cx: &mut Context<Root>) -> AnyElement {
    let logged_in = !root.settings.cookie.trim().is_empty();
    let avatar_path = root
        .account
        .as_ref()
        .filter(|_| logged_in)
        .map(|info| info.avatar_url.clone())
        .filter(|url| !url.trim().is_empty())
        .and_then(|url| root.cover_of(&url));

    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(theme::space::LG))
        .p(px(theme::space::LG))
        .rounded(px(theme::radius::CARD))
        .bg(theme::surface_elevated())
        .border_1()
        .border_color(theme::border())
        .child(if logged_in {
            cover(avatar_path, 64.0, 999.0)
        } else {
            cover(None, 64.0, 999.0)
        })
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w(px(0.0))
                .gap(px(theme::space::XS))
                .child(
                    div()
                        .text_size(theme::Text::Large.size())
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme::text())
                        .child(if logged_in {
                            match &root.account {
                                Some(info) if !info.nickname.is_empty() => info.nickname.clone(),
                                _ => root.tr("已登录").to_string(),
                            }
                        } else {
                            root.tr("未登录").to_string()
                        }),
                )
                .child(
                    div()
                        .text_size(theme::Text::Small.size())
                        .text_color(theme::text_muted())
                        .child(if logged_in {
                            match &root.account {
                                Some(info) => {
                                    let vip = if info.vip {
                                        root.tr("VIP")
                                    } else {
                                        root.tr("非 VIP")
                                    };
                                    root.localized(
                                        "用户 {} · {}",
                                        &[info.user_id.to_string(), vip.to_string()],
                                    )
                                }
                                None => root.tr("正在读取账号信息…").to_string(),
                            }
                        } else {
                            root.tr("登录后才能使用搜索、歌单与播放").to_string()
                        }),
                )
                .when(!logged_in, |this| {
                    this.child(
                        div()
                            .id("go-login")
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(theme::space::SM))
                            .h(px(theme::size::CONTROL))
                            .px(px(theme::space::LG))
                            .mt(px(theme::space::SM))
                            .rounded(px(theme::radius::PILL))
                            .bg(theme::accent())
                            .cursor_pointer()
                            .hover(|style| style.opacity(0.9))
                            .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                root.login_modal_open = true;
                                root.start_login(cx);
                            }))
                            .child(
                                svg()
                                    .path(icons::path("log-in"))
                                    .size(px(theme::ICON))
                                    .text_color(theme::accent_foreground()),
                            )
                            .child(
                                div()
                                    .text_size(theme::Text::Small.size())
                                    .text_color(theme::accent_foreground())
                                    .child(root.tr("去登录")),
                            ),
                    )
                }),
        )
        .when(logged_in, |this| {
            this.child(
                div()
                    .id("logout")
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(theme::space::SM))
                    .flex_none()
                    .h(px(theme::size::CONTROL))
                    .px(px(theme::space::LG))
                    .rounded(px(theme::radius::PILL))
                    .bg(theme::surface_hover())
                    .cursor_pointer()
                    .hover(|style| style.bg(theme::surface_selected()))
                    .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.logout(cx);
                    }))
                    .child(
                        svg()
                            .path(icons::path("x"))
                            .size(px(theme::ICON))
                            .text_color(theme::text_muted()),
                    )
                    .child(
                        div()
                            .text_size(theme::Text::Small.size())
                            .text_color(theme::text_muted())
                            .child(root.tr("退出登录")),
                    ),
            )
        })
        .into_any_element()
}

pub(crate) fn settings_view(root: &Root, cx: &mut Context<Root>) -> AnyElement {
    let options = [
        (
            "auto",
            root.tr("自动"),
            root.tr("按账号权益和单曲实际可用的档位择优"),
        ),
        (
            "lossless",
            root.tr("无损"),
            root.tr("封顶无损；单曲没有无损时自动降级"),
        ),
        ("highest", root.tr("极高"), root.tr("封顶极高（≈320k）")),
        ("medium", root.tr("较高"), root.tr("封顶较高")),
        ("low", root.tr("标准"), root.tr("省流")),
    ];
    let current = {
        let value = root.settings.quality.trim();
        if value.is_empty() {
            "auto"
        } else {
            value
        }
    };
    let vip = root.account.as_ref().map(|info| info.vip).unwrap_or(false);

    let theme_rows: Vec<AnyElement> = [
        (ThemeKind::Dark, root.tr("深色")),
        (ThemeKind::Light, root.tr("浅色")),
    ]
    .into_iter()
    .map(|(value, title)| {
        let chosen = root.theme == value;
        div()
            .id(gpui::ElementId::Name(
                format!("theme-set-{}", value.key()).into(),
            ))
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .gap(px(theme::space::LG))
            .px(px(theme::space::MD))
            .py(px(theme::space::SM))
            .rounded(px(theme::radius::ROW))
            .cursor_pointer()
            .when(chosen, |this| this.bg(theme::surface_selected()))
            .hover(|style| style.bg(theme::surface_hover()))
            .on_click(cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                root.set_ui_theme(value, cx);
            }))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(theme::space::SM))
                    .child(
                        div()
                            .size(px(14.0))
                            .rounded(px(theme::radius::PILL))
                            .border_1()
                            .border_color(theme::border())
                            .bg(if value == ThemeKind::Light {
                                theme::hex(0xFFFFFF)
                            } else {
                                theme::hex(0x0A0A0A)
                            }),
                    )
                    .child(
                        div()
                            .text_size(theme::Text::Body.size())
                            .text_color(if chosen {
                                theme::text()
                            } else {
                                theme::text_muted()
                            })
                            .child(title),
                    ),
            )
            .when(chosen, |this| {
                this.child(
                    svg()
                        .path(icons::path("check"))
                        .size(px(theme::ICON))
                        .text_color(theme::accent()),
                )
            })
            .into_any_element()
    })
    .collect();

    let rows: Vec<AnyElement> = options
        .iter()
        .map(|(value, title, hint)| {
            let chosen = *value == current;
            let value = value.to_string();
            div()
                .id(gpui::ElementId::Name(format!("quality-set-{value}").into()))
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .gap(px(theme::space::LG))
                .px(px(theme::space::MD))
                .py(px(theme::space::SM))
                .rounded(px(theme::radius::ROW))
                .cursor_pointer()
                .when(chosen, |this| this.bg(theme::surface_selected()))
                .hover(|style| style.bg(theme::surface_hover()))
                .on_click(cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                    root.set_quality(&value, cx);
                    cx.notify();
                }))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(theme::Text::Body.size())
                                .text_color(if chosen {
                                    theme::text()
                                } else {
                                    theme::text_muted()
                                })
                                .child(title.to_string()),
                        )
                        .child(
                            div()
                                .text_size(theme::Text::Tiny.size())
                                .text_color(theme::text_faint())
                                .child(hint.to_string()),
                        ),
                )
                .when(chosen, |this| {
                    this.child(
                        svg()
                            .path(icons::path("check"))
                            .size(px(theme::ICON))
                            .text_color(theme::accent()),
                    )
                })
                .into_any_element()
        })
        .collect();

    let download_quality_current = match root
        .settings
        .download_quality
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "" => "follow",
        "lossless" => "lossless",
        "highest" => "highest",
        "medium" => "medium",
        "low" => "low",
        _ => "follow",
    };
    let download_quality_rows: Vec<AnyElement> = [
        (
            "follow",
            root.tr("跟随播放设置"),
            root.tr("使用当前播放音质偏好；播放为自动时选择已有缓存中的最高档"),
        ),
        (
            "lossless",
            root.tr("无损"),
            root.tr("只导出无损缓存；没有对应缓存时保持待下载"),
        ),
        (
            "highest",
            root.tr("极高"),
            root.tr("只导出极高缓存（≈320k）"),
        ),
        ("medium", root.tr("较高"), root.tr("只导出较高缓存")),
        ("low", root.tr("标准"), root.tr("只导出标准缓存")),
    ]
    .iter()
    .map(|(value, title, hint)| {
        let chosen = *value == download_quality_current;
        let value = value.to_string();
        div()
            .id(gpui::ElementId::Name(
                format!("download-quality-set-{value}").into(),
            ))
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .gap(px(theme::space::LG))
            .px(px(theme::space::MD))
            .py(px(theme::space::SM))
            .rounded(px(theme::radius::ROW))
            .cursor_pointer()
            .when(chosen, |this| this.bg(theme::surface_selected()))
            .hover(|style| style.bg(theme::surface_hover()))
            .on_click(cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                root.set_download_quality(&value, cx);
            }))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(theme::Text::Body.size())
                            .text_color(if chosen {
                                theme::text()
                            } else {
                                theme::text_muted()
                            })
                            .child(title.to_string()),
                    )
                    .child(
                        div()
                            .text_size(theme::Text::Tiny.size())
                            .text_color(theme::text_faint())
                            .child(hint.to_string()),
                    ),
            )
            .when(chosen, |this| {
                this.child(
                    svg()
                        .path(icons::path("check"))
                        .size(px(theme::ICON))
                        .text_color(theme::accent()),
                )
            })
            .into_any_element()
    })
    .collect();

    let download_format_current = match root
        .settings
        .download_format
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "mp3" => "mp3",
        "flac" => "flac",
        _ => "source",
    };
    let download_format_rows: Vec<AnyElement> = [
        (
            "source",
            root.tr("原始格式"),
            root.tr("直接保存 SodaM 实际播放缓存，速度最快且不二次编码"),
        ),
        (
            "mp3",
            "MP3",
            root.tr("使用 ffmpeg 转为高质量 MP3，兼容性最好"),
        ),
        (
            "flac",
            "FLAC",
            root.tr("使用 ffmpeg 转为 FLAC；有损源不会因此变成真正无损"),
        ),
    ]
    .iter()
    .map(|(value, title, hint)| {
        let chosen = *value == download_format_current;
        let value = value.to_string();
        div()
            .id(gpui::ElementId::Name(
                format!("download-format-set-{value}").into(),
            ))
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .gap(px(theme::space::LG))
            .px(px(theme::space::MD))
            .py(px(theme::space::SM))
            .rounded(px(theme::radius::ROW))
            .cursor_pointer()
            .when(chosen, |this| this.bg(theme::surface_selected()))
            .hover(|style| style.bg(theme::surface_hover()))
            .on_click(cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                root.set_download_format(&value, cx);
            }))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(theme::Text::Body.size())
                            .text_color(if chosen {
                                theme::text()
                            } else {
                                theme::text_muted()
                            })
                            .child(title.to_string()),
                    )
                    .child(
                        div()
                            .text_size(theme::Text::Tiny.size())
                            .text_color(theme::text_faint())
                            .child(hint.to_string()),
                    ),
            )
            .when(chosen, |this| {
                this.child(
                    svg()
                        .path(icons::path("check"))
                        .size(px(theme::ICON))
                        .text_color(theme::accent()),
                )
            })
            .into_any_element()
    })
    .collect();

    let prefetch_rows: Vec<AnyElement> = [
        (
            0usize,
            root.tr("关闭"),
            root.tr("不提前缓存后续歌曲；切歌时可能需要等待加载"),
        ),
        (
            1usize,
            root.tr("前方 1 首"),
            root.tr("最省流量，只保证下一首优先缓存"),
        ),
        (
            3usize,
            root.tr("前方 3 首（推荐）"),
            root.tr("兼顾无感切歌、网络占用与缓存空间"),
        ),
        (
            5usize,
            root.tr("前方 5 首"),
            root.tr("网络稳定时切歌更从容，但会增加缓存和流量"),
        ),
    ]
    .iter()
    .map(|(value, title, hint)| {
        let chosen = root.settings.prefetch_count == *value;
        let value = *value;
        div()
            .id(gpui::ElementId::Name(
                format!("prefetch-set-{value}").into(),
            ))
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .gap(px(theme::space::LG))
            .px(px(theme::space::MD))
            .py(px(theme::space::SM))
            .rounded(px(theme::radius::ROW))
            .cursor_pointer()
            .when(chosen, |this| this.bg(theme::surface_selected()))
            .hover(|style| style.bg(theme::surface_hover()))
            .on_click(cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                root.set_prefetch_count(value, cx);
            }))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(theme::Text::Body.size())
                            .text_color(if chosen {
                                theme::text()
                            } else {
                                theme::text_muted()
                            })
                            .child(title.to_string()),
                    )
                    .child(
                        div()
                            .text_size(theme::Text::Tiny.size())
                            .text_color(theme::text_faint())
                            .child(hint.to_string()),
                    ),
            )
            .when(chosen, |this| {
                this.child(
                    svg()
                        .path(icons::path("check"))
                        .size(px(theme::ICON))
                        .text_color(theme::accent()),
                )
            })
            .into_any_element()
    })
    .collect();

    let adaptive_prefetch_rows: Vec<AnyElement> = [
        (5u32, root.tr("5 分钟")),
        (12u32, root.tr("12 分钟（推荐）")),
        (20u32, root.tr("20 分钟")),
        (30u32, root.tr("30 分钟")),
    ]
    .iter()
    .map(|(value, title)| {
        let chosen = root.settings.prefetch_adaptive && root.settings.prefetch_minutes == *value;
        let value = *value;
        div()
            .id(gpui::ElementId::Name(
                format!("prefetch-minutes-{value}").into(),
            ))
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .px(px(theme::space::MD))
            .py(px(theme::space::SM))
            .rounded(px(theme::radius::ROW))
            .cursor_pointer()
            .when(chosen, |this| this.bg(theme::surface_selected()))
            .hover(|style| style.bg(theme::surface_hover()))
            .on_click(cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                root.set_prefetch_minutes(value, cx);
            }))
            .child(title.to_string())
            .when(chosen, |this| {
                this.child(
                    svg()
                        .path(icons::path("check"))
                        .size(px(theme::ICON))
                        .text_color(theme::accent()),
                )
            })
            .into_any_element()
    })
    .collect();

    let adaptive_toggle = div()
        .id("prefetch-adaptive-toggle")
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(px(theme::space::LG))
        .px(px(theme::space::MD))
        .py(px(theme::space::SM))
        .rounded(px(theme::radius::ROW))
        .cursor_pointer()
        .when(root.settings.prefetch_adaptive, |this| {
            this.bg(theme::surface_selected())
        })
        .hover(|style| style.bg(theme::surface_hover()))
        .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
            root.set_prefetch_adaptive(!root.settings.prefetch_adaptive, cx);
        }))
        .child(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(theme::Text::Body.size())
                        .text_color(theme::text())
                        .child(root.tr("自适应预加载")),
                )
                .child(
                    div()
                        .text_size(theme::Text::Tiny.size())
                        .text_color(theme::text_faint())
                        .child(root.tr("同时满足最低曲目数和目标分钟数，最长预取 8 首")),
                ),
        )
        .child(
            div()
                .px(px(theme::space::SM))
                .py(px(2.0))
                .rounded(px(theme::radius::PILL))
                .bg(if root.settings.prefetch_adaptive {
                    theme::accent_soft()
                } else {
                    theme::surface_elevated()
                })
                .text_size(theme::Text::Tiny.size())
                .text_color(if root.settings.prefetch_adaptive {
                    theme::accent()
                } else {
                    theme::text_muted()
                })
                .child(if root.settings.prefetch_adaptive {
                    root.tr("已开启")
                } else {
                    root.tr("已关闭")
                }),
        )
        .into_any_element();

    let offline_toggle = div()
        .id("offline-mode-toggle")
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(px(theme::space::LG))
        .px(px(theme::space::MD))
        .py(px(theme::space::SM))
        .rounded(px(theme::radius::ROW))
        .cursor_pointer()
        .when(root.settings.offline_mode, |this| {
            this.bg(theme::surface_selected())
        })
        .hover(|style| style.bg(theme::surface_hover()))
        .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
            root.set_offline_mode(!root.settings.offline_mode, cx);
        }))
        .child(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(theme::Text::Body.size())
                        .text_color(theme::text())
                        .child(root.tr("离线模式 / 缓存优先")),
                )
                .child(
                    div()
                        .text_size(theme::Text::Tiny.size())
                        .text_color(theme::text_faint())
                        .child(
                            root.tr("开启后只播放当前音质档位已经缓存的歌曲，不发起音频网络请求"),
                        ),
                ),
        )
        .child(
            div()
                .px(px(theme::space::SM))
                .py(px(2.0))
                .rounded(px(theme::radius::PILL))
                .bg(if root.settings.offline_mode {
                    theme::accent_soft()
                } else {
                    theme::surface_elevated()
                })
                .text_size(theme::Text::Tiny.size())
                .text_color(if root.settings.offline_mode {
                    theme::accent()
                } else {
                    theme::text_muted()
                })
                .child(if root.settings.offline_mode {
                    root.tr("离线")
                } else {
                    root.tr("在线")
                }),
        )
        .into_any_element();

    let cache_limit_rows: Vec<AnyElement> = [
        (1u64, "1 GB", root.tr("适合磁盘空间较小的设备")),
        (
            3u64,
            root.tr("3 GB（推荐）"),
            root.tr("兼顾无感切歌与磁盘占用"),
        ),
        (5u64, "5 GB", root.tr("适合经常连续听歌，保留更多本地缓存")),
        (0u64, root.tr("不限制"), root.tr("不自动清理播放缓存")),
    ]
    .iter()
    .map(|(value, title, hint)| {
        let chosen = root.settings.cache_limit_gb == *value;
        let value = *value;
        div()
            .id(gpui::ElementId::Name(format!("cache-limit-{value}").into()))
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .gap(px(theme::space::LG))
            .px(px(theme::space::MD))
            .py(px(theme::space::SM))
            .rounded(px(theme::radius::ROW))
            .cursor_pointer()
            .when(chosen, |this| this.bg(theme::surface_selected()))
            .hover(|style| style.bg(theme::surface_hover()))
            .on_click(cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                root.set_cache_limit_gb(value, cx);
            }))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(theme::Text::Body.size())
                            .text_color(if chosen {
                                theme::text()
                            } else {
                                theme::text_muted()
                            })
                            .child(title.to_string()),
                    )
                    .child(
                        div()
                            .text_size(theme::Text::Tiny.size())
                            .text_color(theme::text_faint())
                            .child(hint.to_string()),
                    ),
            )
            .when(chosen, |this| {
                this.child(
                    svg()
                        .path(icons::path("check"))
                        .size(px(theme::ICON))
                        .text_color(theme::accent()),
                )
            })
            .into_any_element()
    })
    .collect();

    let lyric_font_rows: Vec<AnyElement> = [
        (16u32, root.tr("小")),
        (18u32, root.tr("标准")),
        (22u32, root.tr("大")),
        (26u32, root.tr("特大")),
    ]
    .iter()
    .map(|(value, title)| {
        let chosen = root.settings.lyrics_font_size == *value;
        let value = *value;
        div()
            .id(gpui::ElementId::Name(format!("lyrics-font-{value}").into()))
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .px(px(theme::space::MD))
            .py(px(theme::space::SM))
            .rounded(px(theme::radius::ROW))
            .cursor_pointer()
            .when(chosen, |this| this.bg(theme::surface_selected()))
            .hover(|style| style.bg(theme::surface_hover()))
            .on_click(cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                root.set_lyrics_font_size(value, cx);
            }))
            .child(title.to_string())
            .when(chosen, |this| {
                this.child(
                    svg()
                        .path(icons::path("check"))
                        .size(px(theme::ICON))
                        .text_color(theme::accent()),
                )
            })
            .into_any_element()
    })
    .collect();

    let lyric_spacing_rows: Vec<AnyElement> = [
        (28u32, root.tr("紧凑")),
        (32u32, root.tr("标准")),
        (40u32, root.tr("宽松")),
        (48u32, root.tr("很宽")),
    ]
    .iter()
    .map(|(value, title)| {
        let chosen = root.settings.lyrics_line_height == *value;
        let value = *value;
        div()
            .id(gpui::ElementId::Name(
                format!("lyrics-spacing-{value}").into(),
            ))
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .px(px(theme::space::MD))
            .py(px(theme::space::SM))
            .rounded(px(theme::radius::ROW))
            .cursor_pointer()
            .when(chosen, |this| this.bg(theme::surface_selected()))
            .hover(|style| style.bg(theme::surface_hover()))
            .on_click(cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                root.set_lyrics_line_height(value, cx);
            }))
            .child(title.to_string())
            .when(chosen, |this| {
                this.child(
                    svg()
                        .path(icons::path("check"))
                        .size(px(theme::ICON))
                        .text_color(theme::accent()),
                )
            })
            .into_any_element()
    })
    .collect();

    let lyric_offset_rows: Vec<AnyElement> = [-500i64, -250, 0, 250, 500]
        .into_iter()
        .map(|value| {
            let chosen = root.settings.lyrics_offset_ms == value;
            let title = if value > 0 {
                format!("+{value} ms")
            } else {
                format!("{value} ms")
            };
            div()
                .id(gpui::ElementId::Name(
                    format!("lyrics-offset-{value}").into(),
                ))
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .px(px(theme::space::MD))
                .py(px(theme::space::SM))
                .rounded(px(theme::radius::ROW))
                .cursor_pointer()
                .when(chosen, |this| this.bg(theme::surface_selected()))
                .hover(|style| style.bg(theme::surface_hover()))
                .on_click(cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                    root.set_lyrics_offset_ms(value, cx);
                }))
                .child(title)
                .when(chosen, |this| {
                    this.child(
                        svg()
                            .path(icons::path("check"))
                            .size(px(theme::ICON))
                            .text_color(theme::accent()),
                    )
                })
                .into_any_element()
        })
        .collect();

    let account_card = account_section(root, cx);

    let language_rows: Vec<AnyElement> = [
        (Language::System, root.tr("跟随系统")),
        (Language::Chinese, "中文"),
        (Language::English, "English"),
    ]
    .into_iter()
    .map(|(value, title)| {
        let chosen = if root.language_follows_system {
            matches!(value, Language::System)
        } else {
            !matches!(value, Language::System) && root.language == value
        };
        div()
            .id(gpui::ElementId::Name(
                format!("language-set-{:?}", value).into(),
            ))
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .gap(px(theme::space::LG))
            .px(px(theme::space::MD))
            .py(px(theme::space::SM))
            .rounded(px(theme::radius::ROW))
            .cursor_pointer()
            .when(chosen, |this| this.bg(theme::surface_selected()))
            .hover(|style| style.bg(theme::surface_hover()))
            .on_click(cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                match value {
                    Language::System => {
                        root.language = Language::system_locale();
                        root.language_follows_system = true;
                        root.settings.language = "auto".to_string();
                        let _ = root.settings.save();
                        root.status = if root.language.is_zh() {
                            root.tr("语言已恢复为跟随系统").to_string()
                        } else {
                            "Language restored to system".to_string()
                        };
                    }
                    language => root.set_language(language, cx),
                }
                cx.notify();
            }))
            .child(
                div()
                    .text_size(theme::Text::Body.size())
                    .text_color(if chosen {
                        theme::text()
                    } else {
                        theme::text_muted()
                    })
                    .child(title),
            )
            .when(chosen, |this| {
                this.child(
                    svg()
                        .path(icons::path("check"))
                        .size(px(theme::ICON))
                        .text_color(theme::accent()),
                )
            })
            .into_any_element()
    })
    .collect();

    let section_body = match root.settings_section {
        SettingsSection::General => {
            let theme_hint = if root.theme_follows_system {
                root.tr("当前跟随系统偏好，手动选择后会固定主题")
                    .to_string()
            } else {
                root.tr("已固定主题；可随时切换深色或浅色").to_string()
            };
            let language_hint = if root.language_follows_system {
                root.tr("当前跟随系统语言；手动选择后会固定语言")
                    .to_string()
            } else {
                root.tr("已固定语言；可随时切换中文或 English").to_string()
            };
            div()
                .flex()
                .flex_col()
                .gap(px(theme::space::XL))
                .child(settings_heading(root.tr("主题"), &theme_hint))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(theme::space::XS))
                        .children(theme_rows),
                )
                .child(settings_heading(root.tr("语言"), &language_hint))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(theme::space::XS))
                        .children(language_rows),
                )
                .into_any_element()
        }
        SettingsSection::Playback => {
            let quality_hint = if vip {
                root.tr("当前账号是 VIP：登录时默认无损，单曲没有无损会自动降级")
                    .to_string()
            } else {
                root.tr("当前账号非 VIP：默认自动，取免费档里实际可用的最高一档")
                    .to_string()
            };
            div()
                .flex()
                .flex_col()
                .gap(px(theme::space::XL))
                .child(settings_heading(root.tr("播放音质偏好"), &quality_hint))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(theme::space::XS))
                        .children(rows),
                )
                .child(settings_heading(
                    root.tr("智能预加载"),
                    root.tr(
                        "播放开始后后台逐步缓存后续歌曲；自适应模式按未来播放时长动态决定缓存深度",
                    ),
                ))
                .child(adaptive_toggle)
                .when(root.settings.prefetch_adaptive, |this| {
                    this.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(theme::space::XS))
                            .children(adaptive_prefetch_rows),
                    )
                })
                .when(!root.settings.prefetch_adaptive, |this| {
                    this.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(theme::space::XS))
                            .children(prefetch_rows),
                    )
                })
                .child(settings_heading(
                    root.tr("离线播放"),
                    root.tr("优先使用本地缓存；离线模式开启后完全禁止新的音频网络请求"),
                ))
                .child(offline_toggle)
                .into_any_element()
        }
        SettingsSection::Lyrics => div()
            .flex()
            .flex_col()
            .gap(px(theme::space::XL))
            .child(settings_heading(
                root.tr("歌词字号"),
                root.tr("调整播放页歌词文字大小"),
            ))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(theme::space::XS))
                    .children(lyric_font_rows),
            )
            .child(settings_heading(
                root.tr("歌词行距"),
                root.tr("调整每行歌词之间的垂直间距"),
            ))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(theme::space::XS))
                    .children(lyric_spacing_rows),
            )
            .child(settings_heading(
                root.tr("歌词时间偏移"),
                root.tr("负值让歌词更早出现，正值让歌词更晚出现"),
            ))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(theme::space::XS))
                    .children(lyric_offset_rows),
            )
            .into_any_element(),
        SettingsSection::Downloads => {
            let path = sodam_core::downloads::download_dir_for(&root.settings.download_dir)
                .display()
                .to_string();
            div()
                .flex()
                .flex_col()
                .gap(px(theme::space::XL))
                .child(settings_heading(
                    root.tr("下载设置"),
                    &root.localized("当前保存到：{}", &[path]),
                ))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(theme::space::SM))
                        .child(
                            div()
                                .id("choose-download-dir")
                                .h(px(theme::size::CONTROL_SM))
                                .px(px(theme::space::MD))
                                .flex()
                                .items_center()
                                .rounded(px(theme::radius::ROW))
                                .bg(theme::surface_elevated())
                                .cursor_pointer()
                                .hover(|style| style.bg(theme::surface_hover()))
                                .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                    root.choose_download_directory(cx);
                                }))
                                .child(root.tr("选择目录")),
                        )
                        .child(
                            div()
                                .id("open-download-dir-settings")
                                .h(px(theme::size::CONTROL_SM))
                                .px(px(theme::space::MD))
                                .flex()
                                .items_center()
                                .rounded(px(theme::radius::ROW))
                                .cursor_pointer()
                                .hover(|style| style.bg(theme::surface_hover()))
                                .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                    root.open_download_folder(cx);
                                }))
                                .child(root.tr("打开目录")),
                        )
                        .when(!root.settings.download_dir.trim().is_empty(), |this| {
                            this.child(
                                div()
                                    .id("reset-download-dir")
                                    .h(px(theme::size::CONTROL_SM))
                                    .px(px(theme::space::MD))
                                    .flex()
                                    .items_center()
                                    .rounded(px(theme::radius::ROW))
                                    .cursor_pointer()
                                    .hover(|style| style.bg(theme::surface_hover()))
                                    .on_click(cx.listener(
                                        |root, _event: &ClickEvent, _window, cx| {
                                            root.reset_download_directory(cx);
                                        },
                                    ))
                                    .child(root.tr("恢复默认")),
                            )
                        }),
                )
                .child(
                    div()
                        .px(px(theme::space::MD))
                        .text_size(theme::Text::Tiny.size())
                        .text_color(theme::text_faint())
                        .child(root.tr("导出时写入歌曲名、歌手、专辑；封面缓存可用时一并嵌入")),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(theme::space::XS))
                        .child(
                            div()
                                .px(px(theme::space::MD))
                                .text_size(theme::Text::Small.size())
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(theme::text_muted())
                                .child(root.tr("下载音质")),
                        )
                        .children(download_quality_rows),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(theme::space::XS))
                        .child(
                            div()
                                .px(px(theme::space::MD))
                                .text_size(theme::Text::Small.size())
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(theme::text_muted())
                                .child(root.tr("下载格式")),
                        )
                        .children(download_format_rows),
                )
                .into_any_element()
        }
        SettingsSection::Storage => {
            let (audio_bytes, audio_files, cover_bytes, cover_files) = root.cache_summary;
            let total = audio_bytes + cover_bytes;
            let usage = if root.language.resolved().is_zh() {
                root.localized(
                    "共 {}（歌曲 {} 首 {} / 封面 {} 张 {}）",
                    &[
                        human_bytes(total),
                        audio_files.to_string(),
                        human_bytes(audio_bytes),
                        cover_files.to_string(),
                        human_bytes(cover_bytes),
                    ],
                )
            } else {
                format!(
                    "Total {} ({} songs {}, {} covers {})",
                    human_bytes(total),
                    audio_files,
                    human_bytes(audio_bytes),
                    cover_files,
                    human_bytes(cover_bytes),
                )
            };
            div()
                .flex()
                .flex_col()
                .gap(px(theme::space::XL))
                .child(settings_heading(
                    root.tr("缓存管理"),
                    root.tr("达到上限后按最近使用顺序自动清理播放缓存；不会删除下载管理里的歌曲"),
                ))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(theme::space::XS))
                        .children(cache_limit_rows),
                )
                .child(settings_heading(root.tr("缓存使用情况"), &usage))
                .child(
                    div()
                        .id("clear-cache")
                        .flex()
                        .items_center()
                        .justify_center()
                        .h(px(theme::size::CONTROL))
                        .w(px(160.0))
                        .rounded(px(theme::radius::ROW))
                        .bg(theme::surface_elevated())
                        .cursor_pointer()
                        .hover(|style| style.bg(theme::surface_hover()))
                        .text_size(theme::Text::Small.size())
                        .text_color(theme::text())
                        .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                            let removed = sodam_core::audio::clear_cache();
                            let message =
                                root.localized("已清理缓存：{} 个文件", &[removed.to_string()]);
                            root.refresh_cache_stats(cx);
                            root.refresh_audio_cache_index(cx);
                            root.toast(message, cx);
                        }))
                        .child(root.tr("清除歌曲缓存")),
                )
                .into_any_element()
        }
        SettingsSection::Desktop => {
            let opacity_rows = [60u8, 75, 86, 100]
                .into_iter()
                .map(|value| {
                    choice_setting_row(
                        format!("desktop-opacity-{value}"),
                        format!("{value}%"),
                        root.tr("桌面歌词背景透明度").to_string(),
                        root.settings.desktop_lyrics_opacity == value,
                        cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                            root.e3_set_desktop_opacity(value, cx);
                        }),
                    )
                })
                .collect::<Vec<_>>();
            let align_rows = [
                ("left", root.tr("左对齐")),
                ("center", root.tr("居中")),
                ("right", root.tr("右对齐")),
            ]
            .into_iter()
            .map(|(value, label)| {
                choice_setting_row(
                    format!("desktop-align-{value}"),
                    label.to_string(),
                    String::new(),
                    root.settings.desktop_lyrics_align == value,
                    cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                        root.e3_set_desktop_align(value, cx);
                    }),
                )
            })
            .collect::<Vec<_>>();
            div()
                .flex()
                .flex_col()
                .gap(px(theme::space::XL))
                .child(settings_heading(
                    root.tr("桌面歌词"),
                    root.tr("调整独立歌词窗口的显示方式、透明度和交互"),
                ))
                .child(toggle_setting_row(
                    "desktop-single-line",
                    root.tr("单行歌词"),
                    root.tr("关闭时显示当前行和下一行"),
                    root.settings.desktop_lyrics_single_line,
                    cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.e3_set_desktop_single_line(
                            !root.settings.desktop_lyrics_single_line,
                            cx,
                        );
                    }),
                ))
                .child(toggle_setting_row(
                    "desktop-always-top",
                    root.tr("始终置顶"),
                    root.tr("桌面歌词窗口保持在其他窗口上方"),
                    root.settings.desktop_lyrics_always_on_top,
                    cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.e3_set_desktop_always_on_top(
                            !root.settings.desktop_lyrics_always_on_top,
                            cx,
                        );
                    }),
                ))
                .child(toggle_setting_row(
                    "desktop-lock",
                    root.tr("锁定位置"),
                    root.tr("锁定后禁止拖动和调整窗口尺寸"),
                    root.settings.desktop_lyrics_locked,
                    cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.e3_set_desktop_locked(!root.settings.desktop_lyrics_locked, cx);
                    }),
                ))
                .child(toggle_setting_row(
                    "desktop-click-through",
                    root.tr("鼠标穿透"),
                    root.tr("受当前 Linux/窗口后端能力限制；不支持时保持普通窗口交互"),
                    root.settings.desktop_lyrics_click_through,
                    cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.e3_set_desktop_click_through(
                            !root.settings.desktop_lyrics_click_through,
                            cx,
                        );
                    }),
                ))
                .child(settings_heading(
                    root.tr("背景透明度"),
                    root.tr("降低背景存在感，让歌词更适合悬浮在桌面"),
                ))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(theme::space::XS))
                        .children(opacity_rows),
                )
                .child(settings_heading(
                    root.tr("歌词对齐"),
                    root.tr("设置桌面歌词文字的水平对齐方式"),
                ))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(theme::space::XS))
                        .children(align_rows),
                )
                .into_any_element()
        }
        SettingsSection::Advanced => {
            let devices = crate::system_audio::list_output_devices();
            let device_rows = devices
                .into_iter()
                .map(|device| {
                    let id = device.id.clone();
                    let selected = if root.settings.audio_output_device.trim().is_empty() {
                        device.default
                    } else {
                        root.settings.audio_output_device == device.id
                    };
                    choice_setting_row(
                        format!("audio-device-{}", device.id),
                        device.name,
                        if device.default {
                            root.tr("系统默认输出").to_string()
                        } else {
                            String::new()
                        },
                        selected,
                        cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                            root.e3_set_audio_device(id.clone(), cx);
                        }),
                    )
                })
                .collect::<Vec<_>>();
            let crossfade_rows = [0u32, 2, 3, 5, 8]
                .into_iter()
                .map(|value| {
                    let label = if value == 0 {
                        root.tr("关闭").to_string()
                    } else {
                        root.localized("{} 秒", &[value.to_string()])
                    };
                    choice_setting_row(
                        format!("crossfade-{value}"),
                        label,
                        String::new(),
                        root.settings.crossfade_seconds == value,
                        cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                            root.e3_set_crossfade(value, cx);
                        }),
                    )
                })
                .collect::<Vec<_>>();
            let compact_rows = [760u32, 900, 1080]
                .into_iter()
                .map(|value| {
                    choice_setting_row(
                        format!("compact-width-{value}"),
                        format!("{value} px"),
                        root.tr("低于此宽度时折叠次要播放栏按钮").to_string(),
                        root.settings.player_bar_compact_width == value,
                        cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                            root.e3_set_compact_width(value, cx);
                        }),
                    )
                })
                .collect::<Vec<_>>();
            div()
                .flex()
                .flex_col()
                .gap(px(theme::space::XL))
                .child(settings_heading(
                    root.tr("音频输出设备"),
                    root.tr("Ubuntu / PipeWire-Pulse 下可直接切换扬声器、耳机、HDMI 或蓝牙输出"),
                ))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(theme::space::XS))
                        .children(device_rows),
                )
                .child(settings_heading(
                    root.tr("播放增强"),
                    root.tr("响度标准化和歌曲衔接均可独立关闭"),
                ))
                .child(toggle_setting_row(
                    "system-notifications",
                    root.tr("系统切歌通知"),
                    root.tr("切歌时显示封面、歌曲名和歌手"),
                    root.settings.system_notifications,
                    cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.e3_set_notifications(!root.settings.system_notifications, cx);
                    }),
                ))
                .child(toggle_setting_row(
                    "normalize-volume",
                    root.tr("响度标准化"),
                    root.tr("使用本地 ffmpeg 分析并缓存增益，减少歌曲之间忽大忽小"),
                    root.settings.normalize_volume,
                    cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.e3_set_normalize_volume(!root.settings.normalize_volume, cx);
                    }),
                ))
                .child(toggle_setting_row(
                    "gapless-playback",
                    root.tr("无缝播放 Gapless"),
                    root.tr("提前衔接下一首，尽量消除曲目边界的短暂空白"),
                    root.settings.gapless_playback,
                    cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.e3_set_gapless(!root.settings.gapless_playback, cx);
                    }),
                ))
                .child(settings_heading(
                    root.tr("交叉淡化 Crossfade"),
                    root.tr("让上一首淡出、下一首淡入；0 秒表示关闭"),
                ))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(theme::space::XS))
                        .children(crossfade_rows),
                )
                .child(settings_heading(
                    root.tr("性能与布局"),
                    root.tr("控制启动时的后台工作和底部播放栏的响应式折叠"),
                ))
                .child(toggle_setting_row(
                    "lazy-startup",
                    root.tr("启动延迟加载"),
                    root.tr("优先打开界面和恢复播放，再延迟加载收藏、下载索引和缓存统计"),
                    root.settings.lazy_startup,
                    cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.e3_set_lazy_startup(!root.settings.lazy_startup, cx);
                    }),
                ))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(theme::space::XS))
                        .children(compact_rows),
                )
                .into_any_element()
        }
        SettingsSection::Account => {
            let repository = env!("CARGO_PKG_REPOSITORY");
            let signer = if root.settings.signer_url.trim().is_empty() {
                root.tr("（未配置）").to_string()
            } else {
                root.settings.signer_url.trim().to_string()
            };
            div()
                .flex()
                .flex_col()
                .gap(px(theme::space::XL))
                .child(account_card)
                .child(settings_heading(
                    root.tr("其他"),
                    &root.localized("签名服务：{}", &[signer]),
                ))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(theme::space::SM))
                        .p(px(theme::space::MD))
                        .rounded(px(theme::radius::ROW))
                        .bg(theme::surface_elevated())
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.0))
                                .truncate()
                                .text_size(theme::Text::Small.size())
                                .text_color(theme::text_muted())
                                .child(root.localized(
                                    "配置：{}",
                                    &[sodam_core::Settings::config_path().display().to_string()],
                                )),
                        )
                        .child(
                            div()
                                .id("edit-config")
                                .flex()
                                .items_center()
                                .justify_center()
                                .h(px(theme::size::CONTROL_SM))
                                .px(px(theme::space::MD))
                                .rounded(px(theme::radius::ROW))
                                .cursor_pointer()
                                .hover(|style| style.bg(theme::surface_hover()))
                                .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                    root.open_config_file(cx);
                                }))
                                .child(root.tr("Edit")),
                        ),
                )
                .child(
                    div()
                        .id("about-footer")
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(theme::space::MD))
                        .p(px(theme::space::MD))
                        .rounded(px(theme::radius::CARD))
                        .bg(theme::surface_elevated())
                        .border_1()
                        .border_color(theme::border())
                        .cursor_pointer()
                        .hover(|style| style.bg(theme::surface_hover()))
                        .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                            root.open_github_repository(cx);
                        }))
                        .child(
                            img("icons/sodam-logo.svg")
                                .size(px(40.0))
                                .flex_none()
                                .rounded(px(theme::radius::ROW))
                                .object_fit(ObjectFit::Cover),
                        )
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .min_w(px(0.0))
                                .gap(px(theme::space::XS))
                                .child(
                                    div()
                                        .text_size(theme::Text::Small.size())
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .text_color(theme::text())
                                        .child(format!("SodaM v{}", env!("CARGO_PKG_VERSION"))),
                                )
                                .child(
                                    div()
                                        .truncate()
                                        .text_size(theme::Text::Tiny.size())
                                        .text_color(theme::text_muted())
                                        .child(format!("GitHub: {repository}")),
                                )
                                .child(
                                    div()
                                        .text_size(theme::Text::Tiny.size())
                                        .text_color(theme::text_faint())
                                        .child("Author: ZephyrCheung"),
                                ),
                        )
                        .child(
                            svg()
                                .path(icons::path("chevron-right"))
                                .size(px(theme::ICON_SM))
                                .text_color(theme::text_faint()),
                        ),
                )
                .into_any_element()
        }
    };

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
        .child(settings_nav_item(
            root,
            SettingsSection::General,
            root.tr("常规"),
            "palette",
            cx,
        ))
        .child(settings_nav_item(
            root,
            SettingsSection::Playback,
            root.tr("播放"),
            "music",
            cx,
        ))
        .child(settings_nav_item(
            root,
            SettingsSection::Lyrics,
            root.tr("歌词"),
            "captions",
            cx,
        ))
        .child(settings_nav_item(
            root,
            SettingsSection::Downloads,
            root.tr("下载"),
            "download",
            cx,
        ))
        .child(settings_nav_item(
            root,
            SettingsSection::Storage,
            root.tr("存储"),
            "hard-drive",
            cx,
        ))
        .child(settings_nav_item(
            root,
            SettingsSection::Desktop,
            root.tr("桌面"),
            "captions",
            cx,
        ))
        .child(settings_nav_item(
            root,
            SettingsSection::Advanced,
            root.tr("高级"),
            "settings",
            cx,
        ))
        .child(settings_nav_item(
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
                .child(section_body),
        )
        .into_any_element()
}
