//! 本地最近播放历史。
//!
//! 只记录曲目元数据与本地统计，不包含 Cookie、签名 token 等账号信息。

use crate::models::TrackItem;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_RECENT: usize = 200;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecentPlayEntry {
    pub track: TrackItem,
    pub played_at: u64,
    pub play_count: u64,
}

fn history_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("sodam")
        .join("recent.json")
}

pub fn load_entries() -> Vec<RecentPlayEntry> {
    fs::read_to_string(history_path())
        .ok()
        .and_then(|text| serde_json::from_str::<Vec<RecentPlayEntry>>(&text).ok())
        .unwrap_or_default()
}

pub fn load_tracks() -> Vec<TrackItem> {
    load_entries()
        .into_iter()
        .map(|entry| entry.track)
        .collect()
}

pub fn record_track(track: &TrackItem) -> Result<Vec<TrackItem>> {
    let mut entries = load_entries();
    let previous_count = entries
        .iter()
        .find(|entry| entry.track.id == track.id)
        .map(|entry| entry.play_count)
        .unwrap_or(0);
    entries.retain(|entry| entry.track.id != track.id);
    let played_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0);
    entries.insert(
        0,
        RecentPlayEntry {
            track: track.clone(),
            played_at,
            play_count: previous_count.saturating_add(1),
        },
    );
    entries.truncate(MAX_RECENT);
    save_entries(&entries)?;
    Ok(entries.into_iter().map(|entry| entry.track).collect())
}

pub fn clear() -> Result<()> {
    let path = history_path();
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err).with_context(|| format!("清理最近播放失败：{}", path.display())),
    }
}

fn save_entries(entries: &[RecentPlayEntry]) -> Result<()> {
    let path = history_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("创建最近播放目录失败")?;
    }
    let part = path.with_extension("json.part");
    fs::write(&part, serde_json::to_vec_pretty(entries)?).context("写最近播放失败")?;
    if path.exists() {
        let _ = fs::remove_file(&path);
    }
    fs::rename(&part, &path).context("保存最近播放失败")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_round_trip() {
        let entry = RecentPlayEntry {
            track: TrackItem {
                id: "1".into(),
                title: "Song".into(),
                artist: "Artist".into(),
                ..Default::default()
            },
            played_at: 42,
            play_count: 3,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let decoded: RecentPlayEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, entry);
    }
}
