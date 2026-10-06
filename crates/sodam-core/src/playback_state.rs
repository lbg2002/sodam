//! 播放恢复状态：只保存队列与播放器位置，不包含账号凭据。

use crate::{models::TrackItem, queue::PlayMode};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlaybackState {
    pub queue: Vec<TrackItem>,
    pub index: usize,
    pub mode: PlayMode,
    pub position_seconds: f64,
    pub was_playing: bool,
    pub saved_at: u64,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self {
            queue: Vec::new(),
            index: 0,
            mode: PlayMode::Sequential,
            position_seconds: 0.0,
            was_playing: false,
            saved_at: 0,
        }
    }
}

impl PlaybackState {
    pub fn path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("sodam")
            .join("playback-state.json")
    }

    pub fn load() -> Option<Self> {
        Self::load_from(&Self::path())
    }

    pub fn load_from(path: &Path) -> Option<Self> {
        let text = std::fs::read_to_string(path).ok()?;
        let mut state: Self = serde_json::from_str(&text).ok()?;
        if state.queue.is_empty() {
            return None;
        }
        state.index = state.index.min(state.queue.len().saturating_sub(1));
        state.position_seconds = state.position_seconds.max(0.0);
        Some(state)
    }

    pub fn save(&self) -> anyhow::Result<()> {
        self.save_to(&Self::path())
    }

    pub fn save_to(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut state = self.clone();
        state.saved_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_secs())
            .unwrap_or(0);
        let part = path.with_extension("json.part");
        std::fs::write(&part, serde_json::to_vec_pretty(&state)?)?;
        if path.exists() {
            let _ = std::fs::remove_file(path);
        }
        std::fs::rename(part, path)?;
        Ok(())
    }

    pub fn clear() {
        let _ = std::fs::remove_file(Self::path());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playback_state_round_trip_clamps_index() {
        let dir =
            std::env::temp_dir().join(format!("sodam-playback-state-{}", std::process::id()));
        let path = dir.join("state.json");
        let state = PlaybackState {
            queue: vec![TrackItem {
                id: "1".into(),
                title: "Song".into(),
                ..Default::default()
            }],
            index: 99,
            mode: PlayMode::RepeatOne,
            position_seconds: 42.5,
            was_playing: true,
            ..Default::default()
        };
        state.save_to(&path).expect("save");
        let loaded = PlaybackState::load_from(&path).expect("load");
        assert_eq!(loaded.index, 0);
        assert_eq!(loaded.mode, PlayMode::RepeatOne);
        assert_eq!(loaded.position_seconds, 42.5);
        assert!(loaded.was_playing);
        let _ = std::fs::remove_dir_all(dir);
    }
}
