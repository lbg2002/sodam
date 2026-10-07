//! 应用自绘标题栏：拖拽区 + 最小化/最大化/关闭按钮。
//!
//! Windows 通过 `window_control_area` 交给系统完成命中测试；Linux/Wayland
//! 不能依赖这套 Windows 命中机制，因此在控件上显式调用 GPUI 的窗口操作 API。
//! 这样 Ubuntu 新版采用客户端装饰时，顶部空白区仍可拖动，窗口按钮也始终可用。

use gpui::prelude::*;
use gpui::{
    div, hsla, px, svg, ClickEvent, Div, Hsla, MouseButton, MouseDownEvent, WindowControlArea,
};

use crate::ui::{icons, theme};

/// 标题栏条高度（逻辑像素）。
const STRIP_HEIGHT: f32 = 32.0;
/// 单个标题栏按钮宽度。
const BUTTON_WIDTH: f32 = 44.0;
/// 控制图标边长。
const GLYPH_SIZE: f32 = 12.0;
/// 关闭按钮悬停色（沿用 Windows 惯例红）。
const CLOSE_HOVER: Hsla = hsla(4.0 / 360.0, 0.83, 0.49, 1.0);

/// 渲染标题栏条：左侧空白为拖拽区，右侧是三个窗口控制按钮。
pub fn render() -> Div {
    div()
        .flex()
        .flex_row()
        .items_center()
        .w_full()
        .h(px(STRIP_HEIGHT))
        .flex_none()
        .child(drag_area())
        .child(caption_button(
            "titlebar-min",
            "minus",
            WindowControlArea::Min,
            false,
        ))
        .child(caption_button(
            "titlebar-max",
            "square",
            WindowControlArea::Max,
            false,
        ))
        .child(caption_button(
            "titlebar-close",
            "x",
            WindowControlArea::Close,
            true,
        ))
}

fn drag_area() -> impl IntoElement {
    div()
        .id("titlebar-drag")
        .flex_1()
        .h_full()
        .window_control_area(WindowControlArea::Drag)
        .when(cfg!(target_os = "linux"), |this| {
            this.on_mouse_down(
                MouseButton::Left,
                |_event: &MouseDownEvent, window, _cx| {
                    window.start_window_move();
                },
            )
        })
}

fn caption_button(
    id: &'static str,
    icon: &'static str,
    area: WindowControlArea,
    danger: bool,
) -> impl IntoElement {
    let base = div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .w(px(BUTTON_WIDTH))
        .h(px(STRIP_HEIGHT))
        .window_control_area(area)
        .group(id)
        .when(cfg!(target_os = "linux"), |this| {
            this.on_click(move |_event: &ClickEvent, window, _cx| match area {
                WindowControlArea::Min => window.minimize_window(),
                WindowControlArea::Max => window.zoom_window(),
                WindowControlArea::Close => window.remove_window(),
                WindowControlArea::Drag => window.start_window_move(),
            })
        })
        .child(
            svg()
                .path(icons::path(icon))
                .size(px(GLYPH_SIZE))
                .text_color(theme::text_muted())
                .group_hover(id, |style| {
                    if danger {
                        style.text_color(gpui::white())
                    } else {
                        style.text_color(theme::text())
                    }
                }),
        );

    if danger {
        base.hover(|style| style.bg(CLOSE_HOVER))
    } else {
        base.hover(|style| style.bg(theme::surface_hover()))
    }
}
