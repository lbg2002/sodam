//! Player Experience 3.0 的轻状态与 Root 扩展。
//!
//! 尽量把跨页面能力集中在这里，避免继续膨胀 `app.rs`。

use crate::app::Root;
use gpui::{ClickEvent, Context};
use sodam_core::models::TrackItem;
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

#[derive(Default)]
struct SelectionState {
    ids: HashSet<String>,
    anchor: Option<usize>,
}

fn selection() -> &'static Mutex<SelectionState> {
    static STATE: OnceLock<Mutex<SelectionState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(SelectionState::default()))
}

#[derive(Clone)]
struct ToastState {
    message: String,
    until: Instant,
}

fn toast_slot() -> &'static Mutex<Option<ToastState>> {
    static STATE: OnceLock<Mutex<Option<ToastState>>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(None))
}

pub fn toast_message() -> Option<String> {
    let mut slot = toast_slot().lock().ok()?;
    if slot.as_ref().is_some_and(|toast| Instant::now() > toast.until) {
        *slot = None;
    }
    slot.as_ref().map(|toast| toast.message.clone())
}

pub fn set_toast(message: impl Into<String>) {
    if let Ok(mut slot) = toast_slot().lock() {
        *slot = Some(ToastState {
            message: message.into(),
            until: Instant::now() + Duration::from_secs(3),
        });
    }
}

pub fn selected_ids() -> HashSet<String> {
    selection()
        .lock()
        .map(|state| state.ids.clone())
        .unwrap_or_default()
}

pub fn selected_count() -> usize {
    selection().lock().map(|state| state.ids.len()).unwrap_or(0)
}

pub fn is_selected(id: &str) -> bool {
    selection()
        .lock()
        .map(|state| state.ids.contains(id))
        .unwrap_or(false)
}

pub fn clear_selection() {
    if let Ok(mut state) = selection().lock() {
        state.ids.clear();
        state.anchor = None;
    }
}

/// Ctrl/Cmd 点击切换单项；Shift 点击选择锚点到当前项；普通点击在已有多选时清空并选当前。
pub fn select_click(event: &ClickEvent, tracks: &[TrackItem], index: usize) -> bool {
    let Some(track) = tracks.get(index) else { return false };
    let modifiers = event.modifiers();
    let multi = modifiers.control || modifiers.platform;
    let shift = modifiers.shift;
    let Ok(mut state) = selection().lock() else { return false };

    if shift {
        let anchor = state.anchor.unwrap_or(index);
        let (start, end) = if anchor <= index { (anchor, index) } else { (index, anchor) };
        if !multi {
            state.ids.clear();
        }
        for item in tracks.iter().take(end + 1).skip(start) {
            state.ids.insert(item.id.clone());
        }
        state.anchor = Some(anchor);
        return true;
    }

    if multi {
        if !state.ids.insert(track.id.clone()) {
            state.ids.remove(&track.id);
        }
        state.anchor = Some(index);
        return true;
    }

    if !state.ids.is_empty() {
        state.ids.clear();
        state.ids.insert(track.id.clone());
        state.anchor = Some(index);
        return true;
    }
    false
}

pub fn selected_tracks(tracks: &[TrackItem]) -> Vec<TrackItem> {
    let ids = selected_ids();
    tracks
        .iter()
        .filter(|track| ids.contains(&track.id))
        .cloned()
        .collect()
}

impl Root {
    pub fn toast(&mut self, message: impl Into<String>, cx: &mut Context<Self>) {
        let message = message.into();
        self.status = message.clone();
        set_toast(message);
        cx.notify();
        cx.spawn(async move |_this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(3100))
                .await;
            cx.notify();
        })
        .detach();
    }

    pub fn e3_clear_selection(&mut self, cx: &mut Context<Self>) {
        clear_selection();
        cx.notify();
    }

    pub fn e3_download_selected(
        &mut self,
        tracks: std::sync::Arc<Vec<TrackItem>>,
        cx: &mut Context<Self>,
    ) {
        let selected = selected_tracks(&tracks);
        if selected.is_empty() {
            self.toast(self.tr("请先选择歌曲"), cx);
            return;
        }
        self.queue_batch_download(std::sync::Arc::new(selected), cx);
        clear_selection();
        self.toast(self.tr("已加入批量下载"), cx);
    }

    pub fn e3_play_next_selected(
        &mut self,
        tracks: std::sync::Arc<Vec<TrackItem>>,
        cx: &mut Context<Self>,
    ) {
        let selected = selected_tracks(&tracks);
        if selected.is_empty() {
            self.toast(self.tr("请先选择歌曲"), cx);
            return;
        }
        let should_start = self.queue.insert_next(selected);
        self.sync_queue_cache();
        if should_start {
            self.start_track(cx);
        }
        clear_selection();
        self.toast(self.tr("已加入下一首播放"), cx);
    }

    pub fn e3_append_selected(
        &mut self,
        tracks: std::sync::Arc<Vec<TrackItem>>,
        cx: &mut Context<Self>,
    ) {
        let selected = selected_tracks(&tracks);
        if selected.is_empty() {
            self.toast(self.tr("请先选择歌曲"), cx);
            return;
        }
        let count = self.queue.append(selected);
        self.sync_queue_cache();
        clear_selection();
        self.toast(format!("已加入队列：{count} 首"), cx);
    }

    pub fn e3_delete_selected_downloads(
        &mut self,
        tracks: std::sync::Arc<Vec<TrackItem>>,
        cx: &mut Context<Self>,
    ) {
        let selected = selected_tracks(&tracks);
        if selected.is_empty() {
            self.toast(self.tr("请先选择歌曲"), cx);
            return;
        }
        let mut count = 0usize;
        for track in selected {
            if self.downloaded_ids.contains(&track.id) {
                self.delete_download(track.id.clone(), track.title.clone(), cx);
                count += 1;
            }
        }
        clear_selection();
        self.toast(format!("已请求删除 {count} 首本地下载"), cx);
    }

    pub fn e3_save_queue_snapshot_auto(&mut self, cx: &mut Context<Self>) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|value| value.as_secs())
            .unwrap_or(0);
        let name = format!("队列快照 {timestamp}");
        self.e3_save_queue_snapshot(&name, cx);
    }

    pub fn e3_save_queue_snapshot(&mut self, name: &str, cx: &mut Context<Self>) {
        match sodam_core::local_playlists::save_queue(name, self.queue.tracks()) {
            Ok(_) => self.toast(self.tr("已保存本地播放列表"), cx),
            Err(err) => self.toast(format!("保存失败：{err}"), cx),
        }
    }

    pub fn e3_play_local_playlist(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(item) = sodam_core::local_playlists::load()
            .into_iter()
            .find(|item| item.id == id)
        else {
            self.toast(self.tr("本地播放列表不存在"), cx);
            return;
        };
        let tracks = std::sync::Arc::new(item.tracks);
        if tracks.is_empty() {
            return;
        }
        self.queue_origin = crate::app::QueueOrigin::LocalPlaylist(id.to_string());
        self.play_from_arc(tracks, 0, cx);
    }

    pub fn e3_delete_local_playlist(&mut self, id: &str, cx: &mut Context<Self>) {
        match sodam_core::local_playlists::delete(id) {
            Ok(_) => self.toast(self.tr("已删除本地播放列表"), cx),
            Err(err) => self.toast(format!("删除失败：{err}"), cx),
        }
    }

    pub fn e3_set_audio_device(&mut self, id: String, cx: &mut Context<Self>) {
        match crate::system_audio::set_output_device(&id) {
            Ok(()) => {
                self.settings.audio_output_device = id;
                let _ = self.settings.save();
                self.toast(self.tr("音频输出设备已切换"), cx);
            }
            Err(err) => self.toast(format!("切换音频设备失败：{err}"), cx),
        }
    }

    pub fn e3_set_notifications(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.settings.system_notifications = enabled;
        let _ = self.settings.save();
        self.toast(
            if enabled { self.tr("切歌通知已开启") } else { self.tr("切歌通知已关闭") },
            cx,
        );
    }

    pub fn e3_set_normalize_volume(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.settings.normalize_volume = enabled;
        if !enabled {
            self.engine.set_gain(1.0);
        }
        let _ = self.settings.save();
        self.toast(
            if enabled { self.tr("响度标准化已开启") } else { self.tr("响度标准化已关闭") },
            cx,
        );
    }

    pub fn e3_set_gapless(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.settings.gapless_playback = enabled;
        self.engine
            .set_transition(enabled, self.settings.crossfade_seconds);
        let _ = self.settings.save();
        self.toast(
            if enabled { self.tr("无缝衔接已开启") } else { self.tr("无缝衔接已关闭") },
            cx,
        );
    }

    pub fn e3_set_crossfade(&mut self, seconds: u32, cx: &mut Context<Self>) {
        self.settings.crossfade_seconds = seconds.min(8);
        self.engine.set_transition(
            self.settings.gapless_playback,
            self.settings.crossfade_seconds,
        );
        let _ = self.settings.save();
        self.toast(
            if seconds == 0 {
                self.tr("交叉淡化已关闭").to_string()
            } else {
                format!("交叉淡化：{} 秒", seconds.min(8))
            },
            cx,
        );
    }

    pub fn e3_set_desktop_single_line(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.settings.desktop_lyrics_single_line = enabled;
        let _ = self.settings.save();
        self.toast(
            if enabled {
                self.tr("桌面歌词已切换为单行")
            } else {
                self.tr("桌面歌词已切换为双行")
            },
            cx,
        );
    }

    pub fn e3_set_desktop_always_on_top(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.settings.desktop_lyrics_always_on_top = enabled;
        let _ = self.settings.save();
        self.toast(
            if enabled {
                self.tr("桌面歌词已设为置顶")
            } else {
                self.tr("桌面歌词已取消置顶")
            },
            cx,
        );
    }

    pub fn e3_set_desktop_locked(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.settings.desktop_lyrics_locked = enabled;
        let _ = self.settings.save();
        self.toast(
            if enabled {
                self.tr("桌面歌词位置已锁定")
            } else {
                self.tr("桌面歌词位置已解锁")
            },
            cx,
        );
    }

    pub fn e3_set_desktop_click_through(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.settings.desktop_lyrics_click_through = enabled;
        let _ = self.settings.save();
        self.toast(
            if enabled {
                self.tr("桌面歌词已启用鼠标穿透")
            } else {
                self.tr("桌面歌词已关闭鼠标穿透")
            },
            cx,
        );
    }

    pub fn e3_set_desktop_opacity(&mut self, opacity: u8, cx: &mut Context<Self>) {
        self.settings.desktop_lyrics_opacity = opacity.clamp(30, 100);
        let _ = self.settings.save();
        self.toast(
            format!("桌面歌词背景透明度：{}%", self.settings.desktop_lyrics_opacity),
            cx,
        );
    }

    pub fn e3_set_desktop_align(&mut self, align: &str, cx: &mut Context<Self>) {
        self.settings.desktop_lyrics_align = match align {
            "left" | "right" => align.to_string(),
            _ => "center".to_string(),
        };
        let _ = self.settings.save();
        self.toast(self.tr("桌面歌词对齐方式已保存"), cx);
    }

    pub fn e3_set_lazy_startup(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.settings.lazy_startup = enabled;
        let _ = self.settings.save();
        self.toast(
            if enabled {
                self.tr("启动性能优化已开启")
            } else {
                self.tr("启动性能优化已关闭")
            },
            cx,
        );
    }

    pub fn e3_set_compact_width(&mut self, width: u32, cx: &mut Context<Self>) {
        self.settings.player_bar_compact_width = width.clamp(720, 1200);
        let _ = self.settings.save();
        self.toast(
            format!("播放栏折叠阈值：{} px", self.settings.player_bar_compact_width),
            cx,
        );
    }
}
