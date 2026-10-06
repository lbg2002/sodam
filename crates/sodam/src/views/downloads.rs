//! 下载管理页：展示 SodaM 已导出的本地歌曲，并支持过滤、打开目录与删除。

use super::*;

fn download_search_box(root: &Root, window: &Window, cx: &mut Context<Root>) -> impl IntoElement {
    let focused = root.search_focus.is_focused(window);
    let input_focus = root.search_focus.clone();
    let input_entity = cx.entity();
    let text = if root.search_input.is_empty() {
        root.tr("搜索已下载歌曲 / 歌手").to_string()
    } else {
        format!("{}{}", root.search_input, if focused { "▌" } else { "" })
    };
    let color = if root.search_input.is_empty() {
        theme::text_muted()
    } else {
        theme::text()
    };

    div()
        .id("download-search-input")
        .relative()
        .track_focus(&root.search_focus)
        .flex()
        .flex_row()
        .items_center()
        .h(px(38.0))
        .flex_1()
        .px_3()
        .rounded_md()
        .bg(theme::surface())
        .border_1()
        .border_color(if focused {
            theme::accent()
        } else {
            theme::border()
        })
        .text_color(color)
        .cursor_text()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|root, _event, window, cx| {
                window.focus(&root.search_focus, cx);
            }),
        )
        .on_key_down(cx.listener(|root, event: &KeyDownEvent, _window, cx| {
            match event.keystroke.key.as_str() {
                "backspace" => {
                    let _ = root.search_input.pop();
                    cx.notify();
                }
                "escape" => {
                    root.search_input.clear();
                    cx.notify();
                }
                _ => {}
            }
        }))
        .child(
            canvas(
                |_bounds, _window, _cx| {},
                move |bounds, _state, window, cx| {
                    window.handle_input(
                        &input_focus,
                        gpui::ElementInputHandler::new(bounds, input_entity.clone()),
                        cx,
                    );
                },
            )
            .absolute()
            .size_full(),
        )
        .child(text)
}

fn bytes_label(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * KB;
    const GB: f64 = 1024.0 * MB;
    let value = bytes as f64;
    if value >= GB {
        format!("{:.2} GB", value / GB)
    } else if value >= MB {
        format!("{:.1} MB", value / MB)
    } else if value >= KB {
        format!("{:.0} KB", value / KB)
    } else {
        format!("{bytes} B")
    }
}

pub fn downloads_view(root: &Root, window: &Window, cx: &mut Context<Root>) -> AnyElement {
    let query = root.search_input.trim().to_lowercase();
    let pending_items: Vec<_> = root
        .pending_downloads
        .values()
        .filter(|track| {
            query.is_empty()
                || track.title.to_lowercase().contains(&query)
                || track.artist.to_lowercase().contains(&query)
                || track.album.to_lowercase().contains(&query)
        })
        .cloned()
        .collect();
    let items: Vec<_> = root
        .downloads
        .iter()
        .filter(|item| {
            query.is_empty()
                || item.title.to_lowercase().contains(&query)
                || item.artist.to_lowercase().contains(&query)
                || item.album.to_lowercase().contains(&query)
        })
        .cloned()
        .collect();

    let toolbar = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(theme::space::MD))
        .child(download_search_box(root, window, cx))
        .child(
            div()
                .id("open-download-folder")
                .flex()
                .flex_row()
                .items_center()
                .gap(px(theme::space::SM))
                .h(px(38.0))
                .px(px(theme::space::MD))
                .rounded(px(theme::radius::ROW))
                .bg(theme::surface())
                .border_1()
                .border_color(theme::border())
                .cursor_pointer()
                .hover(|style| style.bg(theme::surface_hover()))
                .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                    root.open_download_folder(cx);
                }))
                .child(
                    svg()
                        .path(icons::path("folder-open"))
                        .size(px(theme::ICON_SM))
                        .text_color(theme::text_muted()),
                )
                .child(root.tr("打开下载目录")),
        );

    if root.downloads_loading && root.downloads.is_empty() {
        return div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h(px(0.0))
            .gap(px(theme::space::MD))
            .child(toolbar)
            .child(loading_state(root.tr("正在读取下载列表…")))
            .into_any_element();
    }

    let pending_section = div()
        .flex()
        .flex_col()
        .gap(px(theme::space::XS))
        .when(!pending_items.is_empty(), |this| {
            this.child(
                div()
                    .px(px(theme::space::SM))
                    .text_size(theme::Text::Small.size())
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(theme::text_muted())
                    .child(root.tr("待下载")),
            )
            .children(pending_items.into_iter().map(|track| {
                let cancel_track = track.clone();
                div()
                    .id(gpui::ElementId::Name(
                        format!("pending-download-{}", track.id).into(),
                    ))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(theme::space::MD))
                    .h(px(58.0))
                    .px(px(theme::space::MD))
                    .rounded(px(theme::radius::ROW))
                    .bg(theme::surface())
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w(px(0.0))
                            .child(
                                div()
                                    .truncate()
                                    .text_size(theme::Text::Body.size())
                                    .text_color(theme::text())
                                    .child(track.title),
                            )
                            .child(
                                div()
                                    .truncate()
                                    .text_size(theme::Text::Small.size())
                                    .text_color(theme::text_muted())
                                    .child(track.artist),
                            ),
                    )
                    .child(
                        div()
                            .text_size(theme::Text::Tiny.size())
                            .text_color(theme::accent())
                            .child(root.tr("等待播放缓存")),
                    )
                    .child(
                        div()
                            .id(gpui::ElementId::Name(
                                format!("cancel-download-{}", track.id).into(),
                            ))
                            .flex()
                            .items_center()
                            .justify_center()
                            .h(px(theme::size::CONTROL_SM))
                            .px(px(theme::space::MD))
                            .rounded(px(theme::radius::PILL))
                            .cursor_pointer()
                            .hover(|style| style.bg(theme::surface_hover()))
                            .on_click(cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                                root.toggle_download(cancel_track.clone(), cx);
                            }))
                            .child(root.tr("取消")),
                    )
            }))
        })
        .into_any_element();

    let body = if items.is_empty() {
        empty_state(
            "download",
            if root.downloads.is_empty() {
                root.tr("还没有下载的歌曲")
            } else {
                root.tr("没有找到匹配的下载")
            },
        )
    } else {
        div()
            .id("downloads-list")
            .flex()
            .flex_col()
            .flex_1()
            .min_h(px(0.0))
            .gap(px(theme::space::XS))
            .overflow_y_scroll()
            .children(items.into_iter().map(|item| {
                let id = item.track_id.clone();
                let title = item.title.clone();
                let cover_path = root.cover_of(&item.cover);
                if cover_path.is_none() && !item.cover.is_empty() {
                    if let Ok(mut queue) = root.cover_requests.lock() {
                        if !queue.iter().any(|url| url == &item.cover) {
                            queue.push(item.cover.clone());
                        }
                    }
                }
                div()
                    .id(gpui::ElementId::Name(
                        format!("download-{}", item.track_id).into(),
                    ))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(theme::space::MD))
                    .h(px(68.0))
                    .px(px(theme::space::MD))
                    .rounded(px(theme::radius::ROW))
                    .hover(|style| style.bg(theme::surface_hover()))
                    .child(cover(cover_path, 48.0, theme::radius::ART))
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
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme::text())
                                    .child(item.title),
                            )
                            .child(
                                div()
                                    .truncate()
                                    .text_size(theme::Text::Small.size())
                                    .text_color(theme::text_muted())
                                    .child(if item.album.is_empty() {
                                        item.artist
                                    } else {
                                        format!("{} · {}", item.artist, item.album)
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .w(px(160.0))
                            .flex_none()
                            .text_right()
                            .text_size(theme::Text::Small.size())
                            .text_color(theme::text_faint())
                            .child(format!("{} · {}", item.quality, bytes_label(item.bytes))),
                    )
                    .child(
                        div()
                            .id(gpui::ElementId::Name(
                                format!("delete-download-{id}").into(),
                            ))
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(theme::size::CONTROL_SM))
                            .rounded(px(theme::radius::PILL))
                            .cursor_pointer()
                            .hover(|style| style.bg(theme::surface_hover()))
                            .on_click(cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                                root.delete_download(id.clone(), title.clone(), cx);
                            }))
                            .child(
                                svg()
                                    .path(icons::path("trash-2"))
                                    .size(px(theme::ICON_SM))
                                    .text_color(theme::text_muted()),
                            ),
                    )
                    .into_any_element()
            }))
            .into_any_element()
    };

    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_h(px(0.0))
        .gap(px(theme::space::MD))
        .child(toolbar)
        .child(pending_section)
        .child(body)
        .into_any_element()
}
