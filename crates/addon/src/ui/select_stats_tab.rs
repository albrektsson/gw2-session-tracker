use nexus::imgui::Ui;
use std::{
    cell::RefCell,
    path::Path,
    sync::{Arc, Mutex},
};
use session_tracker_core::{
    config::save_config,
    stats::{select_all, toggle_stat, unselect_all, WVW_STATS},
};
use session_tracker_net::state::AppState;

use super::settings_window::config_from_state;

thread_local! {
    static SEARCH_FILTER: RefCell<String> = const { RefCell::new(String::new()) };
}

fn persist(state: &AppState, addon_dir: &Path) {
    let config = config_from_state(state);
    if let Err(err) = save_config(addon_dir, &config) {
        log::warn!("failed to save session tracker config: {err}");
    }
}

pub fn render_select_stats_tab(ui: &Ui, shared: &Arc<Mutex<AppState>>, addon_dir: &Path) {
    SEARCH_FILTER.with(|filter| {
        let mut query = filter.borrow_mut();
        ui.input_text("##stat_search", &mut query)
            .hint("Search stats...")
            .build();

        if ui.button("Select all") {
            let mut state = shared.lock().unwrap();
            select_all(&mut state.selected_stats);
            persist(&state, addon_dir);
        }
        ui.same_line();
        if ui.button("Unselect all") {
            let mut state = shared.lock().unwrap();
            unselect_all(&mut state.selected_stats);
            persist(&state, addon_dir);
        }

        ui.separator();

        let needle = query.to_lowercase();
        let mut state = shared.lock().unwrap();
        for stat in WVW_STATS {
            if !needle.is_empty() && !stat.display_name.to_lowercase().contains(&needle) {
                continue;
            }
            let mut checked = state.selected_stats.iter().any(|id| id == stat.id);
            if ui.checkbox(stat.display_name, &mut checked) {
                toggle_stat(&mut state.selected_stats, stat.id);
                persist(&state, addon_dir);
            }
        }
    });
}
