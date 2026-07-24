use nexus::imgui::{TableColumnFlags, TableColumnSetup, TableFlags, Ui};
use std::{
    sync::{atomic::AtomicBool, Arc, Mutex},
    time::Instant,
};
use session_tracker_core::{format::format_thousands, stats::resolve_selected_stats};
use session_tracker_net::state::{AppState, PollStatus};

pub static SHOW_MAIN: AtomicBool = AtomicBool::new(false);

/// Session KDR from the session kills/deaths deltas, falling back to raw
/// kills when there have been no deaths this session (mirrors the
/// lifetime KDR fallback in `compute_lifetime_values`).
fn session_kdr(state: &AppState, kills_id: &str) -> f64 {
    let session_kills = state.session.session_value(kills_id);
    let session_deaths = state.session.session_value("deaths");
    if session_deaths > 0.0 {
        session_kills / session_deaths
    } else {
        session_kills
    }
}

pub fn render_main_window(ui: &Ui, shared: &Arc<Mutex<AppState>>) {
    nexus::imgui::Window::new("Session Tracker").build(ui, || {
        let state = shared.lock().unwrap();

        match &state.status {
            PollStatus::AwaitingApiKey => {
                ui.text("No API key configured yet.");
                ui.text(format!(
                    "Open Settings (default keybind {}, rebindable in Nexus) to add one.",
                    crate::SETTINGS_KEYBIND_DEFAULT
                ));
                return;
            }
            PollStatus::Error(err) => {
                ui.text_colored([1.0, 0.4, 0.4, 1.0], format!("Last poll failed: {err}"));
                ui.text("Showing last known values below.");
            }
            PollStatus::Ok => {}
        }

        if !state.session.has_data() {
            ui.text("Waiting for first successful poll...");
            return;
        }

        if let Some(last_updated) = state.last_updated {
            let secs_ago = Instant::now().saturating_duration_since(last_updated).as_secs();
            ui.text(format!("Last updated {secs_ago}s ago"));
        }

        let selected = resolve_selected_stats(&state.selected_stats);
        if selected.is_empty() {
            ui.text(format!(
                "No stats selected. Open Settings (default keybind {}, rebindable in Nexus) to pick some.",
                crate::SETTINGS_KEYBIND_DEFAULT
            ));
            return;
        }

        let table_flags = TableFlags::RESIZABLE;
        if let Some(_table) = ui.begin_table_with_flags("wvw-stats-table", 3, table_flags) {
            ui.table_setup_column_with(TableColumnSetup {
                flags: TableColumnFlags::WIDTH_STRETCH,
                init_width_or_weight: 3.0,
                ..TableColumnSetup::new("Stat")
            });
            ui.table_setup_column_with(TableColumnSetup {
                flags: TableColumnFlags::WIDTH_STRETCH,
                init_width_or_weight: 1.0,
                ..TableColumnSetup::new("Session")
            });
            ui.table_setup_column_with(TableColumnSetup {
                flags: TableColumnFlags::WIDTH_STRETCH,
                init_width_or_weight: 1.0,
                ..TableColumnSetup::new("Lifetime")
            });
            ui.table_headers_row();

            for stat in selected {
                ui.table_next_row();
                ui.table_next_column();
                ui.text(stat.display_name);
                ui.table_next_column();
                // KDR-shaped stats are ratios, not counts: diffing lifetime
                // KDR at session start vs. now (the generic session_value
                // behavior) produces a meaningless number. Compute them
                // properly from the session kills/deaths deltas instead,
                // matching the zero-deaths fallback convention used for
                // lifetime KDR in compute_lifetime_values.
                let session_value = match stat.id {
                    "kdr" => session_kdr(&state, "kills"),
                    "pvp_kdr" => session_kdr(&state, "pvp_kills"),
                    _ => state.session.session_value(stat.id),
                };
                ui.text(format_thousands(session_value));
                ui.table_next_column();
                ui.text(format_thousands(state.session.lifetime_value(stat.id)));
            }
        }
    });
}
