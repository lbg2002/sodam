//! 独立迷你播放器窗口。

use gpui::prelude::*;
use gpui::{
    div, px, size, svg, App, Bounds, ClickEvent, Context, Entity, IntoElement, Render, TitlebarOptions,
    Window, WindowBounds, WindowKind, WindowOptions,
};

use crate::app::Root;
use crate::ui::artwork::cover;
use crate::ui::{icons, theme};

pub fn open(root: Entity<Root>, cx: &mut App) {
    let bounds = Bounds::centered(None, size(px(430.0), px(142.0)), cx);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        window_min_size: Some(size(px(380.0), px(130.0))),
        kind: WindowKind::Floating,
        is_resizable: false,
        titlebar: Some(TitlebarOptions {
            title: Some("SodaM Mini".into()),
            ..Default::default()
        }),
        app_id: Some("SodaM".into()),
        ..Default::default()
    };
    let _ = cx.open_window(options, move |_window, cx| {
        cx.new(|_| MiniPlayer::new(root))
    });
}

pub struct MiniPlayer {
    root: Entity<Root>,
}

impl MiniPlayer {
    pub fn new(root: Entity<Root>) -> Self {
        Self { root }
    }
}

impl Render for MiniPlayer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let root = self.root.read(cx);
        let snapshot = root.engine.snapshot();
        let track = root
            .pending_track
            .clone()
            .or_else(|| root.queue.current().cloned());
        let cover_path = track.as_ref().and_then(|track| root.cover_of(&track.cover));
        let title = track
            .as_ref()
            .map(|track| track.title.clone())
            .unwrap_or_else(|| root.tr("未在播放").to_string());
        let artist = track
            .as_ref()
            .map(|track| track.artist.clone())
            .unwrap_or_else(|| root.tr("选择一首歌开始播放吧").to_string());
        let progress = snapshot.progress_fraction().clamp(0.0, 1.0);
        let progress_width = 178.0 * progress;
        let entity = self.root.clone();

        div()
            .size_full()
            .flex()
            .flex_col()
            .gap(px(theme::space::SM))
            .p(px(theme::space::MD))
            .bg(theme::bg())
            .text_color(theme::text())
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(theme::space::MD))
                    .child(cover(cover_path, 58.0, theme::radius::ART))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w(px(0.0))
                            .gap(px(theme::space::XS))
                            .child(
                                div()
                                    .truncate()
                                    .text_size(theme::Text::Body.size())
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child(title),
                            )
                            .child(
                                div()
                                    .truncate()
                                    .text_size(theme::Text::Small.size())
                                    .text_color(theme::text_muted())
                                    .child(artist),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(theme::space::XS))
                            .child(
                                div()
                                    .id("mini-prev")
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .size(px(theme::size::CONTROL_SM))
                                    .rounded_full()
                                    .cursor_pointer()
                                    .hover(|style| style.bg(theme::surface_hover()))
                                    .on_click({
                                        let entity = entity.clone();
                                        move |_event: &ClickEvent, _window, cx| {
                                            entity.update(cx, |root, cx| root.prev_track(cx));
                                        }
                                    })
                                    .child(
                                        svg()
                                            .path(icons::path("skip-back"))
                                            .size(px(theme::ICON))
                                            .text_color(theme::text_muted()),
                                    ),
                            )
                            .child(
                                div()
                                    .id("mini-play")
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .size(px(36.0))
                                    .rounded_full()
                                    .bg(theme::accent())
                                    .cursor_pointer()
                                    .on_click({
                                        let entity = entity.clone();
                                        move |_event: &ClickEvent, _window, cx| {
                                            entity.update(cx, |root, cx| root.toggle_play(cx));
                                        }
                                    })
                                    .child(
                                        svg()
                                            .path(icons::path(if snapshot.playing {
                                                "pause"
                                            } else {
                                                "play"
                                            }))
                                            .size(px(theme::ICON))
                                            .text_color(theme::accent_foreground()),
                                    ),
                            )
                            .child(
                                div()
                                    .id("mini-next")
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .size(px(theme::size::CONTROL_SM))
                                    .rounded_full()
                                    .cursor_pointer()
                                    .hover(|style| style.bg(theme::surface_hover()))
                                    .on_click({
                                        let entity = entity.clone();
                                        move |_event: &ClickEvent, _window, cx| {
                                            entity.update(cx, |root, cx| root.next_track(cx));
                                        }
                                    })
                                    .child(
                                        svg()
                                            .path(icons::path("skip-forward"))
                                            .size(px(theme::ICON))
                                            .text_color(theme::text_muted()),
                                    ),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(theme::space::SM))
                    .child(
                        div()
                            .text_size(theme::Text::Tiny.size())
                            .text_color(theme::text_faint())
                            .child(snapshot.position_label()),
                    )
                    .child(
                        div()
                            .relative()
                            .w(px(178.0))
                            .h(px(4.0))
                            .rounded(px(2.0))
                            .bg(theme::surface_hover())
                            .child(
                                div()
                                    .absolute()
                                    .left(px(0.0))
                                    .top(px(0.0))
                                    .w(px(progress_width))
                                    .h_full()
                                    .rounded(px(2.0))
                                    .bg(theme::accent()),
                            ),
                    )
                    .child(
                        div()
                            .text_size(theme::Text::Tiny.size())
                            .text_color(theme::text_faint())
                            .child(snapshot.duration_label()),
                    )
                    .when(root.settings.offline_mode, |this| {
                        this.child(
                            div()
                                .px(px(theme::space::SM))
                                .py(px(1.0))
                                .rounded(px(theme::radius::PILL))
                                .bg(theme::accent_soft())
                                .text_size(theme::Text::Tiny.size())
                                .text_color(theme::accent())
                                .child(root.tr("离线")),
                        )
                    }),
            )
    }
}
