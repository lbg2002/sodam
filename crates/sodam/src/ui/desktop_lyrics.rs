//! 独立桌面歌词窗口。

use gpui::prelude::*;
use gpui::{
    div, px, size, App, Bounds, Context, Entity, IntoElement, Render, TitlebarOptions, Window,
    WindowBounds, WindowKind, WindowOptions,
};

use crate::app::Root;
use crate::ui::theme;

pub fn open(root: Entity<Root>, cx: &mut App) {
    let bounds = Bounds::centered(None, size(px(760.0), px(170.0)), cx);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        window_min_size: Some(size(px(520.0), px(150.0))),
        kind: WindowKind::Floating,
        is_resizable: true,
        titlebar: Some(TitlebarOptions {
            title: Some("SodaM Lyrics".into()),
            ..Default::default()
        }),
        app_id: Some("SodaM".into()),
        ..Default::default()
    };
    let _ = cx.open_window(options, move |_window, cx| {
        cx.new(|_| DesktopLyrics::new(root))
    });
}

pub struct DesktopLyrics {
    root: Entity<Root>,
}

impl DesktopLyrics {
    pub fn new(root: Entity<Root>) -> Self {
        Self { root }
    }
}

impl Render for DesktopLyrics {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let root = self.root.read(cx);
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

        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(theme::space::SM))
            .px(px(theme::space::XL))
            .bg(theme::ambient_background())
            .child(
                div()
                    .w_full()
                    .text_center()
                    .text_size(px((base + 10.0).clamp(24.0, 40.0)))
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(theme::accent())
                    .child(current),
            )
            .when(!next.is_empty(), |this| {
                this.child(
                    div()
                        .w_full()
                        .text_center()
                        .truncate()
                        .text_size(px((base + 1.0).clamp(16.0, 28.0)))
                        .text_color(theme::text_muted())
                        .child(next),
                )
            })
    }
}
