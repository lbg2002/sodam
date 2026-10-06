//! 本地搜索历史。

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_HISTORY: usize = 30;

fn memory_cache() -> &'static Mutex<Option<Vec<SearchHistoryItem>>> {
    static CACHE: OnceLock<Mutex<Option<Vec<SearchHistoryItem>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchHistoryItem {
    pub query: String,
    pub used_at: u64,
}

fn path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("sodam")
        .join("search-history.json")
}

pub fn load() -> Vec<SearchHistoryItem> {
    if let Ok(cache) = memory_cache().lock() {
        if let Some(history) = cache.as_ref() {
            return history.clone();
        }
    }
    let history = fs::read_to_string(path())
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    if let Ok(mut cache) = memory_cache().lock() {
        *cache = Some(history.clone());
    }
    history
}

pub fn record(query: &str) -> Result<Vec<SearchHistoryItem>> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(load());
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0);
    let mut history = load();
    history.retain(|item| !item.query.eq_ignore_ascii_case(query));
    history.insert(0, SearchHistoryItem { query: query.to_string(), used_at: now });
    history.truncate(MAX_HISTORY);
    save(&history)?;
    Ok(history)
}

pub fn clear() -> Result<()> {
    let result = match fs::remove_file(path()) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err).context("清理搜索历史失败"),
    };
    if result.is_ok() {
        if let Ok(mut cache) = memory_cache().lock() {
            *cache = Some(Vec::new());
        }
    }
    result
}

fn save(history: &[SearchHistoryItem]) -> Result<()> {
    let path = path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("创建搜索历史目录失败")?;
    }
    fs::write(path, serde_json::to_vec_pretty(history)?).context("保存搜索历史失败")?;
    if let Ok(mut cache) = memory_cache().lock() {
        *cache = Some(history.to_vec());
    }
    Ok(())
}
