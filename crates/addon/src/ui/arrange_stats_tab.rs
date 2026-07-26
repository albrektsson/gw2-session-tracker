use nexus::imgui::{Direction, DragDropFlags, DragDropSource, DragDropTarget, Selectable, Ui};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use session_tracker_core::{
    config::save_config,
    stats::{move_stat_down, move_stat_to, move_stat_up, resolve_selected_stats},
    sync::lock_recover,
};
use session_tracker_net::state::{AppState, PollStatus};

use super::settings_window::config_from_state;

const DRAG_DROP_PAYLOAD: &str = "SESSION_TRACKER_STAT_ROW";

enum PendingMove {
    Step { id: &'static str, up: bool },
    To { id: &'static str, before_id: &'static str },
}

pub fn render_arrange_stats_tab(ui: &Ui, shared: &Arc<Mutex<AppState>>, addon_dir: &Path) {
    let mut state = lock_recover(shared);
    let selected = resolve_selected_stats(&state.selected_stats);
    if selected.is_empty() {
        ui.text("No stats selected. Use the Select Stats tab to pick some first.");
        return;
    }

    let mut pending_move: Option<PendingMove> = None;
    for stat in &selected {
        if ui.arrow_button(format!("##up_{}", stat.id), Direction::Up) {
            pending_move = Some(PendingMove::Step { id: stat.id, up: true });
        }
        ui.same_line();
        if ui.arrow_button(format!("##down_{}", stat.id), Direction::Down) {
            pending_move = Some(PendingMove::Step { id: stat.id, up: false });
        }
        ui.same_line();

        Selectable::new(format!("{}##drag_{}", stat.display_name, stat.id)).build(ui);

        if DragDropSource::new(DRAG_DROP_PAYLOAD).begin_payload(ui, stat.id).is_some() {
            ui.text(stat.display_name);
        }
        if let Some(target) = DragDropTarget::new(ui) {
            if let Some(Ok(payload)) =
                target.accept_payload::<&'static str, _>(DRAG_DROP_PAYLOAD, DragDropFlags::empty())
            {
                pending_move = Some(PendingMove::To { id: payload.data, before_id: stat.id });
            }
            target.pop();
        }
    }

    if let Some(pending) = pending_move {
        match pending {
            PendingMove::Step { id, up: true } => move_stat_up(&mut state.selected_stats, id),
            PendingMove::Step { id, up: false } => move_stat_down(&mut state.selected_stats, id),
            PendingMove::To { id, before_id } => move_stat_to(&mut state.selected_stats, id, before_id),
        }

        let config = config_from_state(&state);
        if let Err(err) = save_config(addon_dir, &config) {
            log::warn!("failed to save session tracker config: {err}");
            state.status = PollStatus::Error(format!("failed to save config: {err}"));
        }
    }
}
