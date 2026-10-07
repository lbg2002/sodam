// Compatibility scope for the original settings implementation.
// Keep it one module deeper so its `use super::*` imports the views module's
// UI helpers, not the custom wrapper functions defined in settings.rs.
mod legacy_scope {
    use super::super::super::*;

    pub(crate) mod old {
        include!("settings_legacy_full.rs");
    }
}

pub(crate) use legacy_scope::old::{duration_of, settings_view};
