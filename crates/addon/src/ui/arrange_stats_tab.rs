use nexus::imgui::{Direction, Ui};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use session_tracker_core::{
    config::save_config,
    stats::{move_stat_down, move_stat_up, resolve_selected_stats},
    sync::lock_recover,
};
use session_tracker_net::state::{AppState, PollStatus};

use super::settings_window::config_from_state;

pub fn render_arrange_stats_tab(ui: &Ui, shared: &Arc<Mutex<AppState>>, addon_dir: &Path) {
    let mut state = lock_recover(shared);
    let selected = resolve_selected_stats(&state.selected_stats);
    if selected.is_empty() {
        ui.text("No stats selected. Use the Select Stats tab to pick some first.");
        return;
    }

    // Rows only read state during this loop; any reorder is applied once
    // after the loop so we're never mutating `state.selected_stats` while
    // `selected` (derived from it) is still being iterated.
    let mut pending_move: Option<(&'static str, bool)> = None;
    for stat in &selected {
        ui.text(stat.display_name);
        ui.same_line();
        if ui.arrow_button(format!("##up_{}", stat.id), Direction::Up) {
            pending_move = Some((stat.id, true));
        }
        ui.same_line();
        if ui.arrow_button(format!("##down_{}", stat.id), Direction::Down) {
            pending_move = Some((stat.id, false));
        }
    }

    if let Some((id, move_up)) = pending_move {
        if move_up {
            move_stat_up(&mut state.selected_stats, id);
        } else {
            move_stat_down(&mut state.selected_stats, id);
        }

        let config = config_from_state(&state);
        if let Err(err) = save_config(addon_dir, &config) {
            log::warn!("failed to save session tracker config: {err}");
            state.status = PollStatus::Error(format!("failed to save config: {err}"));
        }
    }
}
