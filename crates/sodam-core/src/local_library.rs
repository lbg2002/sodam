//! 本地音乐目录索引：记录用户真正遇到过的曲目元数据，供离线音乐中心使用。

use crate::models::TrackItem;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_TRACKS: usize = 5000;

fn memory_cache() -> &'static Mutex<Option<Vec<LocalTrackRecord>>> {
    static CACHE: OnceLock<Mutex<Option<Vec<LocalTrackRecord>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocalTrackRecord {
    pub track: TrackItem,
    pub last_seen: u64,
}

fn catalog_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("sodam")
        .join("local-library.json")
}

pub fn load() -> Vec<LocalTrackRecord> {
    if let Ok(cache) = memory_cache().lock() {
        if let Some(records) = cache.as_ref() {
            return records.clone();
        }
    }
    let records = fs::read_to_string(catalog_path())
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    if let Ok(mut cache) = memory_cache().lock() {
        *cache = Some(records.clone());
    }
    records
}

pub fn tracks() -> Vec<TrackItem> {
    load().into_iter().map(|record| record.track).collect()
}

pub fn record(track: &TrackItem) -> Result<()> {
    record_many(std::slice::from_ref(track))
}

pub fn record_many(tracks: &[TrackItem]) -> Result<()> {
    if tracks.is_empty() {
        return Ok(());
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0);
    let mut map: HashMap<String, LocalTrackRecord> = load()
        .into_iter()
        .map(|record| (record.track.id.clone(), record))
        .collect();
    for track in tracks {
        if track.id.trim().is_empty() {
            continue;
        }
        map.insert(
            track.id.clone(),
            LocalTrackRecord {
                track: track.clone(),
                last_seen: now,
            },
        );
    }
    let mut records: Vec<_> = map.into_values().collect();
    records.sort_by_key(|record| std::cmp::Reverse(record.last_seen));
    records.truncate(MAX_TRACKS);
    save(&records)
}

fn save(records: &[LocalTrackRecord]) -> Result<()> {
    let path = catalog_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("创建本地音乐索引目录失败")?;
    }
    let part = path.with_extension("json.part");
    fs::write(&part, serde_json::to_vec_pretty(records)?).context("写本地音乐索引失败")?;
    if path.exists() {
        let _ = fs::remove_file(&path);
    }
    fs::rename(&part, &path).context("保存本地音乐索引失败")?;
    if let Ok(mut cache) = memory_cache().lock() {
        *cache = Some(records.to_vec());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_type_round_trip() {
        let record = LocalTrackRecord {
            track: TrackItem { id: "1".into(), title: "Song".into(), ..Default::default() },
            last_seen: 1,
        };
        let text = serde_json::to_string(&record).unwrap();
        let decoded: LocalTrackRecord = serde_json::from_str(&text).unwrap();
        assert_eq!(decoded, record);
    }
}
