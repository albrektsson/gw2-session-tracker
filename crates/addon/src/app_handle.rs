use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
};
use session_tracker_core::{config::save_config, session_state::save_session_state, stat_list, sync::lock_recover};
use session_tracker_net::state::{AppState, PollStatus, StatListKind};

/// Wraps the addon's shared `AppState` together with the on-disk config
/// directory, behind one seam: every mutation that needs to be persisted
/// goes through here instead of each UI call site hand-rolling its own
/// lock/mutate/save/report-error sequence.
pub struct AppHandle {
    shared: Arc<Mutex<AppState>>,
    addon_dir: PathBuf,
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
        {
            let mut state = self.lock();
            f(&mut state);
        }
        self.persist();
    }

    /// Saves the current `AppState.config` to disk without mutating it -
    /// for callers that already wrote directly into a held lock (e.g. a
    /// per-frame in-memory update that only wants to hit disk once, on a
    /// specific frame) rather than going through `mutate_and_persist`.
    pub fn persist(&self) {
        let mut state = self.lock();
        if let Err(err) = save_config(&self.addon_dir, &state.config) {
            log::warn!("failed to save session tracker config: {err}");
            state.status = PollStatus::Error(format!("failed to save config: {err}"));
        }
    }

    /// Saves the current Session's state to disk (see
    /// `session_tracker_core::session_state`), so it can survive an addon
    /// unload/reload per the configured Automatic Reset Mode. Called on
    /// `unload()` and, explicitly, right after a manual reset - unlike
    /// `persist()` (config), this isn't called on every settings change,
    /// only those two points.
    pub fn persist_session(&self) {
        let mut state = self.lock();
        let snapshot = state.session.snapshot();
        if let Err(err) = save_session_state(&self.addon_dir, snapshot.as_ref()) {
            log::warn!("failed to save session state: {err}");
            state.status = PollStatus::Error(format!("failed to save session state: {err}"));
        }
    }

    /// Resets the Session and immediately persists the (now empty) result,
    /// rather than waiting for the next `unload()` - otherwise a crash
    /// between this call and a clean unload would restore the stale
    /// pre-reset session on the next load, silently undoing the reset.
    pub fn reset_session(&self) {
        self.lock().session.reset();
        self.persist_session();
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
