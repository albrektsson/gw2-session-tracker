use nexus::imgui::{ColorEdit, Slider, Ui};
use std::cell::Cell;
use session_tracker_net::state::PollStatus;

use crate::app_handle::AppHandle;
use super::arrange_stats_tab::render_arrange_stats_tab;
use super::select_stats_tab::render_select_stats_tab;

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

pub fn render_settings_window(ui: &Ui, app: &AppHandle) {
    nexus::imgui::Window::new("Session Tracker Settings").build(ui, || {
        if let Some(_tabs) = ui.tab_bar("settings-tabs") {
            if let Some(_tab) = ui.tab_item("General") {
                render_general_tab(ui, app);
            }
            if let Some(_tab) = ui.tab_item("Select Stats") {
                render_select_stats_tab(ui, app);
            }
            if let Some(_tab) = ui.tab_item("Arrange Stats") {
                render_arrange_stats_tab(ui, app);
            }
        }
    });
}

fn render_general_tab(ui: &Ui, app: &AppHandle) {
    API_KEY_INPUT.with(|input| {
        let mut buf = input.borrow_mut();

        if !SEEDED.with(|seeded| seeded.get()) {
            if let Some(existing) = &app.lock().api_key {
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
                    state.api_key = Some(trimmed);
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
    ui.text("Main window background opacity:");
    let mut opacity = app.lock().background_opacity;
    if Slider::new("##background_opacity", 0.0f32, 1.0f32).build(ui, &mut opacity) {
        app.mutate_and_persist(|state| state.background_opacity = opacity);
    }

    ui.text("Main window text size:");
    let mut text_scale = app.lock().text_scale;
    if Slider::new("##text_scale", 0.5f32, 3.0f32).build(ui, &mut text_scale) {
        app.mutate_and_persist(|state| state.text_scale = text_scale);
    }

    let mut bold_text = app.lock().bold_text;
    if ui.checkbox("Bold text", &mut bold_text) {
        app.mutate_and_persist(|state| state.bold_text = bold_text);
    }

    let mut text_color = app.lock().text_color;
    if ColorEdit::new("Text color", &mut text_color).build(ui) {
        app.mutate_and_persist(|state| state.text_color = text_color);
    }

    let mut icon_color = app.lock().icon_color;
    if ColorEdit::new("Icon color", &mut icon_color).build(ui) {
        app.mutate_and_persist(|state| state.icon_color = icon_color);
    }

    ui.separator();
    let has_data = app.lock().session.has_data();
    if has_data {
        if ui.button("Reset Session") {
            app.lock().session.reset();
        }
    } else {
        ui.text("Reset Session (available after the first successful poll)");
    }
}
