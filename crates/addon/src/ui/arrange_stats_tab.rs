use nexus::imgui::{Direction, DragDropFlags, DragDropSource, DragDropTarget, Selectable, Ui};
use session_tracker_core::stat_list::resolve_selected_stats;
use session_tracker_net::state::StatListKind;

use crate::app_handle::AppHandle;
use super::stat_icon::render_stat_icon;

const DRAG_DROP_PAYLOAD: &str = "SESSION_TRACKER_STAT_ROW";
const ICON_SIZE: f32 = 16.0;

enum PendingMove {
    Step { id: &'static str, up: bool },
    To { id: &'static str, before_id: &'static str },
}

pub fn render_arrange_stats_tab(ui: &Ui, app: &AppHandle) {
    if let Some(_tabs) = ui.tab_bar("arrange-list-scope") {
        for kind in StatListKind::ALL {
            if let Some(_tab) = ui.tab_item(kind.label()) {
                render_arrange_stats_editor(ui, app, kind);
            }
        }
    }
}

fn render_arrange_stats_editor(ui: &Ui, app: &AppHandle, kind: StatListKind) {
    let label = kind.label();
    let cache_dir = session_tracker_net::icon_cache::cache_dir(app.addon_dir());
    let selected_ids = app.lock().stat_list(kind).clone();
    let selected = resolve_selected_stats(&selected_ids);
    if selected.is_empty() {
        ui.text("No stats selected. Use the Select Stats tab to pick some first.");
        return;
    }

    let mut pending_move: Option<PendingMove> = None;
    for stat in &selected {
        if ui.arrow_button(format!("##{label}_up_{}", stat.id), Direction::Up) {
            pending_move = Some(PendingMove::Step { id: stat.id, up: true });
        }
        ui.same_line();
        if ui.arrow_button(format!("##{label}_down_{}", stat.id), Direction::Down) {
            pending_move = Some(PendingMove::Step { id: stat.id, up: false });
        }
        ui.same_line();

        render_stat_icon(stat, &app.lock(), &cache_dir, ICON_SIZE, ui);
        Selectable::new(format!("{}##{label}_drag_{}", stat.display_name, stat.id)).build(ui);

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
            PendingMove::Step { id, up: true } => app.move_stat_up(kind, id),
            PendingMove::Step { id, up: false } => app.move_stat_down(kind, id),
            PendingMove::To { id, before_id } => app.move_stat_to(kind, id, before_id),
        }
    }
}
