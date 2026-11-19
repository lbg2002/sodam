//! Update actions use background workers, never blocking GPUI render.

use super::{Root, UpdateState};
use gpui::prelude::*;
use gpui::Context;

impl Root {
    pub(crate) fn check_for_updates(&mut self, cx: &mut Context<Self>) {
        if matches!(
            self.update_state,
            UpdateState::Checking | UpdateState::Installing
        ) {
            return;
        }
        self.update_state = UpdateState::Checking;
        cx.notify();
        let work = cx.background_spawn(async { crate::updater::check_latest() });
        cx.spawn(async move |this, cx| {
            let result = work.await;
            let _ = this.update(cx, |root, cx| {
                root.update_state = match result {
                    Ok(Some(release)) => UpdateState::Available(release),
                    Ok(None) => UpdateState::UpToDate,
                    Err(error) => UpdateState::Error(error.to_string()),
                };
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn install_available_update(&mut self, cx: &mut Context<Self>) {
        let release = match &self.update_state {
            UpdateState::Available(release) => release.clone(),
            _ => return,
        };
        self.update_state = UpdateState::Installing;
        cx.notify();
        let work = cx.background_spawn(async move { crate::updater::install(&release) });
        cx.spawn(async move |this, cx| {
            let result = work.await;
            let _ = this.update(cx, |root, cx| {
                root.update_state = match result {
                    Ok(()) => UpdateState::Installed,
                    Err(error) => UpdateState::Error(error.to_string()),
                };
                cx.notify();
            });
        })
        .detach();
    }
}
