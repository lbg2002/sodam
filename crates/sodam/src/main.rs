//! SodaM——GPUI 原生客户端入口。
//
// 发布版不分配控制台窗口（双击/快捷方式启动不再弹 terminal）；
// debug 构建保留终端，便于查看 eprintln 日志。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
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
            if tick % 10 == 0 {
                let state = cx.read_entity(&app, |root, _| {
                    let snapshot = root.engine.snapshot();
                    tray::TrayState {
                        playing: snapshot.playing,
                        title: snapshot.title,
                    }
                });
                if let Ok(state) = state {
                    sync(state);
                }
            }
        }
    })
    .detach();
}

fn show_window(cx: &mut App, app: &Entity<app::Root>) {
    if let Some(handle) = cx.windows().into_iter().next() {
        let _ = handle.update(cx, |_view, window, _cx| {
            window.activate_window();
        });
        return;
    }
    let app = app.clone();
    let _ = cx.open_window(main_window_options(cx), move |_window, _cx| app.clone());
}

#[cfg(target_os = "linux")]
fn start_mpris_service(bridge: mpris::MprisBridge, app: Entity<app::Root>, cx: &mut App) {
    let state = bridge.state.clone();
    let receiver = bridge.receiver;
    cx.spawn(async move |cx| {
        loop {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(100))
                .await;
            while let Ok(command) = receiver.try_recv() {
                match command {
                    mpris::MprisCommand::PlayPause => cx.update(|cx| {
                        app.update(cx, |root, cx| root.toggle_play(cx));
                    }),
                    mpris::MprisCommand::Play => cx.update(|cx| {
                        app.update(cx, |root, cx| root.play(cx));
                    }),
                    mpris::MprisCommand::Pause => cx.update(|cx| {
                        app.update(cx, |root, cx| root.pause(cx));
                    }),
                    mpris::MprisCommand::Stop => cx.update(|cx| {
                        app.update(cx, |root, cx| root.stop_playback(cx));
                    }),
                    mpris::MprisCommand::Next => cx.update(|cx| {
                        app.update(cx, |root, cx| root.next_track(cx));
                    }),
                    mpris::MprisCommand::Previous => cx.update(|cx| {
                        app.update(cx, |root, cx| root.prev_track(cx));
                    }),
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
                            root.set_volume(volume as f32);
                            cx.notify();
                        })
                    }),
                    mpris::MprisCommand::Raise => cx.update(|cx| show_window(cx, &app)),
                    mpris::MprisCommand::Quit => cx.update(|cx| {
                        app.update(cx, |root, _cx| root.save_playback_state_now());
                        cx.quit();
                    }),
                };
            }

            if let Ok(next) = cx.read_entity(&app, |root, _| {
                let snapshot = root.engine.snapshot();
                let track = root.queue.current();
                let (artist, album, art_url) = track
                    .map(|track| {
                        (
                            track.artist.clone(),
                            track.album.clone(),
                            root.cover_of(&track.cover)
                                .map(|path| format!("file://{}", path.display()))
                                .unwrap_or_default(),
                        )
                    })
                    .unwrap_or_default();
                mpris::MprisState {
                    title: snapshot.title,
                    artist,
                    album,
                    art_url,
                    playing: snapshot.playing,
                    position_micros: (snapshot.position_seconds.max(0.0) * 1_000_000.0) as i64,
                    duration_micros: (snapshot.duration_seconds.max(0.0) * 1_000_000.0) as i64,
                    volume: snapshot.volume as f64,
                    can_go_next: root.queue.len() > 1,
                    can_go_previous: root.queue.len() > 1,
                }
            }) {
                mpris::update_state(&state, next);
            }
        }
    })
    .detach();
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        ui::assets::init(cx);
        let window = cx
            .open_window(main_window_options(cx), |_window, cx| cx.new(app::Root::new))
            .expect("创建主窗口失败");
        let app = window.entity(cx).clone();

        if let Ok((rx, sync)) = tray::start() {
            start_tray_service(rx, app.clone(), cx, sync);
        }

        #[cfg(target_os = "linux")]
        if let Ok(bridge) = mpris::start() {
            start_mpris_service(bridge, app.clone(), cx);
        }

        cx.on_app_quit({
            let app = app.clone();
            move |cx| {
                let _ = app.update(cx, |root, _cx| root.save_playback_state_now());
            }
        })
        .detach();

        cx.activate(true);
    });
}
