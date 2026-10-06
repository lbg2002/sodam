//! 本地下载管理：只导出 SodaM 已经成功播放并落盘的音频缓存。
//!
//! 这里不发起新的汽水取流请求，也不处理平台权限；播放缓存不存在时返回 `Ok(None)`，
//! 由 UI 保持“待下载”状态，等正常播放产生缓存后再重试。

use crate::{audio, models::TrackItem};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
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

pub fn download_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("SODAM_DOWNLOAD_DIR") {
        return PathBuf::from(path);
    }
    dirs::audio_dir()
        .map(|path| path.join("SodaM Downloads"))
        .or_else(|| dirs::home_dir().map(|path| path.join("Music").join("SodaM Downloads")))
        .unwrap_or_else(|| PathBuf::from("SodaM Downloads"))
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

/// 导出当前已经存在的播放缓存。
///
/// - `Ok(Some(item))`：导出成功；
/// - `Ok(None)`：还没有本地播放缓存，调用方可保持“待下载”；
/// - `Err`：本地复制 / 索引写入失败。
pub fn export_cached_track(
    track: &TrackItem,
    quality_preference: &str,
) -> Result<Option<DownloadedTrack>> {
    let assets = find_cached_assets(&track.id)?;
    let Some(asset) = pick_asset(&assets, quality_preference) else {
        return Ok(None);
    };

    let dir = ensure_download_dir()?;
    let extension = asset
        .path
        .extension()
        .and_then(|ext| ext.to_str())
        .filter(|ext| !ext.is_empty())
        .unwrap_or("m4a");
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
    let part = dir.join(format!(".{stem}.{}.part", std::process::id()));

    fs::copy(&asset.path, &part)
        .with_context(|| format!("复制播放缓存失败：{}", asset.path.display()))?;
    let written = fs::metadata(&part).map(|meta| meta.len()).unwrap_or(0);
    if written == 0 {
        let _ = fs::remove_file(&part);
        anyhow::bail!("播放缓存为空");
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

fn pick_asset<'a>(assets: &'a [CachedAsset], quality_preference: &str) -> Option<&'a CachedAsset> {
    if assets.is_empty() {
        return None;
    }
    let preferred = match quality_preference.trim().to_ascii_lowercase().as_str() {
        "" | "auto" | "best" => None,
        value => Some(value.to_string()),
    };
    if let Some(preferred) = preferred {
        if let Some(asset) = assets
            .iter()
            .find(|asset| asset.tag.eq_ignore_ascii_case(&preferred))
        {
            return Some(asset);
        }
    }
    // 多档缓存同时存在时优先使用体积最大的实际播放资产，通常对应更高音质。
    assets.iter().max_by_key(|asset| asset.bytes)
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
