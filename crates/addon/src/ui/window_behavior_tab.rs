use nexus::imgui::Ui;
use session_tracker_core::config::WindowAnchor;

use crate::app_handle::AppHandle;

fn anchor_label(anchor: WindowAnchor) -> &'static str {
    match anchor {
        WindowAnchor::TopLeft => "Top Left",
        WindowAnchor::TopRight => "Top Right",
        WindowAnchor::BottomLeft => "Bottom Left",
        WindowAnchor::BottomRight => "Bottom Right",
    }
}

/// Recomputes `window_offset` so the window's current absolute screen
/// position is preserved under the new anchor corner, rather than
/// resetting to the default offset - applies `anchor_offset_position`
/// (self-inverse per fixed anchor) with the old anchor to recover the
/// absolute position, then again with the new anchor to derive the offset
/// that reproduces it.
fn change_anchor(ui: &Ui, app: &AppHandle, new_anchor: WindowAnchor) {
    let (old_anchor, old_offset, window_size) = {
        let state = app.lock();
        (state.config.window_anchor, state.config.window_offset, state.main_window_size)
    };
    let display_size = ui.io().display_size;
    let absolute = super::main_window::anchor_offset_position(old_anchor, old_offset, window_size, display_size);
    let new_offset = super::main_window::anchor_offset_position(new_anchor, absolute, window_size, display_size);
    app.mutate_and_persist(|state| {
        state.config.window_anchor = new_anchor;
        state.config.window_offset = new_offset;
    });
}

pub fn render_window_behavior_tab(ui: &Ui, app: &AppHandle) {
    ui.text("Anchor corner:");
    let current_anchor = app.lock().config.window_anchor;
    for anchor in [WindowAnchor::TopLeft, WindowAnchor::TopRight, WindowAnchor::BottomLeft, WindowAnchor::BottomRight] {
        let mut value = current_anchor;
        if ui.radio_button(anchor_label(anchor), &mut value, anchor) {
            change_anchor(ui, app, anchor);
        }
        ui.same_line();
    }
    ui.new_line();

    ui.separator();

    let mut window_drag_enabled = app.lock().config.window_drag_enabled;
    if ui.checkbox("Unlock window position (drag with mouse to move)", &mut window_drag_enabled) {
        app.mutate_and_persist(|state| {
            state.config.window_drag_enabled = window_drag_enabled;
            if window_drag_enabled {
                state.config.click_through_enabled = false;
            }
        });
    }

    let mut click_through_enabled = app.lock().config.click_through_enabled;
    if ui.checkbox("Click-through main window", &mut click_through_enabled) {
        app.mutate_and_persist(|state| {
            state.config.click_through_enabled = click_through_enabled;
            if click_through_enabled {
                state.config.window_drag_enabled = false;
            }
        });
    }
    ui.text_disabled("Click-through and drag are mutually exclusive - enabling one disables the other.");

    ui.separator();

    let mut menu_icon_enabled = app.lock().config.menu_icon_enabled;
    if ui.checkbox("Show Nexus quick-access icon", &mut menu_icon_enabled) {
        app.mutate_and_persist(|state| state.config.menu_icon_enabled = menu_icon_enabled);
        crate::set_quick_access_enabled(menu_icon_enabled);
    }
}
