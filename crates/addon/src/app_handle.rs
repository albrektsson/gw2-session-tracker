use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
};
use session_tracker_core::{
    config::{save_config, Config},
    stat_list,
    sync::lock_recover,
};
use session_tracker_net::state::{AppState, PollStatus, StatListKind};

/// Wraps the addon's shared `AppState` together with the on-disk config
/// directory, behind one seam: every mutation that needs to be persisted
/// goes through here instead of each UI call site hand-rolling its own
/// lock/mutate/save/report-error sequence.
pub struct AppHandle {
    shared: Arc<Mutex<AppState>>,
    addon_dir: PathBuf,
}

fn build_config(state: &AppState) -> Config {
    Config {
        api_key: state.api_key.clone(),
        selected_stats: state.selected_stats.clone(),
        wvw_selected_stats: state.wvw_selected_stats.clone(),
        pvp_selected_stats: state.pvp_selected_stats.clone(),
        pve_selected_stats: state.pve_selected_stats.clone(),
        background_opacity: state.background_opacity,
        text_scale: state.text_scale,
        bold_text: state.bold_text,
        text_color: state.text_color,
        icon_color: state.icon_color,
        show_settings: state.show_settings,
        show_main: state.show_main,
    }
}

impl AppHandle {
    pub fn new(shared: Arc<Mutex<AppState>>, addon_dir: PathBuf) -> Self {
        Self { shared, addon_dir }
    }

    pub fn addon_dir(&self) -> &Path {
        &self.addon_dir
    }

    pub fn lock(&self) -> MutexGuard<'_, AppState> {
        lock_recover(&self.shared)
    }

    /// Applies `f` to the locked `AppState`, then persists the result to
    /// disk. A save failure surfaces as `PollStatus::Error` - the one
    /// user-facing error channel the addon has - rather than being
    /// dropped silently.
    pub fn mutate_and_persist(&self, f: impl FnOnce(&mut AppState)) {
        let mut state = self.lock();
        f(&mut state);
        let config = build_config(&state);
        if let Err(err) = save_config(&self.addon_dir, &config) {
            log::warn!("failed to save session tracker config: {err}");
            state.status = PollStatus::Error(format!("failed to save config: {err}"));
        }
    }

    pub fn toggle_stat(&self, kind: StatListKind, id: &str) {
        self.mutate_and_persist(|state| stat_list::toggle_stat(state.stat_list_mut(kind), id));
    }

    pub fn select_all(&self, kind: StatListKind) {
        self.mutate_and_persist(|state| stat_list::select_all(state.stat_list_mut(kind)));
    }

    pub fn unselect_all(&self, kind: StatListKind) {
        self.mutate_and_persist(|state| stat_list::unselect_all(state.stat_list_mut(kind)));
    }

    pub fn select_ids(&self, kind: StatListKind, ids: &[&str]) {
        self.mutate_and_persist(|state| stat_list::select_ids(state.stat_list_mut(kind), ids));
    }

    pub fn unselect_ids(&self, kind: StatListKind, ids: &[&str]) {
        self.mutate_and_persist(|state| stat_list::unselect_ids(state.stat_list_mut(kind), ids));
    }

    pub fn move_stat_up(&self, kind: StatListKind, id: &str) {
        self.mutate_and_persist(|state| stat_list::move_stat_up(state.stat_list_mut(kind), id));
    }

    pub fn move_stat_down(&self, kind: StatListKind, id: &str) {
        self.mutate_and_persist(|state| stat_list::move_stat_down(state.stat_list_mut(kind), id));
    }

    pub fn move_stat_to(&self, kind: StatListKind, id: &str, before_id: &str) {
        self.mutate_and_persist(|state| stat_list::move_stat_to(state.stat_list_mut(kind), id, before_id));
    }
}
