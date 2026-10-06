//! 本地播放列表 / 队列快照。

use crate::models::TrackItem;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

fn memory_cache() -> &'static Mutex<Option<Vec<LocalPlaylist>>> {
    static CACHE: OnceLock<Mutex<Option<Vec<LocalPlaylist>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocalPlaylist {
    pub id: String,
    pub name: String,
    pub tracks: Vec<TrackItem>,
    pub updated_at: u64,
}

fn playlists_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("sodam")
        .join("local-playlists.json")
}

pub fn load() -> Vec<LocalPlaylist> {
    if let Ok(cache) = memory_cache().lock() {
        if let Some(items) = cache.as_ref() {
            return items.clone();
        }
    }
    let items: Vec<LocalPlaylist> = fs::read_to_string(playlists_path())
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    if let Ok(mut cache) = memory_cache().lock() {
        *cache = Some(items.clone());
    }
    items
}

pub fn save_queue(name: &str, tracks: &[TrackItem]) -> Result<Vec<LocalPlaylist>> {
    let name = name.trim();
    if name.is_empty() {
        anyhow::bail!("播放列表名称不能为空");
    }
    if tracks.is_empty() {
        anyhow::bail!("当前队列为空");
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0);
    let mut playlists = load();
    if let Some(existing) = playlists.iter_mut().find(|item| item.name == name) {
        existing.tracks = tracks.to_vec();
        existing.updated_at = now;
    } else {
        playlists.insert(
            0,
            LocalPlaylist {
                id: format!("local-{now}-{}", playlists.len()),
                name: name.to_string(),
                tracks: tracks.to_vec(),
                updated_at: now,
            },
        );
    }
    playlists.sort_by_key(|item| std::cmp::Reverse(item.updated_at));
    save(&playlists)?;
    Ok(playlists)
}

pub fn delete(id: &str) -> Result<Vec<LocalPlaylist>> {
    let mut playlists = load();
    playlists.retain(|item| item.id != id);
    save(&playlists)?;
    Ok(playlists)
}

fn save(playlists: &[LocalPlaylist]) -> Result<()> {
    let path = playlists_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("创建本地播放列表目录失败")?;
    }
    let part = path.with_extension("json.part");
    fs::write(&part, serde_json::to_vec_pretty(playlists)?).context("写本地播放列表失败")?;
    if path.exists() {
        let _ = fs::remove_file(&path);
    }
    fs::rename(&part, &path).context("保存本地播放列表失败")?;
    if let Ok(mut cache) = memory_cache().lock() {
        *cache = Some(playlists.to_vec());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playlist_is_json_serializable() {
        let item = LocalPlaylist {
            id: "1".into(),
            name: "Later".into(),
            tracks: vec![TrackItem {
                id: "t".into(),
                ..Default::default()
            }],
            updated_at: 1,
        };
        let text = serde_json::to_string(&item).unwrap();
        let decoded: LocalPlaylist = serde_json::from_str(&text).unwrap();
        assert_eq!(decoded, item);
    }
}
