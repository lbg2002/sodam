//! 左侧导航栏与响应式紧凑导航：品牌区 + 「我的音乐」分组 + 导航行 + 主题/模式快捷切换。
//!
//! 支持三种响应式形态：
//! 1. **完整侧边栏 (`render`)**：宽屏普通模式下展示图标 + 文字 + 中式主题徽章；
//! 2. **紧凑图标栏 (`render_compact`)**：半屏播放模式或中等窄窗（`< 980px`）自动收窄为 `64px` 图标轨；
//! 3. **顶部横向导航条 (`render_top_nav`)**：竖向窄窗口（`< 680px`）下转为顶部紧凑胶囊导航，不挤压内容宽度。

use gpui::prelude::*;
use gpui::{div, img, px, svg, AnyElement, ClickEvent, Context, IntoElement};

use crate::app::{Nav, Root};
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
    if root.settings.cookie.trim().is_empty()
        && !matches!(
            nav,
            Nav::Settings
                | Nav::Downloads
                | Nav::Recent
                | Nav::LocalMusic
                | Nav::LocalPlaylists
                | Nav::Lyrics
        )
    {
        root.set_nav(Nav::Settings, cx);
        root.set_status("请先在「设置 → 账户」扫码登录", &[]);
        return;
    }
    if nav == Nav::Lyrics {
        root.open_playback(cx);
    } else {
        root.set_nav(nav, cx);
    }
}

fn nav_row(nav: Nav, label: String, selected: bool, cx: &mut Context<Root>) -> AnyElement {
    let foreground = if selected {
        theme::text()
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
            this.bg(theme::surface_selected())
                .border_l_2()
                .border_color(theme::accent())
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
        .when(selected, |this| {
            this.bg(theme::surface_selected())
                .border_1()
                .border_color(theme::accent())
        })
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
        .bg(theme::bg())
        .border_r_1()
        .border_color(theme::border())
        .p(px(theme::space::MD))
        .gap(px(theme::space::XS))
        .child(
            // 品牌区：强调色圆角块 + 汽水音乐名称
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(theme::space::MD))
                .px(px(theme::space::MD))
                .py(px(theme::space::SM))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(28.0))
                        .rounded(px(theme::radius::ROW))
                        .bg(theme::accent_soft())
                        .overflow_hidden()
                        .child(img("icons/sodam-logo.svg").size(px(28.0))),
                )
                .child(
                    div()
                        .text_size(theme::Text::Large.size())
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(theme::text())
                        .child(root.tr("汽水音乐")),
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
        .bg(theme::bg())
        .border_r_1()
        .border_color(theme::border())
        .py(px(theme::space::MD))
        .gap(px(theme::space::XS))
        .child(
            div()
                .id("sidebar-compact-logo")
                .flex()
                .items_center()
                .justify_center()
                .size(px(36.0))
                .mb(px(theme::space::SM))
                .rounded(px(theme::radius::ROW))
                .bg(theme::accent_soft())
                .cursor_pointer()
                .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                    root.set_nav(Nav::Lyrics, cx);
                }))
                .child(img("icons/sodam-logo.svg").size(px(26.0))),
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
        .bg(theme::bg())
        .border_b_1()
        .border_color(theme::border())
        .children(items)
}
