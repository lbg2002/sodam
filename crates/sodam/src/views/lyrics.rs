//! 歌词播放页。

use super::scenes::retry_button;
use super::*;
/// 歌词播放页：左封面，右滚动歌词；展开队列时自动切为「缩小封面 + 封面下方精简歌词 + 右侧队列」。
pub(crate) fn lyrics_view(root: &Root, window: &Window, cx: &mut Context<Root>) -> AnyElement {
    if root.switching_queue.is_some() {
        return div()
            .flex()
            .flex_col()
            .size_full()
            .child(loading_state(&root.switching_label))
            .into_any_element();
    }

    let snapshot = root.engine.snapshot();
    let Some(track) = root
        .pending_track
        .clone()
        .or_else(|| root.queue.current().cloned())
    else {
        return div()
            .flex()
            .flex_col()
            .size_full()
            .child(loading_state(root.tr("正在准备播放…")))
            .into_any_element();
    };

    let liked = root.liked_ids.contains(&track.id);
    let active = root.lyrics_active;
    let lyric_font_size = root.settings.lyrics_font_size.clamp(14, 30) as f32;
    let active_font_size = (lyric_font_size + 7.0).min(38.0);
    let lyric_line_height = root.settings.lyrics_line_height.clamp(24, 52) as f32;
    let lyric_row_height = (lyric_line_height * 2.0).max(active_font_size + 32.0);
    let lines = root.lyrics.clone();
    let list = if root.lyrics.is_empty() {
        None
    } else {
        Some(
            uniform_list(
                "lyrics-list",
                root.lyrics.len(),
                move |range, _window, _cx| {
                    range
                        .map(|index| {
                            let Some(line) = lines.get(index) else {
                                return div().h(px(lyric_row_height)).into_any_element();
                            };
                            let is_active = active == Some(index);
                            let color = if is_active {
                                theme::accent()
                            } else {
                                theme::text_muted()
                            };
                            div()
                                .w_full()
                                .flex()
                                .flex_wrap()
                                .items_center()
                                .h(px(lyric_row_height))
                                .text_size(px(if is_active {
                                    active_font_size
                                } else {
                                    lyric_font_size
                                }))
                                .line_height(px(lyric_line_height))
                                .text_color(color)
                                .when(is_active, |this| {
                                    this.font_weight(gpui::FontWeight::BOLD)
                                        .px(px(theme::space::SM))
                                        .rounded(px(theme::radius::ROW))
                                        .bg(theme::accent_soft())
                                })
                                .child(line.text.clone())
                                .into_any_element()
                        })
                        .collect::<Vec<_>>()
                },
            )
            .flex_1()
            .min_h(px(0.0))
            .track_scroll(&root.lyrics_scroll)
            .into_any_element(),
        )
    };

    let cover_path = root
        .original_covers
        .get(&track.cover)
        .cloned()
        .or_else(|| root.cover_of(&track.cover));
    let original_cover_ready = root.original_covers.contains_key(&track.cover);
    let downloaded = root.downloaded_ids.contains(&track.id);
    let pending_download = root.pending_downloads.contains_key(&track.id);
    let download_inflight = root.download_inflight.contains(&track.id);

    let like_button = div()
        .id("playback-like")
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .size(px(theme::size::CONTROL_SM))
        .rounded(px(theme::radius::PILL))
        .cursor_pointer()
        .hover(|style| style.bg(theme::surface_hover()))
        .on_click(cx.listener({
            let track = track.clone();
            move |root, _event: &ClickEvent, _window, cx| {
                root.toggle_like(track.clone(), cx);
            }
        }))
        .child(
            svg()
                .path(icons::path(if liked { "heart-filled" } else { "heart" }))
                .size(px(theme::ICON_SM))
                .text_color(if liked {
                    theme::accent()
                } else {
                    theme::text_muted()
                }),
        );

    let download_button = div()
        .id("playback-download")
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .size(px(theme::size::CONTROL_SM))
        .rounded(px(theme::radius::PILL))
        .cursor_pointer()
        .hover(|style| style.bg(theme::surface_hover()))
        .on_click(cx.listener({
            let track = track.clone();
            move |root, _event: &ClickEvent, _window, cx| {
                root.toggle_download(track.clone(), cx);
            }
        }))
        .child(
            svg()
                .path(icons::path(if downloaded { "check" } else { "download" }))
                .size(px(theme::ICON_SM))
                .text_color(if downloaded || pending_download {
                    theme::accent()
                } else if download_inflight {
                    theme::text_faint()
                } else {
                    theme::text_muted()
                }),
        );

    // 正常播放页仍沿用官方风格：封面 min(40vh, 30vw)，歌词列约 30vw。
    // 队列展开时给右侧队列固定更多空间，并主动缩小封面，避免只是把原歌词列
    // 生硬替换成队列后造成左侧视觉中心不变、歌词完全消失的问题。
    let viewport_width = f32::from(window.viewport_size().width);
    let viewport_height = (f32::from(window.viewport_size().height) - theme::PLAYER_H).max(360.0);
    let mut cover_size = if root.queue_open {
        (viewport_height * 0.32)
            .min(viewport_width * 0.22)
            .clamp(180.0, 330.0)
    } else {
        (viewport_height * 0.40).min(viewport_width * 0.30)
    };
    let mut lyrics_width = (viewport_width * 0.30).clamp(300.0, 500.0);

    // 窗口被平铺 WM 压得比官方最小尺寸还窄时，整体收缩。
    let centered_width = (viewport_width - theme::SIDEBAR_W - 100.0).max(220.0);
    let natural_width = if root.queue_open {
        cover_size
    } else {
        cover_size + 50.0 + lyrics_width
    };
    if natural_width > centered_width {
        let scale = centered_width / natural_width;
        cover_size *= scale;
        if !root.queue_open {
            lyrics_width *= scale;
        }
    }

    let track_info = div()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(theme::space::XS))
        .w(px(cover_size))
        .max_w_full()
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_center()
                .gap(px(theme::space::SM))
                .max_w_full()
                .min_w(px(0.0))
                .child(
                    div()
                        .min_w(px(0.0))
                        .truncate()
                        .text_size(theme::Text::Large.size())
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(theme::text())
                        .child(track.title.clone()),
                )
                .when(track.vip, |this| this.child(vip_badge()))
                .child(like_button)
                .child(download_button),
        )
        .child(
            div()
                .id("playback-artist-link")
                .max_w_full()
                .truncate()
                .cursor_pointer()
                .text_size(theme::Text::Small.size())
                .text_color(theme::text_muted())
                .hover(|style| style.text_color(theme::accent()))
                .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                    cx.stop_propagation();
                })
                .on_click(cx.listener({
                    let track = track.clone();
                    move |root, _event: &ClickEvent, _window, cx| {
                        root.open_track_artist(track.clone(), cx);
                    }
                }))
                .child(track.artist.clone()),
        );

    // 展开队列后，不再把歌词彻底拿掉：在封面下方保留当前行和下一行。
    // 当前行加粗/强调色，下一行弱化；高度固定，避免歌词长短导致封面上下跳动。
    let compact_lyrics = if root.queue_open {
        let active_index = active.unwrap_or(0);
        let current = root
            .lyrics
            .get(active_index)
            .map(|line| line.text.clone())
            .filter(|text| !text.trim().is_empty())
            .unwrap_or_else(|| track.title.clone());
        let next = root
            .lyrics
            .get(active_index.saturating_add(1))
            .map(|line| line.text.clone())
            .unwrap_or_default();
        Some(
            div()
                .w(px(cover_size))
                .max_w_full()
                .min_h(px(74.0))
                .mt(px(theme::space::SM))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(theme::space::XS))
                .child(
                    div()
                        .w_full()
                        .text_center()
                        .truncate()
                        .text_size(px((lyric_font_size + 2.0).min(26.0)))
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
                            .text_size(px((lyric_font_size - 1.0).max(14.0)))
                            .text_color(theme::text_muted())
                            .child(next),
                    )
                })
                .into_any_element(),
        )
    } else {
        None
    };

    let right_module = if root.queue_open {
        None
    } else {
        Some(
            div()
            .flex()
            .flex_col()
            .w(px(lyrics_width))
            .max_w(px(500.0))
            .min_w(px(260.0))
            .h_full()
            .flex_none()
            .min_h(px(0.0))
            .gap(px(theme::space::MD))
            .child(
                div()
                    .text_size(theme::Text::Title.size())
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(theme::text())
                    .truncate()
                    .child(track.title.clone()),
            )
            .child(
                div()
                    .text_size(theme::Text::Small.size())
                    .text_color(theme::text_muted())
                    .child(format!(
                        "{} · {} · {}",
                        track.artist,
                        track.duration_label(),
                        snapshot.progress_label()
                    )),
            )
            .child(if root.loading_lyrics {
                loading_state(root.tr("正在读取歌词…")).into_any_element()
            } else if let Some(error) = &root.lyrics_error {
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(theme::space::SM))
                    .child(
                        div()
                            .text_size(theme::Text::Small.size())
                            .text_color(theme::text_muted())
                            .child(error.clone()),
                    )
                    .child(retry_button(root.language, cx))
                    .into_any_element()
            } else if root.lyrics.is_empty() {
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(theme::space::SM))
                    .child(empty_state("music", root.tr("这首歌暂时没有歌词")))
                    .child(retry_button(root.language, cx))
                    .into_any_element()
            } else {
                list.expect("loaded lyrics have a list")
            })
            .into_any_element(),
        )
    };

    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_h(px(0.0))
        .px(px(50.0))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_center()
                .flex_1()
                .min_h(px(0.0))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .h_full()
                        .items_start()
                        .justify_center()
                        .max_w_full()
                        .flex_none()
                        .min_h(px(0.0))
                        .gap(px(50.0))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .items_center()
                                .justify_center()
                                .h_full()
                                .gap(px(if root.queue_open {
                                    theme::space::MD
                                } else {
                                    theme::space::LG
                                }))
                                .w(px(cover_size))
                                .flex_none()
                                .child(crate::ui::artwork::cover_loading(
                                    cover_path,
                                    cover_size,
                                    theme::radius::CARD,
                                ))
                                .when(!original_cover_ready, |this| {
                                    this.child(retry_button(root.language, cx))
                                })
                                .child(track_info)
                                .when_some(compact_lyrics, |this, lyrics| this.child(lyrics)),
                        )
                        .when_some(right_module, |this, module| this.child(module)),
                ),
        )
        .into_any_element()
}
