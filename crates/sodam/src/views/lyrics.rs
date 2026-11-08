//! 播放页与沉浸歌词体验：支持普通双栏、纯享歌词、内联队列、半屏模式、全屏沉浸模式与紧凑小窗模式。

use super::scenes::retry_button;
use super::*;
use gpui::{Animation, AnimationExt};
use std::time::Duration;

/// 判断当前音频流是否为试听片段（严禁将试听伪装成完整歌曲）。
pub(crate) fn is_preview_stream(quality: &str) -> bool {
    let lower = quality.to_ascii_lowercase();
    quality.contains("试听") || lower.contains("preview") || lower.contains("trial")
}

/// 试听徽章：醒目提示当前播放为试听片段。
pub(crate) fn preview_badge(root: &Root) -> AnyElement {
    div()
        .px(px(theme::space::SM))
        .py(px(2.0))
        .rounded(px(theme::radius::PILL))
        .bg(theme::accent_soft())
        .border_1()
        .border_color(theme::accent())
        .text_size(theme::Text::Tiny.size())
        .font_weight(gpui::FontWeight::BOLD)
        .text_color(theme::accent())
        .child(root.tr("试听"))
        .into_any_element()
}

/// 播放失败横幅：展示明确错误原因、当前网络/离线/签名状态以及重试入口。
fn playback_error_banner(root: &Root, error: &str, cx: &mut Context<Root>) -> AnyElement {
    let net_state = if root.settings.offline_mode {
        root.tr("当前状态：离线模式（仅本地缓存）")
    } else if root.settings.is_ready_for_vip() {
        root.tr("当前状态：在线 · 已配置登录与签名")
    } else {
        root.tr("当前状态：未完成登录或签名配置")
    };

    div()
        .w_full()
        .p(px(theme::space::MD))
        .rounded(px(theme::radius::CARD))
        .bg(theme::surface_elevated())
        .border_1()
        .border_color(theme::accent())
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
                .gap(px(2.0))
                .min_w(px(180.0))
                .flex_1()
                .child(
                    div()
                        .text_size(theme::Text::Small.size())
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(theme::accent())
                        .child(error.to_string()),
                )
                .child(
                    div()
                        .text_size(theme::Text::Tiny.size())
                        .text_color(theme::text_muted())
                        .child(net_state),
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
                        .id("playback-error-retry")
                        .px(px(theme::space::MD))
                        .py(px(theme::space::XS))
                        .rounded(px(theme::radius::PILL))
                        .bg(theme::accent())
                        .text_size(theme::Text::Small.size())
                        .text_color(theme::accent_foreground())
                        .cursor_pointer()
                        .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                            root.playback_error = None;
                            root.start_track(cx);
                        }))
                        .child(root.tr("重试播放")),
                )
                .child(
                    div()
                        .id("playback-error-settings")
                        .px(px(theme::space::MD))
                        .py(px(theme::space::XS))
                        .rounded(px(theme::radius::PILL))
                        .bg(theme::surface_hover())
                        .text_size(theme::Text::Small.size())
                        .text_color(theme::text())
                        .cursor_pointer()
                        .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                            root.set_nav(Nav::Settings, cx);
                        }))
                        .child(root.tr("网络与设置")),
                ),
        )
        .into_any_element()
}

/// 判断相邻两个逐字歌词片段之间是否需要补充西文空格。
fn needs_english_space(left: &str, right: &str) -> bool {
    let Some(last_char) = left.chars().last() else {
        return false;
    };
    let Some(first_char) = right.chars().next() else {
        return false;
    };
    let left_is_latin = last_char.is_ascii_alphanumeric()
        || matches!(
            last_char,
            ',' | '.' | '\'' | '’' | '!' | '?' | ';' | ':' | ')'
        );
    let right_is_latin =
        first_char.is_ascii_alphanumeric() || matches!(first_char, '\'' | '’' | '"' | '(');
    left_is_latin && right_is_latin
}

/// 根据窗口宽高计算歌词字号自适应增益（大窗口下自动放大歌词与行高）。
fn window_lyrics_boost(viewport_w: f32, viewport_h: f32) -> f32 {
    let w_boost = ((viewport_w - 920.0) / 120.0).clamp(0.0, 9.0);
    let h_boost = ((viewport_h - 580.0) / 75.0).clamp(0.0, 9.0);
    (w_boost * 0.55 + h_boost * 0.45).clamp(0.0, 9.0)
}

/// 渲染单行歌词（支持逐字 LRC 染色与普通 LRC，保留英文单词间距）。
fn render_lyric_line_content(
    line: &sodam_core::models::LyricLine,
    is_active: bool,
    position_seconds: f64,
    active_color: gpui::Rgba,
    base_color: gpui::Rgba,
    align: theme::LyricsAlign,
    font_size: f32,
) -> AnyElement {
    if is_active && line.words.len() > 1 {
        let word_gap = (font_size * 0.28).max(5.0);
        let words: Vec<AnyElement> = line
            .words
            .iter()
            .enumerate()
            .map(|(idx, word)| {
                let reached = position_seconds + 0.05 >= word.start_seconds;
                let current_word = reached && position_seconds <= word.end_seconds + 0.08;
                let clean_word = word.text.trim().to_string();
                let has_space_after = idx + 1 < line.words.len()
                    && (word.text.ends_with(char::is_whitespace)
                        || line.words.get(idx + 1).is_some_and(|next| {
                            next.text.starts_with(char::is_whitespace)
                                || needs_english_space(&clean_word, next.text.trim())
                        }));
                div()
                    .text_color(if reached { active_color } else { theme::text() })
                    .when(has_space_after, |this| this.mr(px(word_gap)))
                    .when(current_word, |this| {
                        this.underline().text_decoration_color(active_color)
                    })
                    .child(clean_word)
                    .into_any_element()
            })
            .collect();
        let row = div()
            .flex()
            .flex_row()
            .flex_wrap()
            .items_center()
            .children(words);
        return match align {
            theme::LyricsAlign::Left => row.justify_start().into_any_element(),
            theme::LyricsAlign::Center => row.justify_center().into_any_element(),
            theme::LyricsAlign::Right => row.justify_end().into_any_element(),
        };
    }

    let text_div = div().text_color(if is_active { active_color } else { base_color });
    let aligned = match align {
        theme::LyricsAlign::Left => text_div.text_left(),
        theme::LyricsAlign::Center => text_div.text_center(),
        theme::LyricsAlign::Right => text_div.text_right(),
    };
    aligned.child(line.text.clone()).into_any_element()
}

/// 构建滚动歌词虚拟列表。
fn build_lyrics_list(
    root: &Root,
    snapshot: &sodam_core::audio::PlaybackSnapshot,
    font_boost: f32,
) -> Option<AnyElement> {
    if root.lyrics.is_empty() {
        return None;
    }
    let active = root.lyrics_active;
    let lyric_font_size =
        (root.settings.lyrics_font_size.clamp(14, 30) as f32 + font_boost).clamp(14.0, 36.0);
    let active_font_size = (lyric_font_size + 6.0 + font_boost * 0.35).clamp(20.0, 44.0);
    let lyric_line_height = (root.settings.lyrics_line_height.clamp(24, 52) as f32
        + font_boost * 1.45)
        .clamp(24.0, 68.0);
    let lyric_row_height = (lyric_line_height * 1.92).max(active_font_size + 28.0);
    let font_family =
        theme::LyricsFontFamily::from_key(&root.settings.lyrics_font_family).font_name();
    let align = theme::LyricsAlign::from_key(&root.settings.lyrics_align);
    let position_seconds =
        snapshot.position_seconds - root.settings.lyrics_offset_ms as f64 / 1000.0;
    let lines = root.lyrics.clone();
    let motion = theme::motion_enabled();

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
                        let distance = active
                            .map(|active_index| active_index.abs_diff(index))
                            .unwrap_or(3);
                        let base_color = if is_active {
                            theme::accent()
                        } else if distance == 1 {
                            theme::text()
                        } else if distance == 2 {
                            theme::text_muted()
                        } else {
                            theme::text_faint()
                        };
                        let opacity = match distance {
                            0 => 1.0,
                            1 => 0.78,
                            2 => 0.56,
                            3 => 0.40,
                            _ => 0.26,
                        };

                        let marker = if is_active {
                            let base_marker = div()
                                .id(("lyric-marker", index))
                                .flex_none()
                                .w(px(18.0))
                                .h(px(18.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_color(theme::accent())
                                .child(
                                    svg()
                                        .path(icons::path("music-playing"))
                                        .size(px(theme::ICON_SM)),
                                );
                            if motion {
                                base_marker
                                    .with_animation(
                                        ("lyric-marker-pulse", index),
                                        Animation::new(Duration::from_millis(1400))
                                            .repeat_synced()
                                            .with_max_fps(24.0),
                                        |this, phase| {
                                            this.layer_scale(
                                                0.92 + (phase * std::f32::consts::TAU).sin().abs()
                                                    * 0.10,
                                            )
                                        },
                                    )
                                    .into_any_element()
                            } else {
                                base_marker.into_any_element()
                            }
                        } else {
                            div().w(px(18.0)).h(px(18.0)).into_any_element()
                        };

                        let line_font_size = if is_active {
                            active_font_size
                        } else {
                            lyric_font_size
                        };
                        let content = render_lyric_line_content(
                            line,
                            is_active,
                            position_seconds,
                            theme::accent(),
                            base_color,
                            align,
                            line_font_size,
                        );

                        let row = div()
                            .w_full()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(theme::space::SM))
                            .h(px(lyric_row_height))
                            .px(px(theme::space::MD))
                            .font_family(font_family)
                            .text_size(px(line_font_size))
                            .line_height(px(lyric_line_height))
                            .opacity(opacity)
                            .when(is_active, |this| {
                                this.font_weight(gpui::FontWeight::BOLD)
                                    .rounded(px(theme::radius::CARD))
                                    .bg(theme::ink_glow())
                                    .border_l_2()
                                    .border_color(theme::accent())
                            });

                        let aligned_row = match align {
                            theme::LyricsAlign::Left => row.justify_start(),
                            theme::LyricsAlign::Center => row.justify_center(),
                            theme::LyricsAlign::Right => row.justify_end(),
                        };

                        aligned_row
                            .child(marker)
                            .child(div().min_w(px(0.0)).child(content))
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
}

/// 歌词微调与状态底部条：支持快速调整歌词偏移与重试同步。
fn lyrics_footer_controls(
    root: &Root,
    track: &sodam_core::models::TrackItem,
    cx: &mut Context<Root>,
) -> AnyElement {
    let offset = root.settings.lyrics_offset_ms;
    let offset_label = if offset == 0 {
        root.tr("歌词同步正常").to_string()
    } else {
        root.localized("偏移 {} ms", &[offset.to_string()])
    };
    let retry_track = track.clone();

    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .justify_between()
        .gap(px(theme::space::SM))
        .pt(px(theme::space::XS))
        .child(
            div()
                .text_size(theme::Text::Tiny.size())
                .text_color(theme::text_faint())
                .child(offset_label),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(theme::space::XS))
                .child(
                    div()
                        .id("lyric-offset-minus")
                        .px(px(theme::space::SM))
                        .py(px(2.0))
                        .rounded(px(theme::radius::PILL))
                        .bg(theme::surface_elevated())
                        .text_size(theme::Text::Tiny.size())
                        .text_color(theme::text_muted())
                        .cursor_pointer()
                        .hover(|style| style.bg(theme::surface_hover()))
                        .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                            root.set_lyrics_offset_ms(root.settings.lyrics_offset_ms - 300, cx);
                        }))
                        .child("-0.3s"),
                )
                .child(
                    div()
                        .id("lyric-offset-reset")
                        .px(px(theme::space::SM))
                        .py(px(2.0))
                        .rounded(px(theme::radius::PILL))
                        .bg(theme::surface_elevated())
                        .text_size(theme::Text::Tiny.size())
                        .text_color(theme::text_muted())
                        .cursor_pointer()
                        .hover(|style| style.bg(theme::surface_hover()))
                        .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                            root.set_lyrics_offset_ms(0, cx);
                        }))
                        .child(root.tr("校准")),
                )
                .child(
                    div()
                        .id("lyric-offset-plus")
                        .px(px(theme::space::SM))
                        .py(px(2.0))
                        .rounded(px(theme::radius::PILL))
                        .bg(theme::surface_elevated())
                        .text_size(theme::Text::Tiny.size())
                        .text_color(theme::text_muted())
                        .cursor_pointer()
                        .hover(|style| style.bg(theme::surface_hover()))
                        .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                            root.set_lyrics_offset_ms(root.settings.lyrics_offset_ms + 300, cx);
                        }))
                        .child("+0.3s"),
                )
                .child(
                    div()
                        .id("lyric-reload-btn")
                        .px(px(theme::space::SM))
                        .py(px(2.0))
                        .rounded(px(theme::radius::PILL))
                        .bg(theme::surface_elevated())
                        .text_size(theme::Text::Tiny.size())
                        .text_color(theme::text_muted())
                        .cursor_pointer()
                        .hover(|style| style.bg(theme::surface_hover()))
                        .on_click(cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                            root.load_lyrics(retry_track.clone(), true, cx);
                        }))
                        .child(root.tr("刷新歌词")),
                ),
        )
        .into_any_element()
}

/// 播放页内嵌队列视图（当 `PlaybackViewMode::QueueOnly` 时在播放页展示完整队列）。
fn inline_queue_panel(root: &Root, cx: &mut Context<Root>) -> AnyElement {
    let tracks = root.queue_cache.1.clone();
    let current_index = root.queue.index();
    let total = tracks.len();
    let mode_label = match root.queue.mode {
        sodam_core::queue::PlayMode::Sequential => root.tr("顺序播放"),
        sodam_core::queue::PlayMode::RepeatOne => root.tr("单曲循环"),
        sodam_core::queue::PlayMode::Shuffle => root.tr("随机播放"),
    };

    let rows: Vec<AnyElement> = tracks
        .iter()
        .enumerate()
        .take(80)
        .map(|(idx, item)| {
            let active = idx == current_index;
            let cover_path = root.cover_of(&item.cover);
            let title = item.title.clone();
            let artist = item.artist.clone();
            let duration = item.duration_label();
            div()
                .id(("inline-queue-row", idx))
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .gap(px(theme::space::MD))
                .h(px(48.0))
                .px(px(theme::space::MD))
                .rounded(px(theme::radius::ROW))
                .cursor_pointer()
                .when(active, |this| {
                    this.bg(theme::accent_soft())
                        .border_l_2()
                        .border_color(theme::accent())
                })
                .when(!active, |this| {
                    this.hover(|style| style.bg(theme::surface_hover()))
                })
                .on_click(cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                    root.jump_in_queue(idx, cx);
                }))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(theme::space::MD))
                        .min_w(px(0.0))
                        .flex_1()
                        .child(crate::ui::artwork::cover(
                            cover_path,
                            34.0,
                            theme::radius::ART,
                        ))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .min_w(px(0.0))
                                .flex_1()
                                .child(
                                    div()
                                        .truncate()
                                        .text_size(theme::Text::Body.size())
                                        .font_weight(if active {
                                            gpui::FontWeight::BOLD
                                        } else {
                                            gpui::FontWeight::NORMAL
                                        })
                                        .text_color(if active {
                                            theme::accent()
                                        } else {
                                            theme::text()
                                        })
                                        .child(title),
                                )
                                .child(
                                    div()
                                        .truncate()
                                        .text_size(theme::Text::Tiny.size())
                                        .text_color(theme::text_muted())
                                        .child(artist),
                                ),
                        ),
                )
                .child(
                    div()
                        .flex_none()
                        .text_size(theme::Text::Tiny.size())
                        .text_color(theme::text_faint())
                        .child(duration),
                )
                .into_any_element()
        })
        .collect();

    div()
        .flex()
        .flex_col()
        .size_full()
        .min_h(px(0.0))
        .gap(px(theme::space::MD))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(theme::Text::Large.size())
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(theme::text())
                        .child(root.localized("播放队列（{} 首）", &[total.to_string()])),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(theme::space::SM))
                        .child(
                            div()
                                .id("inline-queue-mode")
                                .px(px(theme::space::MD))
                                .py(px(theme::space::XS))
                                .rounded(px(theme::radius::PILL))
                                .bg(theme::surface_elevated())
                                .text_size(theme::Text::Small.size())
                                .text_color(theme::text_muted())
                                .cursor_pointer()
                                .hover(|style| style.bg(theme::surface_hover()))
                                .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                    root.queue.mode = root.queue.mode.next();
                                    cx.notify();
                                }))
                                .child(mode_label.to_string()),
                        )
                        .child(
                            div()
                                .id("inline-queue-save")
                                .px(px(theme::space::MD))
                                .py(px(theme::space::XS))
                                .rounded(px(theme::radius::PILL))
                                .bg(theme::surface_elevated())
                                .text_size(theme::Text::Small.size())
                                .text_color(theme::text_muted())
                                .cursor_pointer()
                                .hover(|style| style.bg(theme::surface_hover()))
                                .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                                    root.e3_save_queue_snapshot_auto(cx);
                                }))
                                .child(root.tr("存为列表")),
                        ),
                ),
        )
        .child(
            div()
                .id("inline-queue-scroll")
                .flex()
                .flex_col()
                .flex_1()
                .min_h(px(0.0))
                .overflow_y_scroll()
                .gap(px(2.0))
                .children(rows),
        )
        .into_any_element()
}

/// 专辑封面容器：保持稳定 1:1 比例，带中式画框细描边与切歌缩放淡入动效。
fn framed_cover(track_id: &str, cover_path: Option<std::path::PathBuf>, size: f32) -> AnyElement {
    let inner = div()
        .p(px(4.0))
        .rounded(px(theme::radius::CARD + 4.0))
        .bg(theme::surface_elevated())
        .border_1()
        .border_color(theme::border())
        .shadow_lg()
        .child(crate::ui::artwork::cover_loading(
            cover_path,
            size,
            theme::radius::CARD,
        ));

    if theme::motion_enabled() {
        inner
            .with_animation(
                gpui::ElementId::Name(format!("cover-stage-{track_id}").into()),
                Animation::new(Duration::from_millis(260)).with_easing(gpui::ease_in_out),
                |this, delta| {
                    this.opacity(0.4 + delta * 0.6)
                        .layer_scale(0.95 + delta * 0.05)
                },
            )
            .into_any_element()
    } else {
        inner.into_any_element()
    }
}

/// 渲染标准/半屏播放页：支持双栏视图、纯享歌词视图、内联队列视图，以及窄窗纵向自适应堆叠。
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
    let is_preview = is_preview_stream(&snapshot.quality);
    let playback_err = root
        .playback_error
        .clone()
        .or_else(|| (!snapshot.error.trim().is_empty()).then(|| snapshot.error.clone()));

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

    let more_button = div()
        .id("playback-more-menu")
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .size(px(theme::size::CONTROL_SM))
        .rounded(px(theme::radius::PILL))
        .cursor_pointer()
        .hover(|style| style.bg(theme::surface_hover()))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener({
                let track = track.clone();
                move |root, event: &MouseDownEvent, _window, cx| {
                    cx.stop_propagation();
                    root.track_menu = Some(crate::app::TrackMenu {
                        track: track.clone(),
                        position: event.position,
                    });
                    cx.notify();
                }
            }),
        )
        .child(
            svg()
                .path(icons::path("ellipsis"))
                .size(px(theme::ICON_SM))
                .text_color(theme::text_muted()),
        );

    let viewport_width = f32::from(window.viewport_size().width);
    let sidebar_w = if viewport_width < theme::breakpoints::NARROW_W {
        0.0
    } else if root.window_mode == theme::WindowMode::HalfScreen
        || viewport_width < theme::breakpoints::COMPACT_W
    {
        theme::SIDEBAR_COMPACT_W
    } else {
        theme::SIDEBAR_W
    };
    let content_viewport_width = if root.queue_open {
        (viewport_width - sidebar_w - crate::ui::player_bar::QUEUE_DRAWER_W).max(320.0)
    } else {
        (viewport_width - sidebar_w).max(320.0)
    };
    let viewport_height = (f32::from(window.viewport_size().height) - theme::PLAYER_H).max(320.0);
    let font_boost = window_lyrics_boost(viewport_width, viewport_height);

    let is_stacked = content_viewport_width < 700.0;
    let horizontal_pad = if content_viewport_width < 640.0 {
        16.0
    } else if content_viewport_width < 960.0 {
        28.0
    } else {
        48.0
    };

    let mut cover_size = if is_stacked {
        (viewport_height * 0.28)
            .min(content_viewport_width * 0.46)
            .clamp(140.0, 240.0)
    } else if root.queue_open {
        (viewport_height * 0.36)
            .min(content_viewport_width * 0.32)
            .clamp(170.0, 340.0)
    } else if root.window_mode == theme::WindowMode::HalfScreen {
        (viewport_height * 0.44)
            .min(content_viewport_width * 0.38)
            .clamp(200.0, 360.0)
    } else {
        (viewport_height * 0.42)
            .min(content_viewport_width * 0.34)
            .clamp(200.0, 380.0)
    };

    let mut lyrics_width = if is_stacked {
        (content_viewport_width - horizontal_pad * 2.0).max(240.0)
    } else if root.playback_view_mode == theme::PlaybackViewMode::LyricsOnly {
        (content_viewport_width * 0.76).clamp(320.0, 880.0)
    } else {
        (content_viewport_width * 0.50).clamp(280.0, 740.0)
    };

    if !is_stacked
        && !root.queue_open
        && root.playback_view_mode != theme::PlaybackViewMode::LyricsOnly
    {
        let usable = (content_viewport_width - horizontal_pad * 2.0).max(320.0);
        let total = cover_size + 40.0 + lyrics_width;
        if total > usable {
            let scale = usable / total;
            cover_size = (cover_size * scale).max(150.0);
            lyrics_width = (lyrics_width * scale).max(240.0);
        }
    }

    let list = build_lyrics_list(root, &snapshot, font_boost);

    let track_info = div()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(theme::space::XS))
        .w(px(cover_size + 32.0))
        .max_w_full()
        .child(
            div()
                .flex()
                .flex_row()
                .flex_wrap()
                .items_center()
                .justify_center()
                .gap(px(theme::space::XS))
                .max_w_full()
                .min_w(px(0.0))
                .child(
                    div()
                        .min_w(px(0.0))
                        .truncate()
                        .text_size(px(18.0 + (font_boost * 0.5).min(4.0)))
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(theme::text())
                        .child(track.title.clone()),
                )
                .when(track.vip, |this| this.child(vip_badge()))
                .when(is_preview, |this| this.child(preview_badge(root))),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_center()
                .gap(px(theme::space::SM))
                .max_w_full()
                .child(
                    div()
                        .id("playback-artist-link")
                        .max_w(px(cover_size * 0.55))
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
                )
                .when(!track.album.trim().is_empty(), |this| {
                    this.child(
                        div()
                            .text_size(theme::Text::Tiny.size())
                            .text_color(theme::text_faint())
                            .child("·"),
                    )
                    .child(
                        div()
                            .id("playback-album-link")
                            .max_w(px(cover_size * 0.45))
                            .truncate()
                            .cursor_pointer()
                            .text_size(theme::Text::Small.size())
                            .text_color(theme::text_faint())
                            .hover(|style| style.text_color(theme::accent()))
                            .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                                cx.stop_propagation();
                            })
                            .on_click(cx.listener({
                                let track = track.clone();
                                move |root, _event: &ClickEvent, _window, cx| {
                                    root.open_track_album(track.clone(), cx);
                                }
                            }))
                            .child(track.album.clone()),
                    )
                }),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_center()
                .gap(px(theme::space::SM))
                .pt(px(2.0))
                .child(like_button)
                .child(download_button)
                .child(more_button),
        );

    let two_line_container_w = (content_viewport_width - horizontal_pad * 2.0)
        .max(cover_size + 40.0)
        .min(860.0);

    let compact_lyrics = if root.queue_open {
        let active_index = active.unwrap_or(0);
        let position_seconds =
            snapshot.position_seconds - root.settings.lyrics_offset_ms as f64 / 1000.0;
        let base_font =
            (root.settings.lyrics_font_size.clamp(14, 30) as f32 + font_boost).clamp(16.0, 32.0);
        let active_two_line_size = (base_font + 5.0 + font_boost * 0.45).clamp(20.0, 36.0);
        let next_two_line_size = (base_font * 0.88).clamp(15.0, 26.0);
        let font_family =
            theme::LyricsFontFamily::from_key(&root.settings.lyrics_font_family).font_name();

        let current_element = if let Some(line) = root
            .lyrics
            .get(active_index)
            .filter(|l| !l.text.trim().is_empty())
        {
            render_lyric_line_content(
                line,
                true,
                position_seconds,
                theme::accent(),
                theme::accent(),
                theme::LyricsAlign::Center,
                active_two_line_size,
            )
        } else {
            div()
                .w_full()
                .text_center()
                .truncate()
                .text_color(theme::accent())
                .child(track.title.clone())
                .into_any_element()
        };
        let next = root
            .lyrics
            .get(active_index.saturating_add(1))
            .map(|line| line.text.clone())
            .unwrap_or_default();
        Some(
            div()
                .w(px(two_line_container_w))
                .max_w_full()
                .min_h(px(active_two_line_size + next_two_line_size + 28.0))
                .mt(px(theme::space::MD))
                .px(px(theme::space::MD))
                .py(px(theme::space::SM))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px((font_boost * 0.8 + 6.0).clamp(6.0, 14.0)))
                .font_family(font_family)
                .child(
                    div()
                        .w_full()
                        .flex()
                        .justify_center()
                        .text_size(px(active_two_line_size))
                        .line_height(px(active_two_line_size * 1.38))
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(theme::accent())
                        .child(current_element),
                )
                .when(!next.is_empty(), |this| {
                    this.child(
                        div()
                            .w_full()
                            .text_center()
                            .truncate()
                            .text_size(px(next_two_line_size))
                            .line_height(px(next_two_line_size * 1.35))
                            .text_color(theme::text_muted())
                            .child(next),
                    )
                })
                .into_any_element(),
        )
    } else {
        None
    };

    let lyrics_or_queue_panel = if root.queue_open {
        None
    } else if root.playback_view_mode == theme::PlaybackViewMode::QueueOnly {
        Some(
            div()
                .flex()
                .flex_col()
                .w(px(lyrics_width))
                .max_w(px(740.0))
                .min_w(px(240.0))
                .h_full()
                .flex_1()
                .min_h(px(0.0))
                .child(inline_queue_panel(root, cx))
                .into_any_element(),
        )
    } else {
        Some(
            div()
                .flex()
                .flex_col()
                .w(px(lyrics_width))
                .max_w(px(880.0))
                .min_w(px(240.0))
                .h_full()
                .flex_1()
                .min_h(px(0.0))
                .gap(px(theme::space::SM))
                .when(!is_stacked, |this| {
                    this.child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(theme::space::SM))
                            .child(
                                div()
                                    .text_size(px(20.0 + (font_boost * 0.6).min(6.0)))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(theme::text())
                                    .truncate()
                                    .child(track.title.clone()),
                            )
                            .when(is_preview, |header| header.child(preview_badge(root))),
                    )
                    .child(
                        div()
                            .text_size(theme::Text::Small.size())
                            .text_color(theme::text_muted())
                            .truncate()
                            .child(if track.album.trim().is_empty() {
                                format!("{} · {}", track.artist, snapshot.progress_label())
                            } else {
                                format!(
                                    "{} · {} · {}",
                                    track.artist,
                                    track.album,
                                    snapshot.progress_label()
                                )
                            }),
                    )
                })
                .child(if root.loading_lyrics {
                    loading_state(root.tr("正在读取歌词…")).into_any_element()
                } else if let Some(error) = &root.lyrics_error {
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .items_center()
                        .justify_center()
                        .gap(px(theme::space::MD))
                        .child(empty_state("captions", root.tr("歌词加载或同步失败")))
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
                        .flex_1()
                        .items_center()
                        .justify_center()
                        .gap(px(theme::space::SM))
                        .child(empty_state("music", root.tr("这首歌暂时没有歌词")))
                        .child(retry_button(root.language, cx))
                        .into_any_element()
                } else {
                    list.expect("loaded lyrics have a list")
                })
                .when(!root.lyrics.is_empty(), |this| {
                    this.child(lyrics_footer_controls(root, &track, cx))
                })
                .into_any_element(),
        )
    };

    let show_cover_column =
        root.playback_view_mode != theme::PlaybackViewMode::LyricsOnly || root.queue_open;
    let cover_column_w = if root.queue_open {
        two_line_container_w
    } else {
        cover_size + 16.0
    };

    let body_content = if is_stacked {
        div()
            .flex()
            .flex_col()
            .items_center()
            .flex_1()
            .min_h(px(0.0))
            .w_full()
            .gap(px(theme::space::MD))
            .when(show_cover_column, |this| {
                this.child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .w_full()
                        .gap(px(theme::space::SM))
                        .child(framed_cover(&track.id, cover_path, cover_size))
                        .child(track_info)
                        .when_some(compact_lyrics, |col, lyrics| col.child(lyrics)),
                )
            })
            .when_some(lyrics_or_queue_panel, |this, panel| this.child(panel))
            .into_any_element()
    } else {
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_center()
            .flex_1()
            .min_h(px(0.0))
            .w_full()
            .gap(px(36.0))
            .when(show_cover_column, |this| {
                this.child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .h_full()
                        .gap(px(theme::space::MD))
                        .w(px(cover_column_w))
                        .max_w_full()
                        .flex_none()
                        .child(framed_cover(&track.id, cover_path, cover_size))
                        .when(!original_cover_ready, |col| {
                            col.child(retry_button(root.language, cx))
                        })
                        .child(track_info)
                        .when_some(compact_lyrics, |col, lyrics| col.child(lyrics)),
                )
            })
            .when_some(lyrics_or_queue_panel, |this, panel| this.child(panel))
            .into_any_element()
    };

    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_h(px(0.0))
        .px(px(horizontal_pad))
        .pt(px(theme::space::MD))
        .pb(px(theme::space::MD))
        .gap(px(theme::space::SM))
        .when_some(playback_err, |this, err| {
            this.child(playback_error_banner(root, &err, cx))
        })
        .child(body_content)
        .into_any_element()
}

/// 全屏沉浸模式视图：隐藏侧边栏与系统标题栏，封面、歌词与封面衍生水墨渐变成为视觉中心。
///
/// 支持鼠标拖动窗口，支持鼠标移动或点击唤出浮动控制栏，支持 Esc 退出。
pub(crate) fn fullscreen_immersive_view(
    root: &Root,
    window: &Window,
    cx: &mut Context<Root>,
) -> AnyElement {
    let snapshot = root.engine.snapshot();
    let Some(track) = root
        .pending_track
        .clone()
        .or_else(|| root.queue.current().cloned())
    else {
        return div()
            .id("fullscreen-empty")
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .size_full()
            .gap(px(theme::space::LG))
            .child(empty_state("music", root.tr("当前没有正在播放的歌曲")))
            .child(
                div()
                    .id("fullscreen-exit-empty")
                    .px(px(theme::space::LG))
                    .py(px(theme::space::SM))
                    .rounded(px(theme::radius::PILL))
                    .bg(theme::accent())
                    .text_color(theme::accent_foreground())
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                        cx.stop_propagation();
                    })
                    .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.set_window_mode(theme::WindowMode::Normal, cx);
                    }))
                    .child(root.tr("返回普通模式 (Esc)")),
            )
            .into_any_element();
    };

    let controls_visible = root.fullscreen_controls_visible || root.queue_open;
    let is_preview = is_preview_stream(&snapshot.quality);
    let cover_path = root
        .original_covers
        .get(&track.cover)
        .cloned()
        .or_else(|| root.cover_of(&track.cover));

    let vw = f32::from(window.viewport_size().width);
    let vh = f32::from(window.viewport_size().height);
    let is_portrait = vw < 760.0;
    let cover_size = if is_portrait {
        (vh * 0.30).min(vw * 0.55).clamp(160.0, 280.0)
    } else {
        (vh * 0.48).min(vw * 0.34).clamp(240.0, 440.0)
    };
    let font_boost = window_lyrics_boost(vw, vh) + 3.0;
    let lyrics_list = build_lyrics_list(root, &snapshot, font_boost);

    let top_bar = div()
        .id("fs-top-bar")
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .w_full()
        .px(px(theme::space::XXL))
        .py(px(theme::space::MD))
        .window_control_area(gpui::WindowControlArea::Drag)
        .when(cfg!(target_os = "linux"), |this| {
            this.on_mouse_down(MouseButton::Left, |_event: &MouseDownEvent, window, _cx| {
                window.start_window_move();
            })
        })
        .child(
            div()
                .id("fs-exit-btn")
                .flex()
                .flex_row()
                .items_center()
                .gap(px(theme::space::XS))
                .px(px(theme::space::MD))
                .py(px(6.0))
                .rounded(px(theme::radius::PILL))
                .bg(theme::surface_elevated())
                .border_1()
                .border_color(theme::border())
                .cursor_pointer()
                .hover(|style| style.bg(theme::surface_hover()))
                .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                    cx.stop_propagation();
                })
                .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                    cx.stop_propagation();
                    root.set_window_mode(theme::WindowMode::Normal, cx);
                }))
                .child(
                    svg()
                        .path(icons::path("chevron-left"))
                        .size(px(theme::ICON_SM))
                        .text_color(theme::text()),
                )
                .child(
                    div()
                        .text_size(theme::Text::Small.size())
                        .text_color(theme::text())
                        .child(root.tr("退出沉浸 (Esc)")),
                ),
        )
        .child(div().flex_1());

    let center_stage = if root.playback_view_mode == theme::PlaybackViewMode::LyricsOnly {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .flex_1()
            .min_h(px(0.0))
            .w_full()
            .max_w(px(960.0))
            .mx_auto()
            .px(px(theme::space::XXL))
            .gap(px(theme::space::MD))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(theme::space::MD))
                    .child(
                        div()
                            .text_size(px(28.0 + (font_boost * 0.4).min(6.0)))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(theme::text())
                            .child(track.title.clone()),
                    )
                    .when(is_preview, |this| this.child(preview_badge(root))),
            )
            .child(
                div()
                    .text_size(theme::Text::Body.size())
                    .text_color(theme::text_muted())
                    .child(format!("{} · {}", track.artist, track.album)),
            )
            .child(lyrics_list.unwrap_or_else(|| {
                empty_state("music", root.tr("这首歌暂时没有歌词")).into_any_element()
            }))
            .into_any_element()
    } else if is_portrait {
        div()
            .flex()
            .flex_col()
            .items_center()
            .flex_1()
            .min_h(px(0.0))
            .w_full()
            .px(px(theme::space::XL))
            .gap(px(theme::space::LG))
            .child(framed_cover(&track.id, cover_path, cover_size))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(theme::space::SM))
                    .child(
                        div()
                            .text_size(theme::Text::Title.size())
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(theme::text())
                            .child(track.title.clone()),
                    )
                    .when(is_preview, |this| this.child(preview_badge(root))),
            )
            .child(lyrics_list.unwrap_or_else(|| {
                empty_state("music", root.tr("这首歌暂时没有歌词")).into_any_element()
            }))
            .into_any_element()
    } else {
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_center()
            .flex_1()
            .min_h(px(0.0))
            .w_full()
            .px(px(48.0))
            .gap(px(56.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(theme::space::MD))
                    .child(framed_cover(&track.id, cover_path, cover_size))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(theme::space::SM))
                            .child(
                                div()
                                    .text_size(px(24.0 + (font_boost * 0.4).min(5.0)))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(theme::text())
                                    .child(track.title.clone()),
                            )
                            .when(track.vip, |this| this.child(vip_badge()))
                            .when(is_preview, |this| this.child(preview_badge(root))),
                    )
                    .child(
                        div()
                            .text_size(theme::Text::Body.size())
                            .text_color(theme::text_muted())
                            .child(format!("{} · {}", track.artist, track.album)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .max_w(px(860.0))
                    .h_full()
                    .py(px(theme::space::XL))
                    .min_h(px(0.0))
                    .child(lyrics_list.unwrap_or_else(|| {
                        empty_state("music", root.tr("这首歌暂时没有歌词")).into_any_element()
                    })),
            )
            .into_any_element()
    };

    div()
        .id("fullscreen-immersive-stage")
        .flex()
        .flex_col()
        .size_full()
        .window_control_area(gpui::WindowControlArea::Drag)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|root, event: &MouseDownEvent, _window, _cx| {
                root.fullscreen_drag_origin = Some(event.position);
                root.fullscreen_dragged = false;
            }),
        )
        .on_mouse_move(
            cx.listener(|root, event: &gpui::MouseMoveEvent, window, cx| {
                root.fullscreen_last_interaction = std::time::Instant::now();
                if !root.fullscreen_controls_visible {
                    root.fullscreen_controls_visible = true;
                    cx.notify();
                }
                if event.pressed_button == Some(MouseButton::Left) {
                    if let Some(origin) = root.fullscreen_drag_origin {
                        let dx = f32::from(event.position.x) - f32::from(origin.x);
                        let dy = f32::from(event.position.y) - f32::from(origin.y);
                        if dx.hypot(dy) > 5.0 {
                            root.fullscreen_drag_origin = None;
                            root.fullscreen_dragged = true;
                            window.start_window_move();
                        }
                    }
                }
            }),
        )
        .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
            if root.fullscreen_dragged {
                root.fullscreen_dragged = false;
                return;
            }
            root.fullscreen_last_interaction = std::time::Instant::now();
            root.fullscreen_controls_visible = !root.fullscreen_controls_visible;
            cx.notify();
        }))
        .when(controls_visible, |this| this.child(top_bar))
        .child(
            div()
                .flex()
                .flex_row()
                .flex_1()
                .min_h(px(0.0))
                .child(center_stage)
                .when(root.queue_open, |this| {
                    this.child(crate::ui::player_bar::queue_drawer(root, cx))
                }),
        )
        .when(controls_visible, |this| {
            this.child(
                div()
                    .w_full()
                    .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                        cx.stop_propagation();
                    })
                    .child(crate::ui::player_bar::render(root, cx)),
            )
        })
        .into_any_element()
}

/// 4. 紧凑小窗口模式视图：只显示封面、歌曲名、当前一句歌词、播放控制和进度，保持高 DPI 与窄窗口可用。
pub(crate) fn compact_window_view(root: &Root, cx: &mut Context<Root>) -> AnyElement {
    let snapshot = root.engine.snapshot();
    let current = root
        .pending_track
        .clone()
        .or_else(|| root.queue.current().cloned());
    let title = current
        .as_ref()
        .map(|t| t.title.clone())
        .unwrap_or_else(|| root.tr("未在播放").to_string());
    let artist = current
        .as_ref()
        .map(|t| t.artist.clone())
        .unwrap_or_else(|| root.tr("SodaM").to_string());
    let cover_path = current.as_ref().and_then(|t| root.cover_of(&t.cover));
    let liked = current
        .as_ref()
        .is_some_and(|t| root.liked_ids.contains(&t.id));
    let is_preview = is_preview_stream(&snapshot.quality);
    let progress = snapshot.progress_fraction().clamp(0.0, 1.0);
    let active_lyric = root
        .lyrics_active
        .and_then(|idx| root.lyrics.get(idx))
        .map(|l| l.text.clone())
        .unwrap_or_default();

    div()
        .id("compact-window-stage")
        .flex()
        .flex_col()
        .justify_between()
        .size_full()
        .p(px(theme::space::LG))
        .gap(px(theme::space::MD))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(theme::space::MD))
                .min_w(px(0.0))
                .child(crate::ui::artwork::cover(
                    cover_path,
                    72.0,
                    theme::radius::CARD,
                ))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .min_w(px(0.0))
                        .flex_1()
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(px(theme::space::XS))
                                .min_w(px(0.0))
                                .child(
                                    div()
                                        .truncate()
                                        .text_size(theme::Text::Large.size())
                                        .font_weight(gpui::FontWeight::BOLD)
                                        .text_color(theme::text())
                                        .child(title),
                                )
                                .when(is_preview, |this| this.child(preview_badge(root))),
                        )
                        .child(
                            div()
                                .truncate()
                                .text_size(theme::Text::Small.size())
                                .text_color(theme::text_muted())
                                .child(artist),
                        )
                        .when(!active_lyric.is_empty(), |this| {
                            this.child(
                                div()
                                    .truncate()
                                    .text_size(theme::Text::Small.size())
                                    .text_color(theme::accent())
                                    .child(active_lyric),
                            )
                        }),
                )
                .child(
                    div()
                        .id("compact-expand-normal")
                        .px(px(theme::space::MD))
                        .py(px(theme::space::XS))
                        .rounded(px(theme::radius::PILL))
                        .bg(theme::surface_elevated())
                        .border_1()
                        .border_color(theme::border())
                        .text_size(theme::Text::Tiny.size())
                        .text_color(theme::text_muted())
                        .cursor_pointer()
                        .hover(|style| style.bg(theme::surface_hover()))
                        .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                            root.set_window_mode(theme::WindowMode::Normal, cx);
                        }))
                        .child(root.tr("返回完整窗口")),
                ),
        )
        .child(
            // 进度条 + 核心传输控制
            div()
                .flex()
                .flex_col()
                .gap(px(theme::space::SM))
                .w_full()
                .child(
                    div()
                        .w_full()
                        .h(px(6.0))
                        .rounded(px(theme::radius::PILL))
                        .bg(theme::border())
                        .overflow_hidden()
                        .child(
                            div()
                                .w(gpui::relative(progress))
                                .h_full()
                                .rounded(px(theme::radius::PILL))
                                .bg(theme::accent()),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_size(theme::Text::Tiny.size())
                                .text_color(theme::text_muted())
                                .child(snapshot.progress_label()),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(px(theme::space::MD))
                                .child(
                                    crate::ui::artwork::icon_button(
                                        "compact-prev",
                                        "skip-back",
                                        theme::ICON_LG,
                                        theme::text(),
                                    )
                                    .on_click(cx.listener(
                                        |root, _event: &ClickEvent, _window, cx| {
                                            root.prev_track(cx);
                                        },
                                    )),
                                )
                                .child(
                                    div()
                                        .id("compact-play")
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .size(px(40.0))
                                        .rounded(px(theme::radius::PILL))
                                        .bg(theme::accent())
                                        .cursor_pointer()
                                        .on_click(cx.listener(
                                            |root, _event: &ClickEvent, _window, cx| {
                                                root.toggle_play(cx);
                                            },
                                        ))
                                        .child(
                                            svg()
                                                .path(icons::path(if snapshot.playing {
                                                    "pause"
                                                } else {
                                                    "play"
                                                }))
                                                .size(px(theme::ICON_LG))
                                                .text_color(theme::accent_foreground()),
                                        ),
                                )
                                .child(
                                    crate::ui::artwork::icon_button(
                                        "compact-next",
                                        "skip-forward",
                                        theme::ICON_LG,
                                        theme::text(),
                                    )
                                    .on_click(cx.listener(
                                        |root, _event: &ClickEvent, _window, cx| {
                                            root.next_track(cx);
                                        },
                                    )),
                                ),
                        )
                        .child(
                            crate::ui::artwork::icon_button(
                                "compact-like",
                                if liked { "heart-filled" } else { "heart" },
                                theme::ICON,
                                if liked {
                                    theme::accent()
                                } else {
                                    theme::text_muted()
                                },
                            )
                            .on_click(cx.listener(
                                move |root, _event: &ClickEvent, _window, cx| {
                                    if let Some(track) = current.clone() {
                                        root.toggle_like(track, cx);
                                    }
                                },
                            )),
                        ),
                ),
        )
        .into_any_element()
}
