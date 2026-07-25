use nexus::imgui::{ColorEdit, Slider, Ui};
use std::{
    cell::Cell,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use session_tracker_core::{
    config::{save_config, Config},
    sync::lock_recover,
};
use session_tracker_net::state::{AppState, PollStatus};

use super::arrange_stats_tab::render_arrange_stats_tab;
use super::main_window::SHOW_MAIN;
use super::select_stats_tab::render_select_stats_tab;

pub static SHOW_SETTINGS: AtomicBool = AtomicBool::new(false);

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
    // Set right after a successful Save, so we can show "key saved, first
    // update is coming" instead of the generic "enter a key" prompt while
    // the poller hasn't run its first cycle with the new key yet (it polls
    // at most once every 60s, so there's no other feedback for a while).
    static JUST_SAVED: Cell<bool> = const { Cell::new(false) };
}

/// Builds a `Config` snapshot from the currently-locked `AppState` plus the
/// window-visibility atomics, so every save site persists the full config
/// instead of clobbering fields it doesn't own.
pub(crate) fn config_from_state(state: &AppState) -> Config {
    Config {
        api_key: state.api_key.clone(),
        selected_stats: state.selected_stats.clone(),
        background_opacity: state.background_opacity,
        text_scale: state.text_scale,
        bold_text: state.bold_text,
        text_color: state.text_color,
        icon_color: state.icon_color,
        show_settings: SHOW_SETTINGS.load(Ordering::Relaxed),
        show_main: SHOW_MAIN.load(Ordering::Relaxed),
    }
}

pub fn render_settings_window(ui: &Ui, shared: &Arc<Mutex<AppState>>, addon_dir: &Path) {
    nexus::imgui::Window::new("Session Tracker Settings").build(ui, || {
        if let Some(_tabs) = ui.tab_bar("settings-tabs") {
            if let Some(_tab) = ui.tab_item("General") {
                render_general_tab(ui, shared, addon_dir);
            }
            if let Some(_tab) = ui.tab_item("Select Stats") {
                render_select_stats_tab(ui, shared, addon_dir);
            }
            if let Some(_tab) = ui.tab_item("Arrange Stats") {
                render_arrange_stats_tab(ui, shared, addon_dir);
            }
        }
    });
}

fn render_general_tab(ui: &Ui, shared: &Arc<Mutex<AppState>>, addon_dir: &Path) {
    API_KEY_INPUT.with(|input| {
        let mut buf = input.borrow_mut();

        if !SEEDED.with(|seeded| seeded.get()) {
            if let Some(existing) = &lock_recover(shared).api_key {
                *buf = existing.clone();
            }
            SEEDED.with(|seeded| seeded.set(true));
        }

        ui.text("GW2 API key (needs account, characters, progression scopes;");
        ui.text("add wallet + pvp + inventories scopes too, to also see");
        ui.text("currency/PvP/item stats):");
        ui.input_text("##api_key", &mut buf).password(true).build();
        ui.text_disabled("Stored unencrypted in session_tracker_config.json.");

        if ui.button("Save") {
            let mut state = lock_recover(shared);
            let trimmed = buf.trim();
            if trimmed.is_empty() {
                state.status = PollStatus::Error("API key can't be empty".to_string());
            } else {
                state.api_key = Some(trimmed.to_string());
                let config = config_from_state(&state);
                if let Err(err) = save_config(addon_dir, &config) {
                    log::warn!("failed to save session tracker config: {err}");
                    state.status = PollStatus::Error(format!("failed to save config: {err}"));
                } else {
                    log::info!("API key saved, will be used on the next poll cycle");
                    JUST_SAVED.with(|s| s.set(true));
                }
            }
        }
    });

    {
        let state = lock_recover(shared);
        match &state.status {
            PollStatus::AwaitingApiKey => {
                if JUST_SAVED.with(|s| s.get()) {
                    ui.text("Key saved. First update can take up to 60s...");
                } else {
                    ui.text("Enter an API key above to start tracking.");
                }
            }
            PollStatus::Ok => ui.text("API key accepted, stats are updating."),
            PollStatus::Error(err) => ui.text_colored([1.0, 0.4, 0.4, 1.0], format!("Error: {err}")),
        }
    }

    let mut state = lock_recover(shared);

    ui.separator();
    ui.text("Main window background opacity:");
    let mut opacity = state.background_opacity;
    if Slider::new("##background_opacity", 0.0f32, 1.0f32).build(ui, &mut opacity) {
        state.background_opacity = opacity;
        persist_and_report(&mut state, addon_dir);
    }

    ui.text("Main window text size:");
    let mut text_scale = state.text_scale;
    if Slider::new("##text_scale", 0.5f32, 3.0f32).build(ui, &mut text_scale) {
        state.text_scale = text_scale;
        persist_and_report(&mut state, addon_dir);
    }

    let mut bold_text = state.bold_text;
    if ui.checkbox("Bold text", &mut bold_text) {
        state.bold_text = bold_text;
        persist_and_report(&mut state, addon_dir);
    }

    let mut text_color = state.text_color;
    if ColorEdit::new("Text color", &mut text_color).build(ui) {
        state.text_color = text_color;
        persist_and_report(&mut state, addon_dir);
    }

    let mut icon_color = state.icon_color;
    if ColorEdit::new("Icon color", &mut icon_color).build(ui) {
        state.icon_color = icon_color;
        persist_and_report(&mut state, addon_dir);
    }

    ui.separator();
    if state.session.has_data() {
        if ui.button("Reset Session") {
            state.session.reset();
        }
    } else {
        ui.text("Reset Session (available after the first successful poll)");
    }
}

fn persist_and_report(state: &mut AppState, addon_dir: &Path) {
    let config = config_from_state(state);
    if let Err(err) = save_config(addon_dir, &config) {
        log::warn!("failed to save session tracker config: {err}");
        state.status = PollStatus::Error(format!("failed to save config: {err}"));
    }
}
