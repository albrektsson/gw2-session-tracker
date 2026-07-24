use nexus::imgui::Ui;
use std::{
    sync::{atomic::AtomicBool, Arc, Mutex},
    time::Instant,
};
use session_tracker_core::stats::WVW_STATS;
use session_tracker_net::state::{AppState, PollStatus};

pub static SHOW_MAIN: AtomicBool = AtomicBool::new(false);

pub fn render_main_window(ui: &Ui, shared: &Arc<Mutex<AppState>>) {
    nexus::imgui::Window::new("Session Tracker").build(ui, || {
        let state = shared.lock().unwrap();

        match &state.status {
            PollStatus::AwaitingApiKey => {
                ui.text("No API key configured yet.");
                ui.text("Open settings (ALT+SHIFT+E) to add one.");
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

        if let Some(_table) = ui.begin_table("wvw-stats-table", 3) {
            ui.table_setup_column("Stat");
            ui.table_setup_column("Session");
            ui.table_setup_column("Lifetime");
            ui.table_headers_row();

            for stat in WVW_STATS {
                ui.table_next_row();
                ui.table_next_column();
                ui.text(stat.display_name);
                ui.table_next_column();
                let session_value = if stat.id == "kdr" {
                    // KDR is a ratio, not a count: diffing lifetime KDR at
                    // session start vs. now (the generic session_value
                    // behavior) produces a meaningless number. Compute it
                    // properly from the session kills/deaths deltas instead,
                    // matching the zero-deaths fallback convention used for
                    // lifetime KDR in compute_lifetime_values.
                    let session_kills = state.session.session_value("kills");
                    let session_deaths = state.session.session_value("deaths");
                    if session_deaths > 0.0 {
                        session_kills / session_deaths
                    } else {
                        session_kills
                    }
                } else {
                    state.session.session_value(stat.id)
                };
                ui.text(format!("{:.0}", session_value));
                ui.table_next_column();
                ui.text(format!("{:.0}", state.session.lifetime_value(stat.id)));
            }
        }
    });
}
