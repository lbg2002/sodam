//! SodaM——GPUI 原生客户端入口。
//
// 发布版不分配控制台窗口（双击/快捷方式启动不再弹 terminal）；
// debug 构建保留终端，便于查看 eprintln 日志。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod experience3;
#[cfg(target_os = "linux")]
mod mpris;
mod system_audio;
// 托盘：Linux 走 ksni/SNI，macOS 走 NSStatusItem，见 tray.rs。
mod tray;
mod ui;
mod views;

use gpui::{
    px, size, App, AppContext as _, Bounds, Entity, TitlebarOptions, WindowBounds, WindowOptions,
};
use std::sync::Arc;

/// 启动窗口尺寸（参考官方客户端的 16:10 主窗口）。
const DEFAULT_SIZE: (f32, f32) = (1180.0, 760.0);
const MIN_SIZE: (f32, f32) = (900.0, 560.0);

fn main_window_options(cx: &mut App) -> WindowOptions {
    let bounds = Bounds::centered(None, size(px(DEFAULT_SIZE.0), px(DEFAULT_SIZE.1)), cx);
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        window_min_size: Some(size(px(MIN_SIZE.0), px(MIN_SIZE.1))),
        titlebar: Some(TitlebarOptions {
            title: Some("SodaM".into()),
            // macOS 隐藏系统标题栏（红绿灯悬浮）；Windows 同样隐藏，
            // 由 ui::titlebar 自绘拖拽区与最小化/最大化/关闭按钮。
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            appears_transparent: true,
            ..Default::default()
        }),
        app_id: Some("SodaM".into()),
        icon: Some(Arc::new(
            image::load_from_memory(include_bytes!("../assets/brand/sodam-logo-tray.png"))
                .expect("内置应用图标应为有效 PNG")
                .to_rgba8(),
        )),
        ..Default::default()
    }
}

/// 托盘事件循环：把命令转发为应用动作，并周期同步播放状态到 `sync`。
/// 双平台共用；平台差异只在 `sync` 闭包（见 tray.rs）。
fn start_tray_service(
    rx: std::sync::mpsc::Receiver<tray::TrayCommand>,
    app: Entity<app::Root>,
    cx: &mut App,
    mut sync: impl FnMut(tray::TrayState) + 'static,
) {
    cx.spawn(async move |cx| {
        let mut tick = 0u32;
        loop {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(100))
                .await;
            while let Ok(command) = rx.try_recv() {
                match command {
                    tray::TrayCommand::Show => cx.update(|cx| show_window(cx, &app)),
                    tray::TrayCommand::Toggle => {
                        cx.update(|cx| app.update(cx, |root, cx| root.toggle_play(cx)))
                    }
                    tray::TrayCommand::Previous => {
                        cx.update(|cx| app.update(cx, |root, cx| root.prev_track(cx)))
                    }
                    tray::TrayCommand::Next => {
                        cx.update(|cx| app.update(cx, |root, cx| root.next_track(cx)))
                    }
                    tray::TrayCommand::Quit => cx.update(|cx| {
                        app.update(cx, |root, _cx| root.save_playback_state_now());
                        cx.quit();
                    }),
                };
            }

            tick += 1;
            if tick % 5 == 0 {
                cx.update(|cx| {
                    app.update(cx, |root, _cx| {
                        let snapshot = root.engine.snapshot();
                        let title = if snapshot.track_id.is_empty() {
                            "SodaM".to_string()
                        } else {
                            snapshot.title
                        };
                        let subtitle = if snapshot.track_id.is_empty() {
                            root.tr("就绪").to_string()
                        } else {
                            root.queue
                                .current()
                                .map(|track| track.artist.clone())
                                .unwrap_or_default()
                        };
                        sync(tray::TrayState {
                            language: root.language,
                            title,
                            subtitle,
                            playing: snapshot.playing,
                        });
                    });
                });
            }
        }
    })
    .detach();
}

#[cfg(target_os = "linux")]
fn start_panel_lyrics_service(app: Entity<app::Root>, cx: &mut App) {
    // GNOME Shell 扩展不能直接读取应用内存，因此用一个很小的状态文件做桥接。
    // 文件写入放在独立线程，避免任何磁盘 IO 卡住 GPUI 渲染线程。
    let state_path = sodam_core::Settings::config_path().with_file_name("panel-lyrics-state");
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    std::thread::Builder::new()
        .name("sodam-panel-lyrics".into())
        .spawn(move || {
            if let Some(parent) = state_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            while let Ok(content) = rx.recv() {
                let tmp = state_path.with_extension("tmp");
                if std::fs::write(&tmp, content.as_bytes()).is_ok() {
                    let _ = std::fs::rename(&tmp, &state_path);
                }
            }
        })
        .ok();

    cx.spawn(async move |cx| {
        let mut last_state = String::new();
        loop {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(200))
                .await;
            let Ok(state) = cx.update(|cx| {
                let root = app.read(cx);
                let snapshot = root.engine.snapshot();
                let position =
                    snapshot.position_seconds - root.settings.lyrics_offset_ms as f64 / 1000.0;
                let active = root
                    .lyrics
                    .iter()
                    .rposition(|line| position + 0.25 >= line.start_seconds)
                    .or_else(|| (!root.lyrics.is_empty()).then_some(0));
                let lyric = active
                    .and_then(|index| root.lyrics.get(index))
                    .map(|line| {
                        line.text
                            .trim()
                            .replace('\n', " ")
                            .replace('\r', " ")
                            .replace('\t', " ")
                    })
                    .unwrap_or_default();
                let position = match root.settings.panel_lyrics_position.as_str() {
                    "left" | "center-right" | "right" => {
                        root.settings.panel_lyrics_position.as_str()
                    }
                    _ => "center-left",
                };
                format!(
                    "{}\n{}\n{}\n",
                    if root.settings.panel_lyrics_enabled { "1" } else { "0" },
                    position,
                    lyric
                )
            }) else {
                continue;
            };
            if state != last_state {
                last_state = state.clone();
                if tx.send(state).is_err() {
                    break;
                }
            }
        }
    })
    .detach();
}

#[cfg(target_os = "linux")]
fn start_mpris_service(bridge: mpris::MprisBridge, app: Entity<app::Root>, cx: &mut App) {
    let state = bridge.state.clone();
    let receiver = bridge.receiver;
    cx.spawn(async move |cx| {
        let mut tick = 0u32;
        loop {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(100))
                .await;

            while let Ok(command) = receiver.try_recv() {
                match command {
                    mpris::MprisCommand::Raise => cx.update(|cx| show_window(cx, &app)),
                    mpris::MprisCommand::Play => {
                        cx.update(|cx| app.update(cx, |root, cx| root.play(cx)))
                    }
                    mpris::MprisCommand::Pause => {
                        cx.update(|cx| app.update(cx, |root, cx| root.pause(cx)))
                    }
                    mpris::MprisCommand::PlayPause => {
                        cx.update(|cx| app.update(cx, |root, cx| root.toggle_play(cx)))
                    }
                    mpris::MprisCommand::Stop => {
                        cx.update(|cx| app.update(cx, |root, cx| root.stop_playback(cx)))
                    }
                    mpris::MprisCommand::Next => {
                        cx.update(|cx| app.update(cx, |root, cx| root.next_track(cx)))
                    }
                    mpris::MprisCommand::Previous => {
                        cx.update(|cx| app.update(cx, |root, cx| root.prev_track(cx)))
                    }
                    mpris::MprisCommand::SeekRelative(offset) => cx.update(|cx| {
                        app.update(cx, |root, cx| {
                            let snapshot = root.engine.snapshot();
                            let seconds = snapshot.position_seconds + offset as f64 / 1_000_000.0;
                            root.engine.seek(seconds.max(0.0));
                            root.persist_playback_state(cx);
                            cx.notify();
                        })
                    }),
                    mpris::MprisCommand::SeekAbsolute(position) => cx.update(|cx| {
                        app.update(cx, |root, cx| {
                            root.engine.seek(position.max(0) as f64 / 1_000_000.0);
                            root.persist_playback_state(cx);
                            cx.notify();
                        })
                    }),
                    mpris::MprisCommand::SetVolume(volume) => cx.update(|cx| {
                        app.update(cx, |root, cx| {
                            root.set_volume(volume.clamp(0.0, 1.0) as f32);
                            cx.notify();
                        })
                    }),
                };
            }

            tick = tick.wrapping_add(1);
            if tick % 5 == 0 {
                cx.update(|cx| {
                    app.update(cx, |root, _cx| {
                        let snapshot = root.engine.snapshot();
                        let current = root.queue.current();
                        let (artist, album, art_url) = current
                            .map(|track| {
                                (
                                    track.artist.clone(),
                                    track.album.clone(),
                                    track.cover.clone(),
                                )
                            })
                            .unwrap_or_default();
                        let loop_status = match root.queue.mode {
                            sodam_core::queue::PlayMode::RepeatOne => "Track",
                            _ => "None",
                        }
                        .to_string();
                        if let Ok(mut slot) = state.lock() {
                            *slot = mpris::MprisState {
                                track_id: snapshot.track_id,
                                title: snapshot.title,
                                artist,
                                album,
                                art_url,
                                playing: snapshot.playing,
                                position_micros: (snapshot.position_seconds.max(0.0) * 1_000_000.0)
                                    as i64,
                                duration_micros: (snapshot.duration_seconds.max(0.0) * 1_000_000.0)
                                    as i64,
                                volume: snapshot.volume as f64,
                                can_go_next: root.queue.len() > 1,
                                can_go_previous: root.queue.len() > 1,
                                can_play: !root.queue.is_empty(),
                                loop_status,
                                shuffle: matches!(
                                    root.queue.mode,
                                    sodam_core::queue::PlayMode::Shuffle
                                ),
                            };
                        }
                    });
                });
            }
        }
    })
    .detach();
}

/// 打开或激活主窗口；由托盘的 Show 命令调用。
fn show_window(cx: &mut App, app: &Entity<app::Root>) {
    if let Some(window) = cx
        .windows()
        .iter()
        .find_map(|window| window.downcast::<app::Root>())
    {
        let _ = window.update(cx, |_root, window, _| window.activate_window());
    } else {
        let app = app.clone();
        let options = main_window_options(cx);
        cx.open_window(options, |_window, _cx| app).ok();
    }
    cx.activate(true);
}

fn main() {
    gpui_platform::application()
        .with_assets(ui::icons::Assets)
        .with_quit_mode(gpui::QuitMode::Explicit)
        .run(|cx: &mut App| {
            #[allow(clippy::redundant_closure)]
            let app = cx.new(|cx| app::Root::new(cx));
            let options = main_window_options(cx);
            cx.open_window(options, |_window, _cx| app.clone())
                .expect("打开主窗口失败");

            #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
            {
                let (tray_tx, tray_rx) = std::sync::mpsc::channel();

                #[cfg(target_os = "linux")]
                let sync = {
                    let tray_service = ksni::TrayService::new(tray::linux::SodaTray::new(
                        tray_tx,
                        crate::ui::i18n::Language::system_locale(),
                    ));
                    let tray_handle = tray_service.handle();
                    tray_service.spawn();
                    move |state: tray::TrayState| {
                        tray_handle.update(|tray| {
                            tray.language = state.language;
                            tray.title = state.title;
                            tray.subtitle = state.subtitle;
                            tray.playing = state.playing;
                        });
                    }
                };
                #[cfg(any(target_os = "macos", target_os = "windows"))]
                let sync =
                    tray::create_status_item(tray_tx, crate::ui::i18n::Language::system_locale());

                start_tray_service(tray_rx, app.clone(), cx, sync);
            }

            #[cfg(target_os = "linux")]
            {
                start_mpris_service(mpris::start(), app.clone(), cx);
                start_panel_lyrics_service(app.clone(), cx);
            }

            cx.activate(true);
        });
}
