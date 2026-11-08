//! 听歌模式页。

use super::*;

/// 首页推荐：涵盖推荐横幅、每日推荐、个性化内容（听歌模式）、推荐歌单与最近播放，
/// 适配普通窗口、半屏窗口与全屏窗口。
pub(crate) fn home_view(root: &Root, cx: &mut Context<Root>) -> AnyElement {
    let cards = root.scene_cards.clone();
    let scenes = root.scenes.clone();
    let recent = root.recent.clone();
    let daily_tracks: Vec<sodam_core::models::TrackItem> =
        root.queue.tracks().iter().take(6).cloned().collect();

    if cards.is_empty() && scenes.is_empty() && daily_tracks.is_empty() {
        return div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(theme::space::MD))
            .child(if root.loading_scenes || root.loading_recommendation {
                loading_state(root.tr("正在准备推荐内容…")).into_any_element()
            } else {
                empty_state("music", root.tr("暂无推荐内容，可点击下方按钮加载")).into_any_element()
            })
            .child(
                div()
                    .id("home-empty-reload")
                    .px(px(theme::space::LG))
                    .py(px(theme::space::SM))
                    .rounded(px(theme::radius::PILL))
                    .bg(theme::accent())
                    .text_color(theme::accent_foreground())
                    .cursor_pointer()
                    .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
                        root.load_scenes(cx);
                        root.start_recommendation(false, cx);
                    }))
                    .child(root.tr("加载今日推荐")),
            )
            .into_any_element();
    }

    let featured = cards.first().cloned();
    let featured_cover = featured
        .as_ref()
        .and_then(|c| root.cover_of(&c.playlist.cover))
        .or_else(|| daily_tracks.first().and_then(|t| root.cover_of(&t.cover)));
    let featured_title = featured
        .as_ref()
        .map(|c| c.playlist.title.clone())
        .or_else(|| daily_tracks.first().map(|t| t.title.clone()))
        .unwrap_or_else(|| root.tr("今日私人推荐").to_string());
    let featured_count = featured
        .as_ref()
        .map(|c| c.playlist.track_count.to_string())
        .unwrap_or_else(|| root.queue.len().max(12).to_string());

    let card_cover_size = (root.content_density.card_size() - 14.0).clamp(116.0, 162.0);
    let card_box_w = card_cover_size + 16.0;

    // 1. 推荐歌单卡片
    let card_items = cards
        .iter()
        .take(10)
        .map(|card| {
            let cover_path = root.cover_of(&card.playlist.cover);
            let title = card.playlist.title.clone();
            let count = card.playlist.track_count.to_string();
            let click_card = card.clone();
            div()
                .id(gpui::ElementId::Name(
                    format!("home-recommend-{}", card.inner_block_id).into(),
                ))
                .flex()
                .flex_col()
                .gap(px(theme::space::SM))
                .w(px(card_box_w))
                .p(px(theme::space::SM))
                .rounded(px(theme::radius::CARD))
                .bg(theme::surface_elevated())
                .border_1()
                .border_color(theme::border())
                .cursor_pointer()
                .hover(|style| style.bg(theme::surface_hover()).layer_scale(1.018))
                .on_click(cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                    root.start_scene_card(click_card.clone(), cx);
                }))
                .child(cover(cover_path, card_cover_size, theme::radius::CARD))
                .child(
                    div()
                        .w_full()
                        .truncate()
                        .text_size(theme::Text::Body.size())
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(theme::text())
                        .child(title),
                )
                .child(
                    div()
                        .text_size(theme::Text::Small.size())
                        .text_color(theme::text_muted())
                        .child(root.localized("{} 首", &[count])),
                )
                .into_any_element()
        })
        .collect::<Vec<_>>();

    // 2. 每日推荐单曲卡片
    let daily_items = daily_tracks
        .iter()
        .enumerate()
        .map(|(idx, track)| {
            let cover_path = root.cover_of(&track.cover);
            let title = track.title.clone();
            let artist = track.artist.clone();
            let album = track.album.clone();
            div()
                .id(("home-daily-track", idx))
                .flex()
                .flex_row()
                .items_center()
                .gap(px(theme::space::MD))
                .w(px(270.0))
                .p(px(theme::space::SM))
                .rounded(px(theme::radius::CARD))
                .bg(theme::surface_elevated())
                .border_1()
                .border_color(theme::border())
                .cursor_pointer()
                .hover(|style| style.bg(theme::surface_hover()).layer_scale(1.012))
                .on_click(cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                    root.jump_in_queue(idx, cx);
                    root.open_playback(cx);
                }))
                .child(cover(cover_path, 48.0, theme::radius::ART))
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
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme::text())
                                .child(title),
                        )
                        .child(
                            div()
                                .truncate()
                                .text_size(theme::Text::Small.size())
                                .text_color(theme::text_muted())
                                .child(if album.is_empty() {
                                    artist
                                } else {
                                    format!("{artist} · {album}")
                                }),
                        ),
                )
                .into_any_element()
        })
        .collect::<Vec<_>>();

    // 3. 个性化内容（听歌模式胶囊）
    let personalized_items = scenes
        .iter()
        .take(8)
        .enumerate()
        .map(|(idx, scene)| {
            let label = root.scene_display_text(&scene.sub_queue_type, &scene.text);
            let click_scene = scene.clone();
            let sub_type = scene.sub_queue_type.clone();
            div()
                .id(("home-scene-pill", idx))
                .flex()
                .flex_row()
                .items_center()
                .gap(px(theme::space::SM))
                .px(px(theme::space::MD))
                .py(px(theme::space::SM))
                .rounded(px(theme::radius::CARD))
                .bg(theme::surface_elevated())
                .border_1()
                .border_color(theme::border())
                .cursor_pointer()
                .hover(|style| style.bg(theme::surface_hover()).layer_scale(1.02))
                .on_click(cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                    root.start_scene_queue(click_scene.clone(), cx);
                }))
                .child(crate::ui::artwork::scene_image(&sub_type, 24.0))
                .child(
                    div()
                        .text_size(theme::Text::Body.size())
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(theme::text())
                        .child(label),
                )
                .into_any_element()
        })
        .collect::<Vec<_>>();

    // 4. 最近播放卡片
    let recent_arc = recent.clone();
    let recent_items = recent
        .iter()
        .take(6)
        .enumerate()
        .map(|(idx, track)| {
            let cover_path = root.cover_of(&track.cover);
            let title = track.title.clone();
            let artist = track.artist.clone();
            let play_list = recent_arc.clone();
            div()
                .id(("home-recent-track", idx))
                .flex()
                .flex_row()
                .items_center()
                .gap(px(theme::space::MD))
                .w(px(250.0))
                .p(px(theme::space::SM))
                .rounded(px(theme::radius::ROW))
                .bg(theme::surface_elevated())
                .border_1()
                .border_color(theme::border())
                .cursor_pointer()
                .hover(|style| style.bg(theme::surface_hover()))
                .on_click(cx.listener(move |root, _event: &ClickEvent, _window, cx| {
                    root.play_from_arc(play_list.clone(), idx, cx);
                }))
                .child(cover(cover_path, 40.0, theme::radius::ART))
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
                                .text_color(theme::text())
                                .child(title),
                        )
                        .child(
                            div()
                                .truncate()
                                .text_size(theme::Text::Tiny.size())
                                .text_color(theme::text_muted())
                                .child(artist),
                        ),
                )
                .into_any_element()
        })
        .collect::<Vec<_>>();

    div()
        .id("home-scroll")
        .flex()
        .flex_col()
        .flex_1()
        .min_h(px(0.0))
        .overflow_y_scroll()
        .gap(px(theme::space::XXL))
        // 顶部推荐横幅 (Hero Banner)
        .child(
            div()
                .flex()
                .flex_row()
                .flex_wrap()
                .items_center()
                .gap(px(theme::space::XL))
                .p(px(theme::space::XL))
                .rounded(px(theme::radius::CARD))
                .bg(theme::surface_elevated())
                .border_1()
                .border_color(theme::border())
                .child(cover(featured_cover, 176.0, theme::radius::CARD))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(theme::space::SM))
                        .flex_1()
                        .min_w(px(220.0))
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(px(theme::space::SM))
                                .child(
                                    div()
                                        .px(px(theme::space::SM))
                                        .py(px(2.0))
                                        .rounded(px(theme::radius::PILL))
                                        .bg(theme::accent_soft())
                                        .text_size(theme::Text::Tiny.size())
                                        .font_weight(gpui::FontWeight::BOLD)
                                        .text_color(theme::accent())
                                        .child(root.tr("今日雅荐")),
                                )
                                .child(
                                    div()
                                        .text_size(theme::Text::Small.size())
                                        .text_color(theme::text_muted())
                                        .child(root.tr(theme::current_theme().name_zh())),
                                ),
                        )
                        .child(
                            div()
                                .text_size(px(26.0))
                                .font_weight(gpui::FontWeight::BOLD)
                                .text_color(theme::text())
                                .truncate()
                                .child(featured_title),
                        )
                        .child(
                            div()
                                .text_size(theme::Text::Body.size())
                                .text_color(theme::text_muted())
                                .child(
                                    root.localized(
                                        "{} 首歌曲，按你的听歌习惯整理",
                                        &[featured_count],
                                    ),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .flex_wrap()
                                .items_center()
                                .gap(px(theme::space::MD))
                                .pt(px(theme::space::XS))
                                .child(
                                    div()
                                        .id("home-play-stream")
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .gap(px(theme::space::SM))
                                        .px(px(theme::space::LG))
                                        .h(px(theme::size::CONTROL))
                                        .rounded(px(theme::radius::PILL))
                                        .bg(theme::accent())
                                        .text_color(theme::accent_foreground())
                                        .cursor_pointer()
                                        .hover(|style| style.layer_scale(1.03))
                                        .on_click(cx.listener(
                                            |root, _event: &ClickEvent, _window, cx| {
                                                root.activate_recommendation(cx);
                                            },
                                        ))
                                        .child(
                                            svg()
                                                .path(icons::path("play"))
                                                .size(px(theme::ICON_SM)),
                                        )
                                        .child(root.tr("畅听推荐流")),
                                )
                                .when_some(featured, |row, featured_card| {
                                    row.child(
                                        div()
                                            .id("home-play-all")
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .gap(px(theme::space::SM))
                                            .px(px(theme::space::LG))
                                            .h(px(theme::size::CONTROL))
                                            .rounded(px(theme::radius::PILL))
                                            .bg(theme::surface_hover())
                                            .text_color(theme::text())
                                            .cursor_pointer()
                                            .hover(|style| style.layer_scale(1.03))
                                            .on_click(cx.listener(
                                                move |root, _event: &ClickEvent, _window, cx| {
                                                    root.start_scene_card(
                                                        featured_card.clone(),
                                                        cx,
                                                    );
                                                },
                                            ))
                                            .child(root.tr("播放精选歌单")),
                                    )
                                }),
                        ),
                ),
        )
        // 个性化内容（听歌模式）
        .when(!personalized_items.is_empty(), |this| {
            this.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(theme::space::MD))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .child(section_title(root.tr("个性化听歌模式")))
                            .child(
                                div()
                                    .id("home-more-scenes")
                                    .text_size(theme::Text::Small.size())
                                    .text_color(theme::accent())
                                    .cursor_pointer()
                                    .on_click(cx.listener(
                                        |root, _event: &ClickEvent, _window, cx| {
                                            root.set_nav(Nav::Scenes, cx);
                                        },
                                    ))
                                    .child(root.tr("全部模式 →")),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .flex_wrap()
                            .gap(px(theme::space::MD))
                            .children(personalized_items),
                    ),
            )
        })
        // 每日推荐
        .when(!daily_items.is_empty(), |this| {
            this.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(theme::space::MD))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .child(section_title(root.tr("每日推荐")))
                            .child(
                                div()
                                    .id("home-refresh-daily")
                                    .text_size(theme::Text::Small.size())
                                    .text_color(theme::accent())
                                    .cursor_pointer()
                                    .on_click(cx.listener(
                                        |root, _event: &ClickEvent, _window, cx| {
                                            root.start_recommendation(true, cx);
                                        },
                                    ))
                                    .child(root.tr("刷新日推")),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .flex_wrap()
                            .gap(px(theme::space::MD))
                            .children(daily_items),
                    ),
            )
        })
        // 推荐歌单
        .when(!card_items.is_empty(), |this| {
            this.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(theme::space::MD))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .child(section_title(root.tr("推荐歌单")))
                            .child(
                                div()
                                    .id("home-load-more-cards")
                                    .text_size(theme::Text::Small.size())
                                    .text_color(theme::accent())
                                    .cursor_pointer()
                                    .on_click(cx.listener(
                                        |root, _event: &ClickEvent, _window, cx| {
                                            root.load_scenes(cx);
                                        },
                                    ))
                                    .child(root.tr("加载更多")),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .flex_wrap()
                            .gap(px(theme::space::LG))
                            .children(card_items),
                    ),
            )
        })
        // 最近播放
        .when(!recent_items.is_empty(), |this| {
            this.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(theme::space::MD))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .child(section_title(root.tr("最近播放")))
                            .child(
                                div()
                                    .id("home-open-recent")
                                    .text_size(theme::Text::Small.size())
                                    .text_color(theme::accent())
                                    .cursor_pointer()
                                    .on_click(cx.listener(
                                        |root, _event: &ClickEvent, _window, cx| {
                                            root.set_nav(Nav::Recent, cx);
                                        },
                                    ))
                                    .child(root.tr("查看全部 →")),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .flex_wrap()
                            .gap(px(theme::space::MD))
                            .children(recent_items),
                    ),
            )
        })
        .into_any_element()
}

/// 听歌模式页：常用模式 + 探索歌单，使用变高虚拟列表只渲染可视行。
pub(crate) fn scenes_view(root: &Root, cx: &mut Context<Root>) -> AnyElement {
    let scenes = root.scenes.clone();
    let language = root.language;
    let scene_texts: std::sync::Arc<Vec<String>> = std::sync::Arc::new(
        scenes
            .iter()
            .map(|scene| root.scene_display_text(&scene.sub_queue_type, &scene.text))
            .collect(),
    );
    let cards = root.scene_cards.clone();
    let covers = root.covers.clone();
    let cover_requests = root.cover_requests.clone();
    let selected_scene = root
        .recommendation
        .as_ref()
        .and_then(|source| source.scene.as_ref())
        .map(|scene| scene.sub_queue_type.clone());
    let _loading_scenes = root.loading_scenes;
    let loading_cards = root.loading_scene_cards;
    let card_count = cards.len();
    let entity = cx.entity();
    let common_columns = root.scenes_common_columns.max(1);
    let card_columns = root.scenes_card_columns.max(1);
    let common_rows = scenes.len().div_ceil(common_columns).max(1);
    let card_rows = cards.len().div_ceil(card_columns).max(1);
    let width_slot = root.scenes_width.clone();

    // 首次加载是整页状态：不要同时显示“读取模式/暂无歌单/加载更多”。
    if root.loading_scenes && cards.is_empty() {
        return div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(loading_state(language.text("正在读取听歌模式…")))
            .into_any_element();
    }

    let list = gpui::list(root.scenes_list.clone(), move |index, _window, _cx| {
        // Header: 常用模式
        if index == 0 {
            div()
                .w_full()
                .h(px(44.0))
                .flex()
                .items_end()
                .pb(px(theme::space::MD))
                .child(section_title(language.text("常用模式")))
                .into_any_element()
        }
        // 常用模式虚拟网格行
        else if index <= common_rows {
            let row = index - 1;
            if scenes.is_empty() {
                return div()
                    .id("scene-modes-loading")
                    .w_full()
                    .h(px(theme::size::CONTROL + 20.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(loading_state(language.text("正在读取常用模式…")))
                    .into_any_element();
            }
            let mut row_element = div()
                .id(("scene-mode-row", row))
                .w_full()
                .flex()
                .flex_row()
                .gap(px(theme::space::MD))
                .h(px(theme::size::CONTROL + 20.0));

            for column in 0..common_columns {
                let Some(scene) = scenes.get(row * common_columns + column) else {
                    row_element = row_element.child(div().flex_1().min_w(px(0.0)));
                    continue;
                };
                let selected = selected_scene
                    .as_ref()
                    .is_some_and(|selected| selected == &scene.sub_queue_type);
                let click_scene = scene.clone();
                row_element = row_element.child(
                    div()
                        .id(gpui::ElementId::Name(
                            format!("scene-{}", scene.sub_queue_type).into(),
                        ))
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(theme::space::SM))
                        .flex_1()
                        .min_w(px(0.0))
                        .h(px(theme::size::CONTROL + 12.0))
                        .px(px(theme::space::MD))
                        .rounded(px(theme::radius::CARD))
                        .bg(theme::surface_elevated())
                        .border_1()
                        .border_color(if selected {
                            theme::accent()
                        } else {
                            theme::border()
                        })
                        .cursor_pointer()
                        .hover(|style| style.bg(theme::surface_hover()).layer_scale(1.015))
                        .on_click({
                            let entity = entity.clone();
                            move |_event: &ClickEvent, _window, cx: &mut gpui::App| {
                                entity.update(cx, |root, cx| {
                                    root.start_scene_queue(click_scene.clone(), cx);
                                });
                            }
                        })
                        .child(scene_image(&scene.sub_queue_type, 24.0))
                        .child(
                            div()
                                .min_w(px(0.0))
                                .truncate()
                                .text_size(theme::Text::Body.size())
                                .text_color(theme::text())
                                .child(
                                    scene_texts
                                        .get(row * common_columns + column)
                                        .cloned()
                                        .unwrap_or_else(|| scene.text.clone()),
                                ),
                        ),
                );
            }
            row_element.into_any_element()
        } else {
            let mut index = index - common_rows - 1;

            // Header: 探索更多新模式
            if index == 0 {
                return div()
                    .w_full()
                    .h(px(44.0))
                    .flex()
                    .items_end()
                    .pb(px(theme::space::MD))
                    .child(section_title(language.text("探索更多新模式")))
                    .into_any_element();
            }
            index -= 1;

            // 探索卡虚拟网格行
            if index < card_rows {
                let row = index;
                let mut row_element = div()
                    .id(("scene-card-row", row))
                    .w_full()
                    .flex()
                    .flex_row()
                    .items_start()
                    .gap(px(theme::space::LG))
                    .h(px(232.0));

                for column in 0..card_columns {
                    let Some(card) = cards.get(row * card_columns + column) else {
                        row_element = row_element.child(div().flex_1().min_w(px(0.0)));
                        continue;
                    };
                    let url = card.playlist.cover.clone();
                    let cached = covers.get(&url).cloned();
                    if cached.is_none() && !url.is_empty() {
                        if let Ok(mut pending) = cover_requests.lock() {
                            if !pending.iter().any(|item| item == &url) {
                                pending.push(url.clone());
                            }
                        }
                    }

                    let subtitle = if card.kind == sodam_core::library::SceneCardKind::Radio {
                        language.text("电台").to_string()
                    } else {
                        language.textf("{} 首", &[card.playlist.track_count.to_string()])
                    };
                    row_element = row_element.child(
                        div()
                            .id(gpui::ElementId::Name(
                                format!("scene-card-{}", card.inner_block_id).into(),
                            ))
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap(px(theme::space::SM))
                            .flex_1()
                            .min_w(px(0.0))
                            .p(px(theme::space::SM))
                            .rounded(px(theme::radius::CARD))
                            .bg(theme::surface_elevated())
                            .border_1()
                            .border_color(theme::border())
                            .cursor_pointer()
                            .hover(|style| style.bg(theme::surface_hover()).layer_scale(1.02))
                            .on_click({
                                let entity = entity.clone();
                                let card = card.clone();
                                move |_event: &ClickEvent, _window, cx: &mut gpui::App| {
                                    entity.update(cx, |root, cx| {
                                        root.start_scene_card(card.clone(), cx);
                                    });
                                }
                            })
                            .child(cover(cached, 148.0, theme::radius::CARD))
                            .child(
                                div()
                                    .w_full()
                                    .truncate()
                                    .text_size(theme::Text::Body.size())
                                    .text_color(theme::text())
                                    .child(card.playlist.title.clone()),
                            )
                            .child(
                                div()
                                    .text_size(theme::Text::Small.size())
                                    .text_color(theme::text_muted())
                                    .child(subtitle),
                            ),
                    );
                }
                if cards.is_empty() {
                    let state = if loading_cards {
                        loading_state(language.text("正在读取探索歌单…"))
                    } else {
                        empty_state("list-music", language.text("暂无探索歌单"))
                    };
                    row_element = row_element.child(
                        div()
                            .flex_1()
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(state),
                    );
                }
                row_element.into_any_element()
            } else {
                // Load more row
                div()
                    .id("load-more-scene-cards-row")
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .h(px(theme::size::CONTROL + theme::space::MD))
                    .child(
                        div()
                            .id("load-more-scene-cards")
                            .flex()
                            .items_center()
                            .justify_center()
                            .h(px(theme::size::CONTROL))
                            .px(px(theme::space::XL))
                            .rounded(px(theme::radius::PILL))
                            .bg(theme::surface_elevated())
                            .cursor_pointer()
                            .hover(|style| style.bg(theme::surface_hover()))
                            .on_click({
                                let entity = entity.clone();
                                move |_event: &ClickEvent, _window, cx: &mut gpui::App| {
                                    entity.update(cx, |root, cx| {
                                        root.load_scenes(cx);
                                    });
                                }
                            })
                            .text_size(theme::Text::Small.size())
                            .text_color(theme::text_muted())
                            .child(if loading_cards {
                                language.text("正在加载…").to_string()
                            } else {
                                language.textf(
                                    "加载更多探索歌单（当前 {} 张）",
                                    &[card_count.to_string()],
                                )
                            }),
                    )
                    .into_any_element()
            }
        }
    })
    .w_full()
    .flex_1()
    .min_h(px(0.0));

    div()
        .relative()
        .flex()
        .flex_col()
        .flex_1()
        .min_h(px(0.0))
        .child(
            canvas(
                move |bounds: gpui::Bounds<gpui::Pixels>, _window, _cx| {
                    if let Ok(mut width) = width_slot.lock() {
                        *width = f32::from(bounds.size.width);
                    }
                },
                |_bounds, _state, _window, _cx| {},
            )
            .absolute()
            .size_full(),
        )
        .child(list)
        .into_any_element()
}

pub(crate) fn retry_button(
    language: crate::ui::i18n::Language,
    cx: &mut Context<Root>,
) -> AnyElement {
    div()
        .id("retry-playback-assets")
        .flex()
        .items_center()
        .justify_center()
        .h(px(theme::size::CONTROL_SM))
        .px(px(theme::space::MD))
        .rounded(px(theme::radius::PILL))
        .bg(theme::surface_elevated())
        .cursor_pointer()
        .hover(|style| style.bg(theme::surface_hover()))
        .text_size(theme::Text::Tiny.size())
        .text_color(theme::text_muted())
        .on_click(cx.listener(|root, _event: &ClickEvent, _window, cx| {
            if let Some(track) = root
                .pending_track
                .clone()
                .or_else(|| root.queue.current().cloned())
            {
                root.retry_playback_assets(track, cx);
            }
        }))
        .child(language.text("重试加载"))
        .into_any_element()
}

fn section_title(text: &str) -> AnyElement {
    div()
        .text_size(theme::Text::Large.size())
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(theme::text())
        .child(text.to_string())
        .into_any_element()
}
