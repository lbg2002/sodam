//! 独立桌面歌词窗口。

use gpui::prelude::*;
use gpui::{
    div, point, px, size, App, Bounds, Context, Entity, IntoElement, Render, TitlebarOptions,
    Window, WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions,
};

use crate::app::Root;
use crate::ui::theme;

pub fn open(root: Entity<Root>, cx: &mut App) {
    if let Some(handle) = cx
        .windows()
        .iter()
        .find_map(|window| window.downcast::<DesktopLyrics>())
    {
        let _ = handle.update(cx, |_view, window, _cx| window.activate_window());
        cx.activate(true);
        return;
    }
    let settings = root.read(cx).settings.clone();
    let window_size = size(
        px(settings.desktop_lyrics_w.max(520.0)),
        px(settings.desktop_lyrics_h.max(120.0)),
    );
    let bounds = if settings.desktop_lyrics_x != 0.0 || settings.desktop_lyrics_y != 0.0 {
        Bounds::new(
            point(px(settings.desktop_lyrics_x), px(settings.desktop_lyrics_y)),
            window_size,
        )
    } else {
        Bounds::centered(None, window_size, cx)
    };
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        window_min_size: Some(size(px(520.0), px(120.0))),
        kind: if settings.desktop_lyrics_always_on_top {
            WindowKind::Floating
        } else {
            WindowKind::Normal
        },
        is_movable: !settings.desktop_lyrics_locked,
        is_resizable: !settings.desktop_lyrics_locked,
        window_background: WindowBackgroundAppearance::Transparent,
        titlebar: Some(TitlebarOptions {
            title: Some("SodaM Lyrics".into()),
            ..Default::default()
        }),
        app_id: Some("SodaM".into()),
        ..Default::default()
    };
    let _ = cx.open_window(options, move |_window, cx| {
        cx.new(|cx| DesktopLyrics::new(root, cx))
    });
}

pub struct DesktopLyrics {
    root: Entity<Root>,
}

impl DesktopLyrics {
    pub fn new(root: Entity<Root>, cx: &mut Context<Self>) -> Self {
        cx.observe(&root, |_view, _root, cx| {
            cx.notify();
        })
        .detach();
        Self { root }
    }
}

impl Render for DesktopLyrics {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let bounds = window.bounds();
        let x = f32::from(bounds.origin.x);
        let y = f32::from(bounds.origin.y);
        let width = f32::from(bounds.size.width);
        let height = f32::from(bounds.size.height);
        self.root.update(cx, |root, _cx| {
            if (root.settings.desktop_lyrics_x - x).abs() > 1.0
                || (root.settings.desktop_lyrics_y - y).abs() > 1.0
                || (root.settings.desktop_lyrics_w - width).abs() > 1.0
                || (root.settings.desktop_lyrics_h - height).abs() > 1.0
            {
                root.settings.desktop_lyrics_x = x;
                root.settings.desktop_lyrics_y = y;
                root.settings.desktop_lyrics_w = width;
                root.settings.desktop_lyrics_h = height;
                let _ = root.settings.save();
            }
        });

        let root = self.root.read(cx);
        #[cfg(target_os = "linux")]
        if root.settings.desktop_lyrics_click_through {
            window.set_input_region(Some(&[]));
        } else {
            window.set_input_region(None);
        }
        let active = root.lyrics_active.unwrap_or(0);
        let current = root
            .lyrics
            .get(active)
            .map(|line| line.text.clone())
            .filter(|text| !text.trim().is_empty())
            .unwrap_or_else(|| {
                root.queue
                    .current()
                    .map(|track| track.title.clone())
                    .unwrap_or_else(|| root.tr("暂无歌词").to_string())
            });
        let next = root
            .lyrics
            .get(active.saturating_add(1))
            .map(|line| line.text.clone())
            .unwrap_or_default();
        let base = root.settings.lyrics_font_size.clamp(14, 30) as f32;
        let mut background = theme::bg();
        background.a = (root.settings.desktop_lyrics_opacity.clamp(30, 100) as f32 / 100.0)
            .clamp(0.3, 1.0);
        let align = root.settings.desktop_lyrics_align.as_str();

        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(theme::space::SM))
            .px(px(theme::space::XL))
            .bg(background)
            .child(
                div()
                    .w_full()
                    .when(align == "left", |this| this.text_left())
                    .when(align == "center", |this| this.text_center())
                    .when(align == "right", |this| this.text_right())
                    .text_size(px((base + 10.0).clamp(24.0, 40.0)))
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(theme::accent())
                    .child(current),
            )
            .when(!root.settings.desktop_lyrics_single_line && !next.is_empty(), |this| {
                this.child(
                    div()
                        .w_full()
                        .when(align == "left", |this| this.text_left())
                        .when(align == "center", |this| this.text_center())
                        .when(align == "right", |this| this.text_right())
                        .truncate()
                        .text_size(px((base + 1.0).clamp(16.0, 28.0)))
                        .text_color(theme::text_muted())
                        .child(next),
                )
            })
    }
}
