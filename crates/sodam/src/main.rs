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
mod updater;
mod views;

#[cfg(target_os = "linux")]
use gpui::WindowDecorations;
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
        is_resizable: true,
        titlebar: Some(TitlebarOptions {
            title: Some("流光音乐 · Lumina".into()),
            // 三个平台都使用透明标题栏。Windows/Linux 由 ui::titlebar
            // 自绘拖拽区与最小化/最大化/关闭按钮；Linux/Wayland（Ubuntu 26）
            // 不再依赖系统窗口装饰是否提供按钮。
            #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
            appears_transparent: true,
            ..Default::default()
        }),
        app_id: Some("SodaM".into()),
        #[cfg(target_os = "linux")]
        window_decorations: Some(WindowDecorations::Client),
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
            if tick.is_multiple_of(5) {
                cx.update(|cx| {
                    app.update(cx, |root, _cx| {
                        let snapshot = root.engine.snapshot();
                        let title = if snapshot.track_id.is_empty() {
                            if root.language.is_zh() {
                                "流光音乐".to_string()
                            } else {
                                "Lumina".to_string()
                            }
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
fn panel_color_hex(color: gpui::Rgba) -> String {
    let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!(
        "#{:02X}{:02X}{:02X}",
        channel(color.r),
        channel(color.g),
        channel(color.b)
    )
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IpcCommand {
    Show,
    Toggle,
    Play,
    Pause,
    Next,
    Previous,
    Quit,
}

#[cfg(target_os = "linux")]
struct SingleInstanceGuard {
    path: std::path::PathBuf,
}

#[cfg(target_os = "linux")]
impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
        let state_path = sodam_core::Settings::config_path().with_file_name("panel-lyrics-state");
        let tmp = state_path.with_extension("tmp");
        if std::fs::write(&tmp, b"{\"enabled\":false}\n").is_ok() {
            let _ = std::fs::rename(&tmp, &state_path);
        } else {
            let _ = std::fs::write(&state_path, b"{\"enabled\":false}\n");
        }
    }
}

#[cfg(target_os = "linux")]
fn instance_socket_path() -> std::path::PathBuf {
    if let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR") {
        return std::path::PathBuf::from(runtime).join("sodam-instance.sock");
    }
    let user = std::env::var("USER").unwrap_or_else(|_| "user".to_string());
    std::env::temp_dir().join(format!("sodam-instance-{user}.sock"))
}

#[cfg(target_os = "linux")]
fn parse_ipc_arg() -> &'static str {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--ipc" {
            if let Some(cmd) = args.next() {
                return match cmd.trim().to_ascii_lowercase().as_str() {
                    "toggle" => "toggle",
                    "play" => "play",
                    "pause" => "pause",
                    "next" => "next",
                    "prev" | "previous" => "prev",
                    "quit" => "quit",
                    _ => "show",
                };
            }
        }
    }
    "show"
}

#[cfg(target_os = "linux")]
enum InstanceLock {
    Primary(
        #[allow(dead_code)] SingleInstanceGuard,
        std::sync::mpsc::Receiver<IpcCommand>,
    ),
    Bypassed,
    ForwardedToExisting,
}

#[cfg(target_os = "linux")]
fn acquire_single_instance() -> InstanceLock {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::{UnixListener, UnixStream};

    if std::env::var_os("SODAM_ALLOW_MULTI").is_some() {
        return InstanceLock::Bypassed;
    }

    let sock_path = instance_socket_path();
    if let Ok(mut stream) = UnixStream::connect(&sock_path) {
        let cmd = parse_ipc_arg();
        let _ = writeln!(stream, "{cmd}");
        let _ = stream.flush();
        return InstanceLock::ForwardedToExisting;
    }

    let _ = std::fs::remove_file(&sock_path);
    if let Some(parent) = sock_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let Ok(listener) = UnixListener::bind(&sock_path) else {
        return InstanceLock::Bypassed;
    };

    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("sodam-ipc".into())
        .spawn(move || {
            for incoming in listener.incoming().flatten() {
                let reader = BufReader::new(incoming);
                for line in reader.lines().map_while(std::result::Result::ok) {
                    let cmd = match line.trim().to_ascii_lowercase().as_str() {
                        "toggle" => IpcCommand::Toggle,
                        "play" => IpcCommand::Play,
                        "pause" => IpcCommand::Pause,
                        "next" => IpcCommand::Next,
                        "prev" | "previous" => IpcCommand::Previous,
                        "quit" => IpcCommand::Quit,
                        _ => IpcCommand::Show,
                    };
                    if tx.send(cmd).is_err() {
                        return;
                    }
                }
            }
        })
        .ok();

    InstanceLock::Primary(SingleInstanceGuard { path: sock_path }, rx)
}

#[cfg(target_os = "linux")]
fn start_ipc_service(
    rx: std::sync::mpsc::Receiver<IpcCommand>,
    app: Entity<app::Root>,
    cx: &mut App,
) {
    cx.spawn(async move |cx| loop {
        cx.background_executor()
            .timer(std::time::Duration::from_millis(40))
            .await;
        while let Ok(command) = rx.try_recv() {
            match command {
                IpcCommand::Show => cx.update(|cx| show_window(cx, &app)),
                IpcCommand::Toggle => {
                    cx.update(|cx| app.update(cx, |root, cx| root.toggle_play(cx)))
                }
                IpcCommand::Play => cx.update(|cx| app.update(cx, |root, cx| root.play(cx))),
                IpcCommand::Pause => cx.update(|cx| app.update(cx, |root, cx| root.pause(cx))),
                IpcCommand::Next => cx.update(|cx| app.update(cx, |root, cx| root.next_track(cx))),
                IpcCommand::Previous => {
                    cx.update(|cx| app.update(cx, |root, cx| root.prev_track(cx)))
                }
                IpcCommand::Quit => cx.update(|cx| {
                    app.update(cx, |root, _cx| root.save_playback_state_now());
                    cx.quit();
                }),
            }
        }
    })
    .detach();
}

#[cfg(target_os = "linux")]
fn sync_local_gnome_extension() {
    let sys_dir =
        std::path::PathBuf::from("/usr/share/gnome-shell/extensions/sodam-panel-lyrics@lbg2002");
    let Some(data_local) = std::env::var_os("XDG_DATA_HOME")
        .filter(|v| !v.is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .filter(|v| !v.is_empty())
                .map(|home| std::path::PathBuf::from(home).join(".local/share"))
        })
    else {
        return;
    };
    let local_dir = data_local
        .join("gnome-shell")
        .join("extensions")
        .join("sodam-panel-lyrics@lbg2002");
    let sys_meta_path = sys_dir.join("metadata.json");
    let local_meta_path = local_dir.join("metadata.json");
    if !sys_meta_path.exists() || !local_meta_path.exists() {
        return;
    }
    let read_ver = |p: &std::path::Path| -> u64 {
        std::fs::read_to_string(p)
            .ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .and_then(|v| v["version"].as_u64())
            .unwrap_or(0)
    };
    if read_ver(&sys_meta_path) <= read_ver(&local_meta_path) {
        return;
    }
    for file in ["extension.js", "stylesheet.css", "metadata.json"] {
        let src = sys_dir.join(file);
        if src.exists() {
            let _ = std::fs::copy(&src, local_dir.join(file));
        }
    }
}

#[cfg(target_os = "linux")]
fn start_panel_lyrics_service(app: Entity<app::Root>, cx: &mut App) {
    // GNOME Shell 扩展不能直接读取应用内存，因此用一个很小的 JSON 状态文件做桥接。
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

    let pid = std::process::id();
    cx.spawn(async move |cx| {
        let mut last_key = String::new();
        let mut last_anchor: Option<(std::time::Instant, u64, bool)> = None;
        let mut last_playing = true;
        loop {
            let sleep_ms = if last_playing { 250 } else { 600 };
            cx.background_executor()
                .timer(std::time::Duration::from_millis(sleep_ms))
                .await;
            let (payload_opt, playing_now) = cx.update(|cx| {
                let root = app.read(cx);
                let snapshot = root.engine.snapshot();
                let current = root.queue.current();
                let lyrics_ready =
                    !snapshot.track_id.is_empty() && root.lyrics_track_id == snapshot.track_id;
                let active_idx = if lyrics_ready {
                    root.synced_lyrics_active()
                } else {
                    None
                };
                let lyric = active_idx
                    .and_then(|index| root.lyrics.get(index))
                    .map(|line| line.text.trim().replace(['\n', '\r', '\t'], " "))
                    .unwrap_or_default();
                let next_lyric = active_idx
                    .and_then(|index| root.lyrics.get(index.saturating_add(1)))
                    .map(|line| line.text.trim().replace(['\n', '\r', '\t'], " "))
                    .unwrap_or_default();
                let position = match root.settings.panel_lyrics_position.as_str() {
                    "left" | "center-left" | "center-right" | "right" => {
                        root.settings.panel_lyrics_position.as_str()
                    }
                    _ => "center-replace",
                };
                let style = if root.settings.panel_lyrics_style == "lyrics" {
                    "lyrics"
                } else {
                    "island"
                };

                // 顶栏颜色与应用当前主题/封面强调色保持一致。
                ui::theme::set_theme(root.theme);
                let ambient =
                    current.and_then(|track| root.ambient_colors.get(&track.cover).copied());
                ui::theme::set_ambient_rgb(ambient);
                let is_dark = ui::theme::current_theme().is_dark();
                let accent = ui::theme::accent();
                let chan = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
                let accent_rgb = [chan(accent.r), chan(accent.g), chan(accent.b)];
                let color = match root.settings.panel_lyrics_color.as_str() {
                    "text" => panel_color_hex(ui::theme::text()),
                    "white" => "#FFFFFF".to_string(),
                    // 默认：与桌面歌词默认模式一致，跟随当前歌曲封面主题强调色。
                    _ => panel_color_hex(accent),
                };

                let title = current
                    .map(|track| track.title.clone())
                    .filter(|s| !s.is_empty())
                    .unwrap_or(snapshot.title.clone());
                let artist = current
                    .map(|track| track.artist.clone())
                    .unwrap_or_default();
                let cover_path = current
                    .and_then(|track| root.cover_of(&track.cover))
                    .filter(|path| path.exists())
                    .map(|path| path.display().to_string())
                    .unwrap_or_default();
                let provider = current
                    .map(|track| sodam_core::MusicProvider::from_track_id(&track.id).key())
                    .unwrap_or_else(|| root.active_provider().key());
                let position_ms =
                    (snapshot.interpolated_position_seconds().max(0.0) * 1000.0).round() as u64;
                let duration_ms = (snapshot.duration_seconds.max(0.0) * 1000.0).round() as u64;

                let key = serde_json::json!({
                    "enabled": root.settings.panel_lyrics_enabled,
                    "style": style,
                    "position": position,
                    "lyric": &lyric,
                    "next_lyric": &next_lyric,
                    "color": &color,
                    "is_dark": is_dark,
                    "accent_rgb": accent_rgb,
                    "playing": snapshot.playing,
                    "title": &title,
                    "artist": &artist,
                    "cover_path": &cover_path,
                    "provider": provider,
                    "duration_ms": duration_ms,
                    "pid": pid,
                })
                .to_string();

                let seeked = match last_anchor {
                    Some((t0, p0, was_playing)) => {
                        let elapsed = if was_playing {
                            t0.elapsed().as_millis() as u64
                        } else {
                            0
                        };
                        let expected = p0.saturating_add(elapsed);
                        position_ms.abs_diff(expected) > 1500
                    }
                    None => false,
                };

                if key != last_key || seeked {
                    last_key = key;
                    last_anchor = Some((std::time::Instant::now(), position_ms, snapshot.playing));
                    let updated_at_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;
                    let json = serde_json::json!({
                        "enabled": root.settings.panel_lyrics_enabled,
                        "style": style,
                        "position": position,
                        "lyric": lyric,
                        "next_lyric": next_lyric,
                        "color": color,
                        "is_dark": is_dark,
                        "accent_rgb": accent_rgb,
                        "playing": snapshot.playing,
                        "title": title,
                        "artist": artist,
                        "cover_path": cover_path,
                        "position_ms": position_ms,
                        "duration_ms": duration_ms,
                        "updated_at_ms": updated_at_ms,
                        "provider": provider,
                        "pid": pid,
                    })
                    .to_string();
                    (Some(json), snapshot.playing)
                } else {
                    (None, snapshot.playing)
                }
            });
            last_playing = playing_now;
            if let Some(state) = payload_opt {
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
            if tick.is_multiple_of(5) {
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

#[cfg(target_os = "linux")]
fn graphical_session_preflight() -> Result<(), String> {
    use std::os::unix::net::UnixStream;

    if let Some(display) = std::env::var_os("WAYLAND_DISPLAY") {
        let display = display.to_string_lossy();
        let runtime = std::env::var_os("XDG_RUNTIME_DIR")
            .map(std::path::PathBuf::from)
            .ok_or_else(|| "XDG_RUNTIME_DIR 未设置，无法连接 Wayland compositor".to_string())?;
        let socket = if display.starts_with('/') {
            std::path::PathBuf::from(display.as_ref())
        } else {
            runtime.join(display.as_ref())
        };
        if UnixStream::connect(&socket).is_ok() {
            return Ok(());
        }
        return Err(format!(
            "Wayland compositor 不可用（{}）；请在 GNOME/Wayland 会话中启动流光音乐 (Lumina)",
            socket.display()
        ));
    }

    if let Some(display) = std::env::var_os("DISPLAY") {
        let display = display.to_string_lossy();
        if let Some(number) = display
            .strip_prefix(':')
            .and_then(|value| value.split('.').next())
        {
            let socket = std::path::PathBuf::from(format!("/tmp/.X11-unix/X{number}"));
            if UnixStream::connect(&socket).is_ok() {
                return Ok(());
            }
            return Err(format!(
                "X11 server 不可用（{}）；请启动 X11 或 Wayland 会话",
                socket.display()
            ));
        }
        return Ok(());
    }

    Err("未检测到 WAYLAND_DISPLAY 或 DISPLAY；无法创建图形窗口".to_string())
}

#[cfg(target_os = "linux")]
fn desktop_integration_available() -> bool {
    // Headless CI/containers often expose no user D-Bus session. Starting ksni
    // there causes an avoidable panic in its worker thread; the main window and
    // audio engine remain useful without tray/MPRIS integration.
    if std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some() {
        return true;
    }
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .is_some_and(|runtime| runtime.join("bus").exists())
}

#[cfg(not(target_os = "linux"))]
fn desktop_integration_available() -> bool {
    true
}

fn main() {
    #[cfg(target_os = "linux")]
    if let Err(error) = graphical_session_preflight() {
        eprintln!("流光音乐 (Lumina) 启动失败：{error}");
        return;
    }

    #[cfg(target_os = "linux")]
    let (_instance_guard, ipc_rx) = match acquire_single_instance() {
        InstanceLock::ForwardedToExisting => return,
        InstanceLock::Primary(guard, rx) => (Some(guard), Some(rx)),
        InstanceLock::Bypassed => (None, None),
    };

    #[cfg(target_os = "linux")]
    std::thread::spawn(sync_local_gnome_extension);

    gpui_platform::application()
        .with_assets(ui::icons::Assets)
        .with_quit_mode(gpui::QuitMode::Explicit)
        .run(move |cx: &mut App| {
            #[allow(clippy::redundant_closure)]
            let app = cx.new(|cx| app::Root::new(cx));
            let options = main_window_options(cx);
            cx.open_window(options, |_window, _cx| app.clone())
                .expect("打开主窗口失败");

            #[cfg(target_os = "linux")]
            if let Some(rx) = ipc_rx {
                start_ipc_service(rx, app.clone(), cx);
            }

            #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
            if desktop_integration_available() {
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
                if desktop_integration_available() {
                    start_mpris_service(mpris::start(), app.clone(), cx);
                    start_panel_lyrics_service(app.clone(), cx);
                }
            }

            cx.activate(true);
        });
}
