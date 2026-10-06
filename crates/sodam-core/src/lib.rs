//! sodam 的领域层：配置、会话、队列与数据模型。
//!
//! 这里**不依赖 GPUI**，方便单独测试与复用（未来做 CLI/TUI 或守护进程都能直接用）。

pub mod audio;
pub mod color;
pub mod config;
pub mod downloads;
pub mod history;
pub mod library;
pub mod login;
pub mod models;
pub mod playback_state;
pub mod queue;
pub mod session;

pub use audio::{PlaybackEngine, PlaybackSnapshot};
pub use config::Settings;
pub use downloads::{download_dir, DownloadedTrack};
pub use history::RecentPlayEntry;
pub use models::{PlaylistItem, SceneItem, TrackItem};
pub use playback_state::PlaybackState;
pub use queue::{PlayMode, Queue};
pub use session::Session;
