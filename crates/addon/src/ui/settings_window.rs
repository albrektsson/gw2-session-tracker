use nexus::imgui::Ui;
use std::{
    cell::Cell,
    path::Path,
    sync::{atomic::AtomicBool, Arc, Mutex},
};
use session_tracker_core::config::{save_config, Config};
use session_tracker_net::state::{AppState, PollStatus};

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

pub fn render_settings_window(ui: &Ui, shared: &Arc<Mutex<AppState>>, addon_dir: &Path) {
    nexus::imgui::Window::new("Session Tracker Settings").build(ui, || {
        API_KEY_INPUT.with(|input| {
            let mut buf = input.borrow_mut();

            if !SEEDED.with(|seeded| seeded.get()) {
                if let Some(existing) = &shared.lock().unwrap().api_key {
                    *buf = existing.clone();
                }
                SEEDED.with(|seeded| seeded.set(true));
            }

            ui.text("GW2 API key (needs account, characters, progression scopes):");
            ui.input_text("##api_key", &mut buf).password(true).build();

            if ui.button("Save") {
                let mut state = shared.lock().unwrap();
                let trimmed = buf.trim();
                if trimmed.is_empty() {
                    state.status = PollStatus::Error("API key can't be empty".to_string());
                } else {
                    state.api_key = Some(trimmed.to_string());
                    let config = Config {
                        api_key: Some(trimmed.to_string()),
                    };
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

        let state = shared.lock().unwrap();
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
    });
}
