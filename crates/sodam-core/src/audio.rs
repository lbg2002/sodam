//! 播放引擎：用 rodio 播放 libresoda 解密后的本地文件。
//!
//! 设计：`OutputStream` 在 rodio 0.20 里是 `!Send`，所以**音频对象全部留在专属线程**，
//! UI 侧只通过命令通道操作、通过 `Arc<Mutex<PlaybackSnapshot>>` 读状态。
//!
//! 没有可用音频设备时**不 panic**：把错误写进快照，界面照常能跑（用户可能没接耳机）。

use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

use crate::models::TrackItem;

/// 播放器状态快照（UI 每帧读这个，加锁时间极短）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PlaybackSnapshot {
    pub track_id: String,
    pub title: String,
    pub playing: bool,
    pub position_seconds: f64,
    pub duration_seconds: f64,
    pub volume: f32,
    /// 当前曲目**实际拉到的流**音质（例：`无损 871k`）。
    pub quality: String,
    /// 最近一次错误（设备不可用、解码失败…），UI 直接展示
    pub error: String,
    /// 当前曲目是否已播完（UI 据此自动切下一首）
    pub finished: bool,
    /// 播完序号：每播完一次 +1。UI 用它做「处理过了吗」的判断 ——
    /// 之前用 track_id 比较，遇到循环同一首或 id 重复就会漏切。
    pub finished_seq: u64,
}

impl PlaybackSnapshot {
    /// 播放进度比例（0.0~1.0），时长未知时返回 0。
    pub fn progress_fraction(&self) -> f32 {
        if self.duration_seconds <= 0.0 {
            return 0.0;
        }
        (self.position_seconds / self.duration_seconds).clamp(0.0, 1.0) as f32
    }

    /// 当前播放位置 `mm:ss`。
    pub fn position_label(&self) -> String {
        format_seconds(self.position_seconds)
    }

    /// 总时长 `mm:ss`。
    pub fn duration_label(&self) -> String {
        format_seconds(self.duration_seconds)
    }

    /// `mm:ss / mm:ss` 形式的进度文案。
    pub fn progress_label(&self) -> String {
        format!("{} / {}", self.position_label(), self.duration_label())
    }
}

enum Command {
    Load {
        track: Box<TrackItem>,
        path: PathBuf,
        /// 这次实际拉到的流音质（显示用）
        quality: String,
    },
    Toggle,
    Play,
    Pause,
    Stop,
    SetVolume(f32),
    /// 跳转到指定秒数
    Seek(f64),
}

/// 播放引擎句柄（可以在任意线程调用，内部只有发命令与读快照）。
pub struct PlaybackEngine {
    tx: Sender<Command>,
    snapshot: Arc<Mutex<PlaybackSnapshot>>,
}

impl Default for PlaybackEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PlaybackEngine {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        let snapshot = Arc::new(Mutex::new(PlaybackSnapshot {
            volume: 1.0,
            ..Default::default()
        }));
        let state = snapshot.clone();
        let spawned = std::thread::Builder::new()
            .name("sodam-audio".to_string())
            .spawn(move || audio_thread(rx, state));
        if let Err(err) = spawned {
            if let Ok(mut snap) = snapshot.lock() {
                snap.error = format!("音频线程启动失败：{err}");
            }
        }
        Self { tx, snapshot }
    }

    pub fn snapshot(&self) -> PlaybackSnapshot {
        self.snapshot
            .lock()
            .map(|snap| snap.clone())
            .unwrap_or_default()
    }

    /// 播放一个**已经下载并解密**到本地的文件。
    /// 装载并播放：`quality` 是这次实际拉到的流音质（显示用）。
    pub fn load(&self, track: TrackItem, path: PathBuf, quality: impl Into<String>) {
        let _ = self.tx.send(Command::Load {
            track: Box::new(track),
            path,
            quality: quality.into(),
        });
    }

    pub fn toggle(&self) {
        let _ = self.tx.send(Command::Toggle);
    }

    pub fn play(&self) {
        let _ = self.tx.send(Command::Play);
    }

    pub fn pause(&self) {
        let _ = self.tx.send(Command::Pause);
    }

    pub fn stop(&self) {
        let _ = self.tx.send(Command::Stop);
    }

    pub fn set_volume(&self, volume: f32) {
        let _ = self.tx.send(Command::SetVolume(volume.clamp(0.0, 1.0)));
    }

    /// 跳转到指定秒数（越界会被引擎裁剪到 [0, 时长]）。
    pub fn seek(&self, seconds: f64) {
        let _ = self.tx.send(Command::Seek(seconds.max(0.0)));
    }
}

fn set_error(state: &Arc<Mutex<PlaybackSnapshot>>, message: String) {
    if let Ok(mut snap) = state.lock() {
        snap.error = message;
        snap.playing = false;
    }
}

fn sync_progress(state: &Arc<Mutex<PlaybackSnapshot>>, player: &rodio::Player) {
    if let Ok(mut snap) = state.lock() {
        snap.position_seconds = player.get_pos().as_secs_f64();
        snap.playing = !player.is_paused() && !player.empty();
    }
}

fn audio_thread(rx: Receiver<Command>, state: Arc<Mutex<PlaybackSnapshot>>) {
    // 打不开输出设备就把错误记在快照里，后续命令照常消费（界面不崩）
    let output = match rodio::DeviceSinkBuilder::open_default_sink() {
        Ok(device) => Some(device),
        Err(err) => {
            set_error(&state, format!("音频设备不可用：{err}"));
            None
        }
    };

    let mut player: Option<rodio::Player> = None;
    // 当前曲目是否已经上报过「播完」；换曲时重置
    let mut finished_reported = false;
    loop {
        // 为什么要超时 recv：进度必须**持续**更新（UI 的进度条依赖它）。
        // 之前只在 play/pause 时同步一次，所以进度条永远停在 0。
        let command = match rx.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok(command) => Some(command),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => None,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        };
        let Some(command) = command else {
            if let Some(current) = &player {
                sync_progress(&state, current);
                // 播完：置 finished，UI 会据此自动切下一首
                // 注意：sync_progress 会把 playing 置 false，所以这里不能用 playing 判断，
                // 必须用本地的 finished_reported 标志（否则自动下一首永远不触发）。
                if current.empty() && !finished_reported {
                    finished_reported = true;
                    if let Ok(mut snap) = state.lock() {
                        snap.playing = false;
                        snap.finished = true;
                        snap.finished_seq = snap.finished_seq.wrapping_add(1);
                    }
                }
            }
            continue;
        };
        match command {
            Command::Load {
                track,
                path,
                quality,
            } => {
                if let Some(previous) = player.take() {
                    previous.stop();
                }
                let Some(device) = &output else {
                    set_error(&state, "音频设备不可用，无法播放".to_string());
                    continue;
                };
                let file = match std::fs::File::open(&path) {
                    Ok(file) => file,
                    Err(err) => {
                        set_error(&state, format!("打开音频文件失败：{err}"));
                        continue;
                    }
                };
                let decoder = match rodio::Decoder::new(std::io::BufReader::new(file)) {
                    Ok(decoder) => decoder,
                    Err(err) => {
                        set_error(&state, format!("解码失败（文件可能未解密）：{err}"));
                        continue;
                    }
                };
                let new_player = rodio::Player::connect_new(device.mixer());
                let volume = state.lock().map(|snap| snap.volume).unwrap_or(1.0);
                new_player.set_volume(volume);
                new_player.append(decoder);
                new_player.play();
                if let Ok(mut snap) = state.lock() {
                    snap.track_id = track.id.clone();
                    snap.title = track.title.clone();
                    snap.playing = true;
                    snap.position_seconds = 0.0;
                    snap.duration_seconds = track.duration_seconds.max(0) as f64;
                    snap.quality = quality.clone();
                    snap.error.clear();
                    snap.finished = false;
                    finished_reported = false;
                }
                player = Some(new_player);
            }
            Command::Toggle => {
                if let Some(current) = &player {
                    if current.is_paused() {
                        current.play();
                    } else {
                        current.pause();
                    }
                    sync_progress(&state, current);
                } else {
                    set_error(&state, "还没有可播放的曲目".to_string());
                }
            }
            Command::Play => {
                if let Some(current) = &player {
                    current.play();
                    sync_progress(&state, current);
                }
            }
            Command::Pause => {
                if let Some(current) = &player {
                    current.pause();
                    sync_progress(&state, current);
                }
            }
            Command::Stop => {
                if let Some(current) = player.take() {
                    current.stop();
                }
                if let Ok(mut snap) = state.lock() {
                    snap.playing = false;
                    snap.position_seconds = 0.0;
                }
            }
            Command::Seek(seconds) => {
                if let Some(current) = &player {
                    if current
                        .try_seek(std::time::Duration::from_secs_f64(seconds))
                        .is_ok()
                    {
                        // 跳转后进度要立刻反映出来（避免界面回跳）
                        if let Ok(mut snap) = state.lock() {
                            snap.position_seconds = seconds;
                        }
                    }
                }
            }
            Command::SetVolume(volume) => {
                if let Some(current) = &player {
                    current.set_volume(volume);
                }
                if let Ok(mut snap) = state.lock() {
                    snap.volume = volume;
                }
            }
        }
        if let Some(current) = &player {
            sync_progress(&state, current);
        }
    }
}

/// 秒 → `mm:ss`。
fn format_seconds(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    format!("{:02}:{:02}", total / 60, total % 60)
}

/// 音频缓存目录。
pub fn cache_dir() -> std::path::PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("sodam")
        .join("audio")
}

/// 从缓存文件名中提取曲目 id。
///
/// 正常文件名为 `<track-id>-<quality>.m4a`。优先匹配已知音质后缀，
/// 兼容 track id 自身包含 `-` 的情况。
fn cache_track_id(path: &std::path::Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    for tag in ["lossless", "highest", "medium", "low", "best", "auto"] {
        let suffix = format!("-{tag}");
        if let Some(id) = stem.strip_suffix(&suffix) {
            if !id.is_empty() {
                return Some(id.to_string());
            }
        }
    }
    stem.rsplit_once('-')
        .map(|(id, _)| id.to_string())
        .filter(|id| !id.is_empty())
}

fn access_path(audio_path: &std::path::Path) -> std::path::PathBuf {
    audio_path.with_extension("access")
}

/// 标记一个播放缓存刚刚被实际使用。
///
/// 不修改音频文件本身，避免播放器打开文件时额外触碰内容；LRU 时间单独放在
/// 很小的 `.access` 边车里。
pub fn touch_audio_cache(audio_path: &std::path::Path) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0);
    let _ = std::fs::write(access_path(audio_path), now.to_string());
}

/// 当前播放音质档位下的有效缓存 id。
///
/// 同时检查 `.quality` 边车记录的字节数，避免截断文件被 UI 标成“已准备”。
pub fn cached_audio_ids_for_quality(quality: &str) -> std::collections::HashSet<String> {
    let tag = {
        let value = quality.trim().to_ascii_lowercase();
        if value.is_empty() {
            "auto".to_string()
        } else {
            value
        }
    };
    let suffix = format!("-{tag}.m4a");
    let Ok(entries) = std::fs::read_dir(cache_dir()) else {
        return std::collections::HashSet::new();
    };

    entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_str()?;
            let track_id = name.strip_suffix(&suffix)?;
            if track_id.is_empty() {
                return None;
            }
            let actual = std::fs::metadata(&path).ok()?.len();
            if actual == 0 {
                return None;
            }
            let quality_path = path.with_extension("quality");
            let text = std::fs::read_to_string(quality_path).ok()?;
            let mut fields = text.split('\t');
            let label = fields.next()?.trim();
            let expected = fields.next()?.trim().parse::<u64>().ok()?;
            (expected == actual && !label.is_empty()).then(|| track_id.to_string())
        })
        .collect()
}

/// 当前所有完整音频缓存对应的曲目 id。
///
/// 这是同步扫盘函数，UI 应在后台线程调用后保存快照。
pub fn cached_audio_ids() -> std::collections::HashSet<String> {
    let Ok(entries) = std::fs::read_dir(cache_dir()) else {
        return std::collections::HashSet::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            (path.extension().and_then(|ext| ext.to_str()) == Some("m4a"))
                .then(|| cache_track_id(&path))
                .flatten()
        })
        .collect()
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CacheTrimResult {
    pub removed_assets: usize,
    pub removed_bytes: u64,
    pub remaining_bytes: u64,
}

#[derive(Debug)]
struct CacheAsset {
    audio: std::path::PathBuf,
    quality: std::path::PathBuf,
    access: std::path::PathBuf,
    track_id: String,
    bytes: u64,
    last_used: u64,
}

fn file_bytes(path: &std::path::Path) -> u64 {
    std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0)
}

fn asset_last_used(audio: &std::path::Path, access: &std::path::Path) -> u64 {
    std::fs::read_to_string(access)
        .ok()
        .and_then(|text| text.trim().parse::<u64>().ok())
        .or_else(|| {
            std::fs::metadata(audio)
                .ok()?
                .modified()
                .ok()?
                .duration_since(std::time::UNIX_EPOCH)
                .ok()
                .map(|value| value.as_secs())
        })
        .unwrap_or(0)
}

/// 按最近使用顺序把**播放缓存**裁剪到给定上限。
///
/// 下载目录与播放缓存完全分离，所以这里不会删除用户主动下载的歌曲。
/// `protected_ids` 用于保护正在播放、加载或预取的曲目。
pub fn trim_audio_cache(
    max_bytes: u64,
    protected_ids: &std::collections::HashSet<String>,
) -> CacheTrimResult {
    if max_bytes == 0 {
        let (bytes, _, _, _) = cache_stats();
        return CacheTrimResult {
            remaining_bytes: bytes,
            ..Default::default()
        };
    }

    let dir = cache_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return CacheTrimResult::default();
    };

    let mut assets = Vec::new();
    let mut total = 0u64;
    for entry in entries.flatten() {
        let audio = entry.path();
        if audio.extension().and_then(|ext| ext.to_str()) != Some("m4a") {
            continue;
        }
        let Some(track_id) = cache_track_id(&audio) else {
            continue;
        };
        let quality = audio.with_extension("quality");
        let access = access_path(&audio);
        let bytes = file_bytes(&audio) + file_bytes(&quality) + file_bytes(&access);
        total = total.saturating_add(bytes);
        assets.push(CacheAsset {
            last_used: asset_last_used(&audio, &access),
            audio,
            quality,
            access,
            track_id,
            bytes,
        });
    }

    if total <= max_bytes {
        return CacheTrimResult {
            remaining_bytes: total,
            ..Default::default()
        };
    }

    assets.sort_by_key(|asset| asset.last_used);
    let mut result = CacheTrimResult {
        remaining_bytes: total,
        ..Default::default()
    };
    for asset in assets {
        if result.remaining_bytes <= max_bytes {
            break;
        }
        if protected_ids.contains(&asset.track_id) {
            continue;
        }

        let removed_audio = std::fs::remove_file(&asset.audio).is_ok();
        let _ = std::fs::remove_file(&asset.quality);
        let _ = std::fs::remove_file(&asset.access);
        if removed_audio {
            result.removed_assets += 1;
            result.removed_bytes = result.removed_bytes.saturating_add(asset.bytes);
            result.remaining_bytes = result.remaining_bytes.saturating_sub(asset.bytes);
        }
    }
    result
}

/// 封面缓存目录。
pub fn cover_cache_dir() -> std::path::PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("sodam")
        .join("covers")
}

/// 目录占用：**递归**统计（字节数，文件数）。
///
/// 封面缩略图在 `covers/`，原图在 `covers/original/` 子目录 ——
/// 只看顶层会把整个子目录漏掉（实测踩过：字节/张数都偏小）。
fn dir_usage(dir: &std::path::Path) -> (u64, usize) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return (0, 0);
    };
    let mut bytes = 0u64;
    let mut files = 0usize;
    for entry in entries.flatten() {
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if meta.is_file() {
            bytes += meta.len();
            files += 1;
        } else if meta.is_dir() {
            let (sub_bytes, sub_files) = dir_usage(&entry.path());
            bytes += sub_bytes;
            files += sub_files;
        }
    }
    (bytes, files)
}

/// 顶层目录里指定扩展名的文件数（用于「歌曲数」：只数 `.m4a`）。
fn count_extension(dir: &std::path::Path, extension: &str) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|entry| entry.path().extension().map(|ext| ext == extension) == Some(true))
        .count()
}

/// 缓存统计：(音频字节, 歌曲数, 封面字节, 封面张数)。
///
/// * 字节数含**全部**文件（`.m4a` / `.quality` 边车 / 下载中的 `.part` /
///   封面原图子目录）；
/// * 「歌曲数」只数 `.m4a`：边车和临时文件不是歌，全算会把数量翻倍（踩过）；
/// * 「封面张数」数全部图片缓存文件（含原图）。
///
/// 注意：这是同步扫盘，别在渲染路径里每帧调 —— 调用方应后台执行并缓存结果。
pub fn cache_stats() -> (u64, usize, u64, usize) {
    let (audio_bytes, _) = dir_usage(&cache_dir());
    let songs = count_extension(&cache_dir(), "m4a");
    let (cover_bytes, cover_files) = dir_usage(&cover_cache_dir());
    (audio_bytes, songs, cover_bytes, cover_files)
}

/// 清空缓存（音频 + 封面），返回删除的文件数。
pub fn clear_cache() -> usize {
    let mut removed = 0usize;
    for dir in [cache_dir(), cover_cache_dir()] {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && std::fs::remove_file(&path).is_ok() {
                removed += 1;
            }
        }
    }
    removed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_label_formats() {
        let snap = PlaybackSnapshot {
            position_seconds: 65.4,
            duration_seconds: 236.6,
            ..Default::default()
        };
        assert_eq!(snap.progress_label(), "01:05 / 03:56");
    }

    #[test]
    fn engine_starts_without_audio_device_and_reports_state() {
        // 没有音频设备时也应该能创建引擎（错误写进快照，不 panic）
        let engine = PlaybackEngine::new();
        let snap = engine.snapshot();
        assert_eq!(snap.volume, 1.0);
        assert!(!snap.playing);
    }

    #[test]
    #[test]
    fn cache_track_id_handles_quality_suffixes() {
        assert_eq!(
            cache_track_id(std::path::Path::new("123-lossless.m4a")).as_deref(),
            Some("123")
        );
        assert_eq!(
            cache_track_id(std::path::Path::new("abc-def-highest.m4a")).as_deref(),
            Some("abc-def")
        );
    }

    fn dir_usage_and_count_extension_classify_files_correctly() {
        let dir = std::env::temp_dir().join(format!("sodam-cache-stats-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("original")).expect("mkdir");
        std::fs::write(dir.join("a.m4a"), [0u8; 100]).expect("m4a");
        std::fs::write(dir.join("a.quality"), b"lossless\t100").expect("sidecar");
        std::fs::write(dir.join("b.part"), [0u8; 50]).expect("part");
        std::fs::write(dir.join("original/full.img"), [0u8; 25]).expect("cover");

        // 字节：递归含子目录，全部文件都算（100 + 12 + 50 + 25）
        let (bytes, files) = dir_usage(&dir);
        assert_eq!(bytes, 187, "应含子目录与边车/part");
        assert_eq!(files, 4);
        // 歌曲数：只数 .m4a（边车/part 不算歌）
        assert_eq!(count_extension(&dir, "m4a"), 1);
        assert_eq!(count_extension(&dir, "img"), 0, "只看顶层");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
