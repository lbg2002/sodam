// Compatibility scope for the original settings implementation.
// Re-export the views module helpers into this compatibility module so the
// legacy implementation's `use super::*` keeps seeing the same UI helpers,
// while compiling the legacy file as a real Rust module (so its inner `//!`
// module doc comment remains valid).
use super::super::*;

#[path = "settings_legacy_full.rs"]
mod old;

pub(crate) use old::{duration_of, settings_view};
