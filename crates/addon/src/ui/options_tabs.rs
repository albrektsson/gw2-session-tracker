use nexus::imgui::{TreeNodeFlags, Ui};
use std::cell::Cell;
use session_tracker_core::config::{AutomaticResetMode, Weekday};
use session_tracker_net::state::PollStatus;

use crate::app_handle::AppHandle;
use super::appearance_tab::render_appearance_tab;
use super::arrange_stats_tab::render_arrange_stats_tab;
use super::formatting_tab::render_formatting_tab;
use super::select_stats_tab::render_select_stats_tab;
use super::window_behavior_tab::render_window_behavior_tab;

// Text input buffer for the API key field. ImGui is single-threaded and
// this is only ever touched from the render callback, so a plain
// thread-local `RefCell` (no atomics/locking) is sufficient.
thread_local! {
    static API_KEY_INPUT: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
    // Tracks whether API_KEY_INPUT has been seeded from the already-loaded
    // AppState.api_key yet. Seeding must happen exactly once (on first
    // render) rather than every frame, otherwise a user clearing the field
    // to type a new key would have it reset out from under them.
    static SEEDED: Cell<bool> = const { Cell::new(false) };
}

/// Renders Session Tracker's config UI as sub-tabs directly into Nexus's
/// own addon "Options" panel (registered via `RenderType::OptionsRender`
/// in `lib.rs`) - Nexus already draws the surrounding "Options" header
/// and window chrome, so this renders the tab bar straight into the
/// current ImGui window rather than opening one of its own.
pub fn render_options_tabs(ui: &Ui, app: &AppHandle) {
    if let Some(_tabs) = ui.tab_bar("session-tracker-options-tabs") {
        if let Some(_tab) = ui.tab_item("General") {
            render_general_tab(ui, app);
        }
        if let Some(_tab) = ui.tab_item("Select Stats") {
            render_select_stats_tab(ui, app);
        }
        if let Some(_tab) = ui.tab_item("Arrange Stats") {
            render_arrange_stats_tab(ui, app);
        }
        if let Some(_tab) = ui.tab_item("Appearance") {
            render_appearance_tab(ui, app);
        }
        if let Some(_tab) = ui.tab_item("Window Behavior") {
            render_window_behavior_tab(ui, app);
        }
        if let Some(_tab) = ui.tab_item("Formatting") {
            render_formatting_tab(ui, app);
        }
    }
}

fn render_general_tab(ui: &Ui, app: &AppHandle) {
    API_KEY_INPUT.with(|input| {
        let mut buf = input.borrow_mut();

        if !SEEDED.with(|seeded| seeded.get()) {
            if let Some(existing) = &app.lock().config.api_key {
                *buf = existing.clone();
            }
            SEEDED.with(|seeded| seeded.set(true));
        }

        let wrap_token = ui.push_text_wrap_pos();
        ui.text("GW2 API key (needs account, characters, progression scopes; add wallet + pvp + inventories scopes too, to also see currency/PvP/item stats):");
        wrap_token.pop(ui);
        ui.input_text("##api_key", &mut buf).password(true).build();
        ui.text_disabled("Stored unencrypted in session_tracker_config.json.");

        if ui.button("Save") {
            let trimmed = buf.trim().to_string();
            if trimmed.is_empty() {
                app.lock().status = PollStatus::Error("API key can't be empty".to_string());
            } else {
                app.mutate_and_persist(|state| {
                    state.config.api_key = Some(trimmed);
                    state.status = PollStatus::Pending;
                });
                log::info!("API key saved, will be used on the next poll cycle");
            }
        }
    });

    {
        let state = app.lock();
        match &state.status {
            PollStatus::AwaitingApiKey => ui.text("Enter an API key above to start tracking."),
            PollStatus::Pending => ui.text("Key saved. First update can take up to 60s..."),
            PollStatus::Ok => ui.text("API key accepted, stats are updating."),
            PollStatus::Error(err) => ui.text_colored([1.0, 0.4, 0.4, 1.0], format!("Error: {err}")),
        }
    }

    ui.separator();
    let has_data = app.lock().session.has_data();
    if has_data {
        if ui.button("Reset Session") {
            app.reset_session();
        }
    } else {
        ui.text("Reset Session (available after the first successful poll)");
    }

    ui.separator();
    render_automatic_reset_section(ui, app);
}

const AUTOMATIC_RESET_MODES: [AutomaticResetMode; 5] = [
    AutomaticResetMode::OnLoad,
    AutomaticResetMode::Never,
    AutomaticResetMode::MinutesAfterUnload,
    AutomaticResetMode::Daily,
    AutomaticResetMode::Weekly,
];

fn automatic_reset_mode_label(mode: AutomaticResetMode) -> &'static str {
    match mode {
        AutomaticResetMode::OnLoad => "On addon load",
        AutomaticResetMode::Never => "Never",
        AutomaticResetMode::MinutesAfterUnload => "N minutes after addon unload",
        AutomaticResetMode::Daily => "Daily reset (00:00 UTC)",
        AutomaticResetMode::Weekly => "Weekly reset",
    }
}

/// The Automatic Reset Mode picker - a collapsible section (matches the
/// `nexus` addon options convention of packing several concerns into one
/// tab) rather than its own tab, since it's one dropdown plus at most a
/// couple of conditional fields.
fn render_automatic_reset_section(ui: &Ui, app: &AppHandle) {
    if !ui.collapsing_header("Automatic Reset", TreeNodeFlags::empty()) {
        return;
    }

    let current_mode = app.lock().config.automatic_reset_mode;
    let mut mode_index = AUTOMATIC_RESET_MODES.iter().position(|&m| m == current_mode).unwrap_or(0);
    let mode_labels: Vec<&str> = AUTOMATIC_RESET_MODES.iter().map(|&m| automatic_reset_mode_label(m)).collect();
    if ui.combo_simple_string("Reset schedule", &mut mode_index, &mode_labels) {
        let new_mode = AUTOMATIC_RESET_MODES[mode_index];
        app.mutate_and_persist(|state| state.config.automatic_reset_mode = new_mode);
    }

    match current_mode {
        AutomaticResetMode::OnLoad => {
            let wrap_token = ui.push_text_wrap_pos();
            ui.text_disabled(
                "Resets every time the addon loads - including a Nexus hotload or auto-update, not just a full game restart.",
            );
            wrap_token.pop(ui);
        }
        AutomaticResetMode::MinutesAfterUnload => {
            let mut minutes = app.lock().config.automatic_reset_minutes as i32;
            if ui.input_int("Minutes", &mut minutes).build() {
                let minutes = minutes.max(1) as u32;
                app.mutate_and_persist(|state| state.config.automatic_reset_minutes = minutes);
            }
        }
        AutomaticResetMode::Weekly => {
            let current_day = app.lock().config.automatic_reset_weekly_day;
            let mut day_index = Weekday::ALL.iter().position(|&d| d == current_day).unwrap_or(0);
            let day_labels: Vec<&str> = Weekday::ALL.iter().map(|d| d.label()).collect();
            if ui.combo_simple_string("Day (UTC)", &mut day_index, &day_labels) {
                let new_day = Weekday::ALL[day_index];
                app.mutate_and_persist(|state| state.config.automatic_reset_weekly_day = new_day);
            }

            let mut hour = app.lock().config.automatic_reset_weekly_hour as i32;
            if ui.input_int("Hour (0-23, UTC)", &mut hour).build() {
                let hour = hour.clamp(0, 23) as u32;
                app.mutate_and_persist(|state| state.config.automatic_reset_weekly_hour = hour);
            }

            let mut minute = app.lock().config.automatic_reset_weekly_minute as i32;
            if ui.input_int("Minute", &mut minute).build() {
                let minute = minute.clamp(0, 59) as u32;
                app.mutate_and_persist(|state| state.config.automatic_reset_weekly_minute = minute);
            }
        }
        AutomaticResetMode::Never | AutomaticResetMode::Daily => {}
    }
}
