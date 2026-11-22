//! 左侧导航栏与响应式紧凑导航：品牌区 + 「我的音乐」分组 + 导航行 + 主题/模式快捷切换。
//!
//! 支持三种响应式形态：
//! 1. **完整侧边栏 (`render`)**：宽屏普通模式下展示图标 + 文字 + 中式主题徽章；
//! 2. **紧凑图标栏 (`render_compact`)**：半屏播放模式或中等窄窗（`< 980px`）自动收窄为 `64px` 图标轨；
//! 3. **顶部横向导航条 (`render_top_nav`)**：竖向窄窗口（`< 680px`）下转为顶部紧凑胶囊导航，不挤压内容宽度。

use gpui::prelude::*;
use gpui::{div, img, px, svg, AnyElement, ClickEvent, Context, IntoElement};
use sodam_core::MusicProvider;

use crate::app::{Nav, Root, SettingsSection};
use crate::ui::{icons, theme};

/// 导航项的图标名（Lucide 线性图标）。
fn icon_name(nav: Nav) -> &'static str {
    match nav {
        Nav::Home => "house",
        Nav::Scenes => "radio",
        Nav::Search => "search",
        Nav::Liked => "heart",
        Nav::Library => "list-music",
        Nav::LocalMusic => "disc-3",
        Nav::LocalPlaylists => "folder-open",
        Nav::Recent => "history",
        Nav::Downloads => "download",
        Nav::Artist | Nav::Album => "search",
        Nav::Settings => "settings",
        Nav::Lyrics => "disc-3",
    }
}

fn handle_nav_click(root: &mut Root, nav: Nav, cx: &mut Context<Root>) {
    let provider = root.active_provider();
    let can_browse = root.settings.can_browse_active_provider();
    let blocked_unauth_browse = !can_browse
        && !matches!(
            nav,
            Nav::Settings
                | Nav::Downloads
                | Nav::Recent
                | Nav::LocalMusic
                | Nav::LocalPlaylists
                | Nav::Lyrics
        );
    let blocked_personal_library =
        matches!(nav, Nav::Liked | Nav::Library) && !root.settings.is_provider_logged_in(provider);
    if blocked_unauth_browse || blocked_personal_library {
        root.settings_section = SettingsSection::Account;
        root.set_nav(Nav::Settings, cx);
        if provider == MusicProvider::Migu {
            root.set_status("请先在「设置 → 账号与会员」登录咪咕音乐", &[]);
        } else {
            root.set_status("请先在「设置 → 账号与会员」扫码登录", &[]);
        }
        return;
    }
    if nav == Nav::Lyrics {
        root.open_playback(cx);
    } else {
        root.set_nav(nav, cx);
    }
}

fn provider_badge(provider: MusicProvider, size_px: f32) -> AnyElement {
    match provider {
        MusicProvider::Soda => div()
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .size(px(size_px))
            .rounded(px(theme::radius::ROW))
            .overflow_hidden()
            .child(img("icons/sodam-logo.svg").size(px(size_px)))
            .into_any_element(),
        MusicProvider::Migu => div()
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .size(px(size_px))
            .rounded(px(theme::radius::ROW))
            .bg(theme::accent())
            .child(
                svg()
                    .path(icons::path("music"))
                    .size(px((size_px * 0.56).round()))
                    .text_color(theme::accent_foreground()),
            )
            .into_any_element(),
    }
}

fn nav_row(nav: Nav, label: String, selected: bool, cx: &mut Context<Root>) -> AnyElement {
    let foreground = if selected {
        theme::accent()
    } else {
        theme::text_muted()
    };
    let icon_color = if selected {
        theme::accent()
    } else {
        theme::text_muted()
    };
    div()
        .id(("nav", nav as usize))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(theme::space::MD))
        .h(theme::nav_row())
        .px(px(theme::space::MD))
        .rounded(px(theme::radius::ROW))
        .cursor_pointer()
        .when(selected, |this| {
            this.font_weight(gpui::FontWeight::SEMIBOLD)
        })
        .when(!selected, |this| {
            this.hover(|style| style.bg(theme::surface_hover()).layer_scale(1.012))
        })
        .text_color(foreground)
        .text_size(theme::Text::Body.size())
        .on_click(cx.listener(move |root, _event: &ClickEvent, _window, cx| {
            handle_nav_click(root, nav, cx);
        }))
        .child(
            svg()
                .path(icons::path(icon_name(nav)))
                .size(px(theme::ICON))
                .flex_none()
                .text_color(icon_color),
        )
        .child(div().min_w_0().truncate().child(label))
        .into_any_element()
}

fn nav_icon_cell(nav: Nav, selected: bool, cx: &mut Context<Root>) -> AnyElement {
    let icon_color = if selected {
        theme::accent()
    } else {
        theme::text_muted()
    };
    div()
        .id(("nav-compact", nav as usize))
        .flex()
        .items_center()
        .justify_center()
        .size(px(40.0))
        .rounded(px(theme::radius::ROW))
        .cursor_pointer()
        .when(!selected, |this| {
            this.hover(|style| style.bg(theme::surface_hover()).layer_scale(1.04))
        })
        .on_click(cx.listener(move |root, _event: &ClickEvent, _window, cx| {
            handle_nav_click(root, nav, cx);
        }))
        .child(
            svg()
                .path(icons::path(icon_name(nav)))
                .size(px(theme::ICON))
                .flex_none()
                .text_color(icon_color),
        )
        .into_any_element()
}

fn section_label(text: &str) -> impl IntoElement {
    div()
        .px(px(theme::space::MD))
        .pt(px(theme::space::LG))
        .pb(px(theme::space::SM))
        .text_size(theme::Text::Tiny.size())
        .text_color(theme::text_faint())
        .child(text.to_string())
}

fn is_nav_active(root: &Root, nav: Nav) -> bool {
    root.nav == nav
}

/// 渲染完整侧边栏（宽屏普通窗口模式）。
pub fn render(root: &Root, cx: &mut Context<Root>) -> impl IntoElement {
    let main_items: Vec<AnyElement> = [Nav::Home, Nav::Scenes, Nav::Search, Nav::Lyrics]
        .into_iter()
        .map(|nav| {
            nav_row(
                nav,
                root.nav_display_label(nav),
                is_nav_active(root, nav),
                cx,
            )
        })
        .collect();
    let library_items: Vec<AnyElement> = [
        Nav::Liked,
        Nav::Library,
        Nav::LocalMusic,
        Nav::LocalPlaylists,
        Nav::Recent,
        Nav::Downloads,
    ]
    .into_iter()
    .map(|nav| {
        nav_row(
            nav,
            root.nav_display_label(nav),
            is_nav_active(root, nav),
            cx,
        )
    })
    .collect();

    div()
        .flex()
        .flex_col()
        .w(px(theme::SIDEBAR_W))
        .h_full()
        .flex_none()
        .p(px(theme::space::MD))
        .gap(px(theme::space::XS))
        .child(
            // 品牌与数据源切换区：点击切换 汽水音乐 / 咪咕音乐（透明融于一体背景）
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .gap(px(4.0))
                .px(px(2.0))
                .py(px(theme::space::XS))
                .child(
                    div()
                        .id("provider-switcher-btn")
                        .flex()
                        .flex_row()
                        .items_center()
                        .flex_1()
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .gap(px(6.0))
                        .px(px(6.0))
                        .py(px(6.0))
                        .rounded(px(theme::radius::ROW))
                        .cursor_pointer()
                        .when(root.provider_menu_open, |this| {
                            this.bg(theme::surface_hover())
                        })
                        .hover(|style| style.bg(theme::surface_hover()))
                        .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                            root.provider_menu_open = !root.provider_menu_open;
                            cx.notify();
                        }))
                        .child(provider_badge(root.active_provider(), 24.0))
                        .child({
                            let title_px = (theme::ui_font_size() as f32 * 1.16)
                                .round()
                                .clamp(15.0, 18.0);
                            div()
                                .whitespace_nowrap()
                                .text_size(px(title_px))
                                .font_weight(gpui::FontWeight::BOLD)
                                .text_color(theme::text())
                                .child(root.provider_display_name(root.active_provider()))
                        })
                        .child(
                            div()
                                .flex_none()
                                .text_size(theme::Text::Tiny.size())
                                .text_color(theme::text_muted())
                                .child("▾"),
                        ),
                )
                .child(
                    div()
                        .id("sidebar-collapse-btn")
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(26.0))
                        .rounded(px(theme::radius::ROW))
                        .cursor_pointer()
                        .hover(|style| style.bg(theme::surface_hover()))
                        .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                            root.provider_menu_open = false;
                            root.toggle_sidebar_collapsed(cx);
                        }))
                        .child(
                            svg()
                                .path(icons::path("chevron-left"))
                                .size(px(theme::ICON_SM))
                                .text_color(theme::text_muted()),
                        ),
                ),
        )
        .children(main_items)
        .child(section_label(root.tr("我的音乐")))
        .children(library_items)
        .child(div().flex_1())
        .child(nav_row(
            Nav::Settings,
            root.nav_display_label(Nav::Settings),
            root.nav == Nav::Settings,
            cx,
        ))
}

/// 渲染紧凑图标侧边栏（半屏模式或 `< 980px` 窗口自动收窄）。
pub fn render_compact(root: &Root, cx: &mut Context<Root>) -> impl IntoElement {
    let navs = [
        Nav::Home,
        Nav::Scenes,
        Nav::Search,
        Nav::Lyrics,
        Nav::Liked,
        Nav::Library,
        Nav::LocalMusic,
        Nav::Recent,
        Nav::Downloads,
    ];
    let items: Vec<AnyElement> = navs
        .into_iter()
        .map(|nav| nav_icon_cell(nav, is_nav_active(root, nav), cx))
        .collect();

    div()
        .flex()
        .flex_col()
        .items_center()
        .w(px(theme::SIDEBAR_COMPACT_W))
        .h_full()
        .flex_none()
        .py(px(theme::space::MD))
        .gap(px(theme::space::XS))
        .child(
            div()
                .id("sidebar-compact-logo")
                .flex()
                .items_center()
                .justify_center()
                .size(px(36.0))
                .rounded(px(theme::radius::ROW))
                .cursor_pointer()
                .hover(|style| style.bg(theme::surface_hover()))
                .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                    root.provider_menu_open = !root.provider_menu_open;
                    cx.notify();
                }))
                .child(provider_badge(root.active_provider(), 26.0)),
        )
        .child(
            div()
                .id("sidebar-expand-btn")
                .flex()
                .items_center()
                .justify_center()
                .w(px(36.0))
                .h(px(26.0))
                .mb(px(theme::space::XS))
                .rounded(px(theme::radius::ROW))
                .cursor_pointer()
                .hover(|style| style.bg(theme::surface_hover()))
                .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                    if root.window_mode == theme::WindowMode::HalfScreen {
                        root.set_window_mode(theme::WindowMode::Normal, cx);
                    }
                    root.set_sidebar_collapsed(false, cx);
                }))
                .child(
                    svg()
                        .path(icons::path("chevron-right"))
                        .size(px(theme::ICON_SM))
                        .text_color(theme::text_muted()),
                ),
        )
        .children(items)
        .child(div().flex_1())
        .child(nav_icon_cell(Nav::Settings, root.nav == Nav::Settings, cx))
}

/// 渲染竖向窄窗口（`< 680px`）下的顶部紧凑导航栏。
pub fn render_top_nav(root: &Root, cx: &mut Context<Root>) -> impl IntoElement {
    let navs = [
        Nav::Home,
        Nav::Scenes,
        Nav::Search,
        Nav::Liked,
        Nav::Library,
        Nav::Lyrics,
        Nav::Settings,
    ];
    let items: Vec<AnyElement> = navs
        .into_iter()
        .map(|nav| {
            let active = is_nav_active(root, nav);
            let label = root.nav_display_label(nav);
            div()
                .id(("top-nav", nav as usize))
                .flex()
                .flex_row()
                .items_center()
                .gap(px(theme::space::XS))
                .h(px(30.0))
                .px(px(theme::space::SM))
                .rounded(px(theme::radius::PILL))
                .cursor_pointer()
                .bg(if active {
                    theme::accent_soft()
                } else {
                    theme::surface_elevated()
                })
                .text_size(theme::Text::Small.size())
                .text_color(if active {
                    theme::accent()
                } else {
                    theme::text_muted()
                })
                .hover(|style| style.bg(theme::surface_hover()))
                .on_click(cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                    handle_nav_click(root, nav, cx);
                }))
                .child(
                    svg()
                        .path(icons::path(icon_name(nav)))
                        .size(px(theme::ICON_SM))
                        .text_color(if active {
                            theme::accent()
                        } else {
                            theme::text_muted()
                        }),
                )
                .child(label)
                .into_any_element()
        })
        .collect();

    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap(px(theme::space::XS))
        .w_full()
        .px(px(theme::space::MD))
        .py(px(theme::space::SM))
        .child(
            div()
                .id("top-nav-provider-btn")
                .flex()
                .flex_row()
                .items_center()
                .gap(px(theme::space::XS))
                .h(px(30.0))
                .px(px(theme::space::SM))
                .rounded(px(theme::radius::PILL))
                .bg(theme::surface_elevated())
                .cursor_pointer()
                .hover(|style| style.bg(theme::surface_hover()))
                .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                    root.provider_menu_open = !root.provider_menu_open;
                    cx.notify();
                }))
                .child(provider_badge(root.active_provider(), 18.0))
                .child(
                    div()
                        .text_size(theme::Text::Small.size())
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme::text())
                        .child(root.provider_display_name(root.active_provider())),
                )
                .child(
                    div()
                        .text_size(theme::Text::Tiny.size())
                        .text_color(theme::text_muted())
                        .child("▾"),
                ),
        )
        .children(items)
}

/// 左上角音乐源切换浮层（支持 汽水音乐 ⇄ 咪咕音乐 一键切换与账号状态展示）。
pub fn provider_popover(root: &Root, cx: &mut Context<Root>) -> AnyElement {
    let active = root.active_provider();
    let soda_logged = !root.settings.cookie.trim().is_empty();
    let migu_logged = !root.settings.migu_cookie.trim().is_empty();

    let soda_sub = if soda_logged {
        if root.account.as_ref().is_some_and(|a| a.vip) {
            root.tr("VIP 会员 · 无损直连").to_string()
        } else {
            root.tr("已登录").to_string()
        }
    } else {
        root.tr("需扫码登录后使用").to_string()
    };

    let migu_sub = if migu_logged {
        if root.migu_account.as_ref().is_some_and(|a| a.vip) {
            root.tr("VIP 会员 · 24bit 母带").to_string()
        } else {
            root.tr("已登录").to_string()
        }
    } else {
        root.tr("免登录可用 · 登录后享 VIP 音质").to_string()
    };

    let option = |id: &'static str, provider: MusicProvider, sub: String| {
        let selected = active == provider;
        div()
            .id(id)
            .flex()
            .flex_row()
            .items_center()
            .gap(px(theme::space::MD))
            .px(px(theme::space::MD))
            .py(px(theme::space::SM))
            .rounded(px(theme::radius::ROW))
            .cursor_pointer()
            .when(selected, |this| this.bg(theme::surface_selected()))
            .hover(|style| style.bg(theme::surface_hover()))
            .on_click(cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                root.set_music_provider(provider, cx);
            }))
            .child(provider_badge(provider, 28.0))
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
                            .font_weight(if selected {
                                gpui::FontWeight::SEMIBOLD
                            } else {
                                gpui::FontWeight::MEDIUM
                            })
                            .text_color(if selected {
                                theme::accent()
                            } else {
                                theme::text()
                            })
                            .child(root.provider_display_name(provider)),
                    )
                    .child(
                        div()
                            .truncate()
                            .text_size(theme::Text::Tiny.size())
                            .text_color(theme::text_muted())
                            .child(sub),
                    ),
            )
            .when(selected, |this| {
                this.child(
                    svg()
                        .path(icons::path("check"))
                        .size(px(theme::ICON_SM))
                        .text_color(theme::accent()),
                )
            })
    };

    let top_offset = if cfg!(target_os = "macos") {
        76.0
    } else {
        84.0
    };

    let scrim = div()
        .id("provider-popover-scrim")
        .absolute()
        .size_full()
        .on_mouse_down(
            gpui::MouseButton::Left,
            cx.listener(|root, _event, _window, cx| {
                root.provider_menu_open = false;
                cx.notify();
            }),
        );

    let card = div()
        .id("provider-popover-card")
        .absolute()
        .top(px(top_offset))
        .left(px(theme::space::MD))
        .w(px(252.0))
        .p(px(theme::space::SM))
        .flex()
        .flex_col()
        .gap(px(theme::space::XS))
        .rounded(px(theme::radius::CARD))
        .bg(theme::surface_elevated())
        .border_1()
        .border_color(theme::border())
        .shadow_lg()
        .on_mouse_down(gpui::MouseButton::Left, |_event, _window, cx| {
            cx.stop_propagation();
        })
        .child(
            div()
                .px(px(theme::space::MD))
                .py(px(4.0))
                .text_size(theme::Text::Tiny.size())
                .text_color(theme::text_faint())
                .child(root.tr("流光音乐 · 切换音源")),
        )
        .child(option(
            "switch-provider-soda",
            MusicProvider::Soda,
            soda_sub,
        ))
        .child(option(
            "switch-provider-migu",
            MusicProvider::Migu,
            migu_sub,
        ))
        .child(
            div()
                .h(px(1.0))
                .mx(px(theme::space::SM))
                .my(px(2.0))
                .bg(theme::border()),
        )
        .child(
            div()
                .id("provider-manage-accounts")
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .px(px(theme::space::MD))
                .py(px(theme::space::SM))
                .rounded(px(theme::radius::ROW))
                .cursor_pointer()
                .hover(|style| style.bg(theme::surface_hover()))
                .text_size(theme::Text::Small.size())
                .text_color(theme::text_muted())
                .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                    root.provider_menu_open = false;
                    root.settings_section = SettingsSection::Account;
                    root.set_nav(Nav::Settings, cx);
                }))
                .child(root.tr("管理账号与会员"))
                .child(
                    svg()
                        .path(icons::path("chevron-right"))
                        .size(px(theme::ICON_SM))
                        .text_color(theme::text_muted()),
                ),
        );

    gpui::deferred(div().absolute().size_full().child(scrim).child(card))
        .with_priority(9)
        .into_any_element()
}
