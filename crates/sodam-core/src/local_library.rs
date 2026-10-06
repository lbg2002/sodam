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
    #[serde(default)]
    pub aliases: Vec<String>,
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

pub fn search_tracks(query: &str) -> Vec<TrackItem> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return tracks();
    }
    load()
        .into_iter()
        .filter(|record| {
            let haystack = format!(
                "{} {} {}",
                record.track.title, record.track.artist, record.track.album
            )
            .to_lowercase();
            fuzzy_text_match(&haystack, &query)
                || record
                    .aliases
                    .iter()
                    .any(|alias| fuzzy_text_match(&alias.to_lowercase(), &query))
        })
        .map(|record| record.track)
        .collect()
}

fn fuzzy_text_match(haystack: &str, query: &str) -> bool {
    if haystack.contains(query) {
        return true;
    }
    let mut wanted = query.chars();
    let mut next = wanted.next();
    for ch in haystack.chars() {
        if Some(ch) == next {
            next = wanted.next();
            if next.is_none() {
                return true;
            }
        }
    }
    false
}

pub fn record(track: &TrackItem) -> Result<()> {
    record_many(std::slice::from_ref(track))
}

pub fn record_many(tracks: &[TrackItem]) -> Result<()> {
    record_many_with_alias(tracks, None)
}

pub fn record_many_with_alias(tracks: &[TrackItem], alias: Option<&str>) -> Result<()> {
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
        let mut aliases = map
            .get(&track.id)
            .map(|record| record.aliases.clone())
            .unwrap_or_default();
        if let Some(alias) = alias.map(str::trim).filter(|value| !value.is_empty()) {
            if !aliases.iter().any(|known| known.eq_ignore_ascii_case(alias)) {
                aliases.insert(0, alias.to_string());
                aliases.truncate(8);
            }
        }
        map.insert(
            track.id.clone(),
            LocalTrackRecord {
                track: track.clone(),
                last_seen: now,
                aliases,
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
    fn fuzzy_match_supports_subsequence() {
        assert!(fuzzy_text_match("jay chou 周杰伦", "jco"));
        assert!(fuzzy_text_match("周杰伦 晴天", "晴天"));
        assert!(!fuzzy_text_match("周杰伦 晴天", "abc"));
    }

    #[test]
    fn older_catalog_without_aliases_still_loads() {
        let text = r#"{"track":{"id":"1","title":"Song","artist":"","album":"","artist_id":"","album_id":"","cover":"","duration_seconds":0,"vip":false},"last_seen":1}"#;
        let decoded: LocalTrackRecord = serde_json::from_str(text).unwrap();
        assert!(decoded.aliases.is_empty());
    }

    #[test]
    fn record_type_round_trip() {
        let record = LocalTrackRecord {
            track: TrackItem { id: "1".into(), title: "Song".into(), ..Default::default() },
            last_seen: 1,
            aliases: vec!["song".into()],
        };
        let text = serde_json::to_string(&record).unwrap();
        let decoded: LocalTrackRecord = serde_json::from_str(&text).unwrap();
        assert_eq!(decoded, record);
    }
}
