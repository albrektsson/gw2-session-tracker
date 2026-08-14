use nexus::imgui::Ui;
use std::cell::{Cell, RefCell};
use session_tracker_core::format::validate_coin_format;
use session_tracker_net::state::PollStatus;

use crate::app_handle::AppHandle;

// Same seed-once-then-let-the-user-type convention as the API key field in
// options_tabs.rs's General tab.
thread_local! {
    static COIN_FORMAT_INPUT: RefCell<String> = const { RefCell::new(String::new()) };
    static SEEDED: Cell<bool> = const { Cell::new(false) };
}

pub fn render_formatting_tab(ui: &Ui, app: &AppHandle) {
    COIN_FORMAT_INPUT.with(|input| {
        let mut buf = input.borrow_mut();
        if !SEEDED.with(|seeded| seeded.get()) {
            *buf = app.lock().config.coin_format.clone();
            SEEDED.with(|seeded| seeded.set(true));
        }

        ui.text("Gold format pattern ({g}/{s}/{c} tokens, e.g. \"{g}g {s}s {c}c\"):");
        ui.input_text("##coin_format", &mut buf).build();
        if ui.button("Save##coin_format_save") {
            match validate_coin_format(&buf) {
                Ok(()) => {
                    let pattern = buf.clone();
                    app.mutate_and_persist(move |state| state.config.coin_format = pattern);
                }
                Err(err) => {
                    app.lock().status = PollStatus::Error(format!("invalid coin format: {err}"));
                }
            }
        }
    });

    {
        let state = app.lock();
        if let PollStatus::Error(err) = &state.status {
            ui.text_colored([1.0, 0.4, 0.4, 1.0], format!("Error: {err}"));
        }
    }

    ui.separator();

    let mut hide_zero_stats = app.lock().config.hide_zero_stats;
    if ui.checkbox("Hide stats with zero Session and Lifetime value", &mut hide_zero_stats) {
        app.mutate_and_persist(|state| state.config.hide_zero_stats = hide_zero_stats);
    }

    let mut show_last_updated_banner = app.lock().config.show_last_updated_banner;
    if ui.checkbox("Show \"Last updated\" banner", &mut show_last_updated_banner) {
        app.mutate_and_persist(|state| state.config.show_last_updated_banner = show_last_updated_banner);
    }
}
