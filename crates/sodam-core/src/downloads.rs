//! 本地下载管理：只导出 SodaM 已经成功播放并落盘的音频缓存。
//!
//! 这里不发起新的汽水取流请求，也不处理平台权限；播放缓存不存在时返回 `Ok(None)`，
//! 由 UI 保持“待下载”状态，等正常播放产生缓存后再重试。

use crate::{audio, config::Settings, models::TrackItem};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DownloadedTrack {
    pub track_id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub cover: String,
    pub quality: String,
    pub path: PathBuf,
    pub bytes: u64,
    pub downloaded_at: u64,
}

#[derive(Debug, Clone)]
struct CachedAsset {
    path: PathBuf,
    quality: String,
    tag: String,
    bytes: u64,
}

pub fn default_download_dir() -> PathBuf {
    dirs::audio_dir()
        .map(|path| path.join("SodaM Downloads"))
        .or_else(|| dirs::home_dir().map(|path| path.join("Music").join("SodaM Downloads")))
        .unwrap_or_else(|| PathBuf::from("SodaM Downloads"))
}

pub fn download_dir_for(configured: &str) -> PathBuf {
    if let Some(path) = std::env::var_os("SODAM_DOWNLOAD_DIR") {
        return PathBuf::from(path);
    }
    let configured = configured.trim();
    if configured.is_empty() {
        default_download_dir()
    } else {
        PathBuf::from(configured)
    }
}

pub fn download_dir() -> PathBuf {
    let settings = Settings::load();
    download_dir_for(&settings.download_dir)
}

fn metadata_dir() -> PathBuf {
    download_dir().join(".sodam")
}

fn pending_path() -> PathBuf {
    metadata_dir().join("pending.json")
}

pub fn ensure_download_dir() -> Result<PathBuf> {
    let dir = download_dir();
    fs::create_dir_all(&dir).context("创建下载目录失败")?;
    fs::create_dir_all(metadata_dir()).context("创建下载索引目录失败")?;
    Ok(dir)
}

pub fn load_pending_downloads() -> Result<Vec<TrackItem>> {
    let path = pending_path();
    let Ok(text) = fs::read_to_string(&path) else {
        return Ok(Vec::new());
    };
    let tracks = serde_json::from_str::<Vec<TrackItem>>(&text)
        .with_context(|| format!("读取待下载索引失败：{}", path.display()))?;
    Ok(tracks)
}

pub fn save_pending_downloads(tracks: &[TrackItem]) -> Result<()> {
    ensure_download_dir()?;
    let path = pending_path();
    if tracks.is_empty() {
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(err).context("清理待下载索引失败"),
        }
        return Ok(());
    }
    let part = path.with_extension("json.part");
    fs::write(&part, serde_json::to_vec_pretty(tracks)?).context("写待下载索引失败")?;
    if path.exists() {
        let _ = fs::remove_file(&path);
    }
    fs::rename(&part, &path).context("保存待下载索引失败")?;
    Ok(())
}

pub fn list_downloads() -> Result<Vec<DownloadedTrack>> {
    let meta_dir = metadata_dir();
    let Ok(entries) = fs::read_dir(&meta_dir) else {
        return Ok(Vec::new());
    };

    let mut items = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json")
            || path.file_name().and_then(|name| name.to_str()) == Some("pending.json")
        {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(mut item) = serde_json::from_str::<DownloadedTrack>(&text) else {
            continue;
        };
        let Ok(meta) = fs::metadata(&item.path) else {
            continue;
        };
        if !meta.is_file() || meta.len() == 0 {
            continue;
        }
        item.bytes = meta.len();
        items.push(item);
    }
    items.sort_by_key(|item| std::cmp::Reverse(item.downloaded_at));
    Ok(items)
}

pub fn downloaded_track(track_id: &str) -> Result<Option<DownloadedTrack>> {
    let path = metadata_path(track_id);
    let Ok(text) = fs::read_to_string(&path) else {
        return Ok(None);
    };
    let mut item = serde_json::from_str::<DownloadedTrack>(&text)
        .with_context(|| format!("读取下载索引失败：{}", path.display()))?;
    let Ok(meta) = fs::metadata(&item.path) else {
        return Ok(None);
    };
    if !meta.is_file() || meta.len() == 0 {
        return Ok(None);
    }
    item.bytes = meta.len();
    Ok(Some(item))
}

/// 导出当前已经存在的播放缓存。
///
/// - `Ok(Some(item))`：导出成功；
/// - `Ok(None)`：还没有本地播放缓存，调用方可保持“待下载”；
/// - `Err`：本地复制 / 索引写入失败。
pub fn export_cached_track(
    track: &TrackItem,
    quality_preference: &str,
    output_format: &str,
    cover_path: Option<&Path>,
) -> Result<Option<DownloadedTrack>> {
    let assets = find_cached_assets(&track.id)?;
    let Some(asset) = pick_asset(&assets, quality_preference) else {
        return Ok(None);
    };

    let dir = ensure_download_dir()?;
    let source_extension = asset
        .path
        .extension()
        .and_then(|ext| ext.to_str())
        .filter(|ext| !ext.is_empty())
        .unwrap_or("m4a");
    let format = match output_format.trim().to_ascii_lowercase().as_str() {
        "" | "source" => "source",
        "mp3" => "mp3",
        "flac" => "flac",
        other => anyhow::bail!("不支持的下载格式：{other}"),
    };
    let extension = if format == "source" {
        source_extension
    } else {
        format
    };
    let stem = safe_filename(&format!(
        "{}{}{} [{}]",
        track.artist,
        if track.artist.trim().is_empty() {
            ""
        } else {
            " - "
        },
        track.title,
        track.id
    ));
    let output = dir.join(format!("{stem}.{extension}"));
    let part = dir.join(format!(".{stem}.{}.tmp.{extension}", std::process::id()));

    write_audio_output(&asset.path, &part, format, track, cover_path)?;

    let written = fs::metadata(&part).map(|meta| meta.len()).unwrap_or(0);
    if written == 0 {
        let _ = fs::remove_file(&part);
        anyhow::bail!("导出后的音频文件为空");
    }
    if output.exists() {
        fs::remove_file(&output)
            .with_context(|| format!("替换旧下载失败：{}", output.display()))?;
    }
    fs::rename(&part, &output).with_context(|| format!("下载落盘失败：{}", output.display()))?;

    let downloaded_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0);
    let item = DownloadedTrack {
        track_id: track.id.clone(),
        title: track.title.clone(),
        artist: track.artist.clone(),
        album: track.album.clone(),
        cover: track.cover.clone(),
        quality: asset.quality.clone(),
        path: output,
        bytes: written,
        downloaded_at,
    };
    write_metadata(&item)?;
    Ok(Some(item))
}

fn write_audio_output(
    input: &Path,
    output: &Path,
    format: &str,
    track: &TrackItem,
    cover_path: Option<&Path>,
) -> Result<()> {
    if format == "source" {
        // 原始格式优先用 ffmpeg 无损 remux 写入 Tag；系统没有 ffmpeg 或容器不接受
        // 封面时自动退回纯复制，保证“原始格式”始终可用。
        if ffmpeg_export(input, output, format, track, cover_path).is_ok() {
            return Ok(());
        }
        let _ = fs::remove_file(output);
        fs::copy(input, output)
            .with_context(|| format!("复制播放缓存失败：{}", input.display()))?;
        return Ok(());
    }

    ffmpeg_export(input, output, format, track, cover_path)
}

fn ffmpeg_export(
    input: &Path,
    output: &Path,
    format: &str,
    track: &TrackItem,
    cover_path: Option<&Path>,
) -> Result<()> {
    let attempt = |cover: Option<&Path>| -> Result<()> {
        let mut command = Command::new("ffmpeg");
        command
            .arg("-hide_banner")
            .arg("-loglevel")
            .arg("error")
            .arg("-y")
            .arg("-i")
            .arg(input);

        if let Some(path) = cover {
            command.arg("-i").arg(path);
        }

        command.arg("-map").arg("0:a:0");
        if cover.is_some() {
            command.arg("-map").arg("1:v:0");
        }

        match format {
            "source" => {
                command.arg("-codec:a").arg("copy");
            }
            "mp3" => {
                command
                    .arg("-codec:a")
                    .arg("libmp3lame")
                    .arg("-q:a")
                    .arg("0")
                    .arg("-id3v2_version")
                    .arg("3");
            }
            "flac" => {
                command.arg("-codec:a").arg("flac");
            }
            other => anyhow::bail!("不支持的转码格式：{other}"),
        }

        if cover.is_some() {
            command
                .arg("-codec:v")
                .arg("copy")
                .arg("-disposition:v:0")
                .arg("attached_pic")
                .arg("-metadata:s:v")
                .arg("title=Album cover")
                .arg("-metadata:s:v")
                .arg("comment=Cover (front)");
        }

        command
            .arg("-metadata")
            .arg(format!("title={}", track.title))
            .arg("-metadata")
            .arg(format!("artist={}", track.artist))
            .arg("-metadata")
            .arg(format!("album={}", track.album));

        let result = command.arg(output).output().map_err(|err| {
            anyhow::anyhow!("无法启动 ffmpeg：{err}。MP3/FLAC 下载需要安装 ffmpeg")
        })?;
        if result.status.success() {
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&result.stderr).trim().to_string();
        let _ = fs::remove_file(output);
        anyhow::bail!(
            "ffmpeg 导出失败{}",
            if stderr.is_empty() {
                String::new()
            } else {
                format!("：{stderr}")
            }
        )
    };

    if cover_path.is_some() {
        if attempt(cover_path).is_ok() {
            return Ok(());
        }
        // 个别容器/封面编码不接受 attached_pic；保留文本 Tag 再试一次。
        return attempt(None);
    }
    attempt(None)
}

pub fn relocate_download_dir(old_dir: &Path, new_dir: &Path) -> Result<()> {
    if old_dir == new_dir {
        fs::create_dir_all(new_dir).context("创建下载目录失败")?;
        return Ok(());
    }
    if new_dir.starts_with(old_dir) {
        anyhow::bail!("新的下载目录不能位于当前下载目录内部");
    }
    fs::create_dir_all(new_dir).context("创建新的下载目录失败")?;
    if !old_dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(old_dir).context("读取旧下载目录失败")? {
        let entry = entry?;
        let source = entry.path();
        let target = new_dir.join(entry.file_name());
        if source.is_dir() {
            copy_dir_recursive(&source, &target)?;
            fs::remove_dir_all(&source).ok();
        } else {
            move_file_cross_device(&source, &target)?;
        }
    }
    fs::remove_dir(old_dir).ok();

    // 下载索引保存的是绝对路径；搬家后同步重写。
    let meta_dir = new_dir.join(".sodam");
    if let Ok(entries) = fs::read_dir(&meta_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json")
                || path.file_name().and_then(|name| name.to_str()) == Some("pending.json")
            {
                continue;
            }
            let Ok(text) = fs::read_to_string(&path) else {
                continue;
            };
            let Ok(mut item) = serde_json::from_str::<DownloadedTrack>(&text) else {
                continue;
            };
            if let Ok(relative) = item.path.strip_prefix(old_dir) {
                item.path = new_dir.join(relative);
                let _ = fs::write(&path, serde_json::to_vec_pretty(&item)?);
            }
        }
    }
    Ok(())
}

fn copy_dir_recursive(source: &Path, target: &Path) -> Result<()> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let from = entry.path();
        let to = target.join(entry.file_name());
        if from.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else {
            move_file_cross_device(&from, &to)?;
        }
    }
    Ok(())
}

fn move_file_cross_device(source: &Path, target: &Path) -> Result<()> {
    if target.exists() {
        fs::remove_file(target)
            .with_context(|| format!("替换目标文件失败：{}", target.display()))?;
    }
    match fs::rename(source, target) {
        Ok(()) => Ok(()),
        Err(_) => {
            fs::copy(source, target)
                .with_context(|| format!("复制到新下载目录失败：{}", target.display()))?;
            fs::remove_file(source)
                .with_context(|| format!("清理旧下载文件失败：{}", source.display()))?;
            Ok(())
        }
    }
}

pub fn delete_download(track_id: &str) -> Result<bool> {
    let meta_path = metadata_path(track_id);
    let item = fs::read_to_string(&meta_path)
        .ok()
        .and_then(|text| serde_json::from_str::<DownloadedTrack>(&text).ok());

    let mut removed = false;
    if let Some(item) = item {
        match fs::remove_file(&item.path) {
            Ok(()) => removed = true,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => {
                return Err(err)
                    .with_context(|| format!("删除下载文件失败：{}", item.path.display()))
            }
        }
    }
    match fs::remove_file(&meta_path) {
        Ok(()) => removed = true,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(err).context("删除下载索引失败"),
    }
    Ok(removed)
}

fn write_metadata(item: &DownloadedTrack) -> Result<()> {
    let dir = metadata_dir();
    fs::create_dir_all(&dir).context("创建下载索引目录失败")?;
    let path = metadata_path(&item.track_id);
    let part = path.with_extension("json.part");
    fs::write(&part, serde_json::to_vec_pretty(item)?).context("写下载索引失败")?;
    if path.exists() {
        let _ = fs::remove_file(&path);
    }
    fs::rename(&part, &path).context("保存下载索引失败")?;
    Ok(())
}

fn metadata_path(track_id: &str) -> PathBuf {
    metadata_dir().join(format!("{}.json", safe_component(track_id)))
}

fn find_cached_assets(track_id: &str) -> Result<Vec<CachedAsset>> {
    let cache_dir = audio::cache_dir();
    let Ok(entries) = fs::read_dir(&cache_dir) else {
        return Ok(Vec::new());
    };
    let prefix = format!("{track_id}-");
    let mut assets = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with(&prefix) || !name.ends_with(".m4a") {
            continue;
        }
        let tag = name
            .trim_start_matches(&prefix)
            .trim_end_matches(".m4a")
            .to_string();
        let bytes = fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
        if bytes == 0 {
            continue;
        }
        let sidecar = cache_dir.join(format!("{track_id}-{tag}.quality"));
        let quality = fs::read_to_string(&sidecar)
            .ok()
            .and_then(|text| text.split('\t').next().map(str::trim).map(str::to_string))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| tag.clone());
        assets.push(CachedAsset {
            path,
            quality,
            tag,
            bytes,
        });
    }
    Ok(assets)
}

fn quality_matches(asset: &CachedAsset, preferred: &str) -> bool {
    if asset.tag.eq_ignore_ascii_case(preferred) {
        return true;
    }
    let label = asset.quality.to_ascii_lowercase();
    match preferred {
        "lossless" => label.contains("无损") || label.contains("lossless"),
        "highest" => label.contains("极高"),
        "medium" => label.contains("较高"),
        "low" => label.contains("标准"),
        _ => false,
    }
}

fn pick_asset<'a>(assets: &'a [CachedAsset], quality_preference: &str) -> Option<&'a CachedAsset> {
    if assets.is_empty() {
        return None;
    }

    let preference = quality_preference.trim().to_ascii_lowercase();
    if let Some(playback_tag) = preference.strip_prefix("follow:") {
        let playback_tag = if playback_tag.is_empty() {
            "auto"
        } else {
            playback_tag
        };
        return assets
            .iter()
            .find(|asset| asset.tag.eq_ignore_ascii_case(playback_tag))
            .or_else(|| assets.iter().max_by_key(|asset| asset.bytes));
    }

    match preference.as_str() {
        "" | "auto" | "best" => assets.iter().max_by_key(|asset| asset.bytes),
        preferred => assets
            .iter()
            .find(|asset| quality_matches(asset, preferred)),
    }
}

fn safe_component(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
            output.push(ch);
        } else {
            output.push('_');
        }
    }
    if output.is_empty() {
        "track".to_string()
    } else {
        output
    }
}

fn safe_filename(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for ch in input.chars() {
        if matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || ch.is_control() {
            output.push('_');
        } else {
            output.push(ch);
        }
    }
    let trimmed = output.trim().trim_matches('.').trim();
    if trimmed.is_empty() {
        "track".to_string()
    } else {
        trimmed.chars().take(160).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filename_sanitizes_path_characters() {
        assert_eq!(safe_filename("a/b:c"), "a_b_c");
        assert_eq!(safe_component("12/34"), "12_34");
    }

    #[test]
    fn specific_quality_does_not_silently_fallback() {
        let assets = vec![CachedAsset {
            path: PathBuf::from("x.m4a"),
            quality: "极高".into(),
            tag: "highest".into(),
            bytes: 10,
        }];
        assert!(pick_asset(&assets, "lossless").is_none());
        assert_eq!(pick_asset(&assets, "highest").unwrap().tag, "highest");
        assert_eq!(
            pick_asset(&assets, "follow:highest").unwrap().tag,
            "highest"
        );
    }

    #[test]
    fn pending_tracks_are_json_serializable() {
        let track = TrackItem {
            id: "42".into(),
            title: "测试歌曲".into(),
            artist: "测试歌手".into(),
            ..Default::default()
        };
        let json = serde_json::to_string(&vec![track.clone()]).unwrap();
        let decoded: Vec<TrackItem> = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, vec![track]);
    }
}
