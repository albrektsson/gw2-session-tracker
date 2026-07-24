mod ui;

use nexus::{
    gui::{register_render, render, RenderType},
    imgui::Ui,
    keybind::{keybind_handler, register_keybind_with_string},
    paths::get_addon_dir,
    AddonFlags,
};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};
use ui::main_window::{render_main_window, SHOW_MAIN};
use ui::settings_window::{render_settings_window, SHOW_SETTINGS};
use session_tracker_core::config::load_config;
use session_tracker_net::{
    gw2_client::fetch_snapshot,
    state::{AppState, Poller},
};

static SHARED_STATE: OnceLock<Arc<Mutex<AppState>>> = OnceLock::new();
static ADDON_DIR: OnceLock<PathBuf> = OnceLock::new();
static POLLER: OnceLock<Mutex<Poller>> = OnceLock::new();

/// Default keybind for toggling the settings window, used both to
/// register it and (since Nexus's addon API has no way to read back a
/// keybind the user has since rebound) as a "default" hint in UI copy.
pub(crate) const SETTINGS_KEYBIND_DEFAULT: &str = "ALT+SHIFT+E";

nexus::export! {
    name: "Session Tracker",
    signature: -0x57565757,
    load,
    unload,
    flags: AddonFlags::None,
    log_filter: "info",
}

fn load() {
    log::info!("Session Tracker addon loading");

    let addon_dir = get_addon_dir("session_tracker").expect("invalid addon dir");
    std::fs::create_dir_all(&addon_dir).expect("failed to create addon dir");
    let config = load_config(&addon_dir);
    log::info!(
        "loaded config from {}, api key present: {}",
        addon_dir.display(),
        config.api_key.is_some()
    );

    let shared = Arc::new(Mutex::new(AppState::new(
        config.api_key,
        config.selected_stats,
        config.background_opacity,
        config.text_scale,
        config.bold_text,
        config.text_color,
    )));
    if SHARED_STATE.set(shared).is_err() {
        panic!("load() called twice without unload()");
    }
    if ADDON_DIR.set(addon_dir).is_err() {
        panic!("load() called twice without unload()");
    }

    register_render(RenderType::Render, render!(render_frame)).revert_on_unload();

    let toggle_settings = keybind_handler!(|_id, is_release| {
        if !is_release {
            let current = SHOW_SETTINGS.load(std::sync::atomic::Ordering::Relaxed);
            SHOW_SETTINGS.store(!current, std::sync::atomic::Ordering::Relaxed);
        }
    });
    register_keybind_with_string(
        "SESSION_TRACKER_TOGGLE_SETTINGS",
        toggle_settings,
        SETTINGS_KEYBIND_DEFAULT,
    )
    .revert_on_unload();

    let toggle_main = keybind_handler!(|_id, is_release| {
        if !is_release {
            let current = SHOW_MAIN.load(std::sync::atomic::Ordering::Relaxed);
            SHOW_MAIN.store(!current, std::sync::atomic::Ordering::Relaxed);
        }
    });
    register_keybind_with_string("SESSION_TRACKER_TOGGLE_MAIN", toggle_main, "ALT+SHIFT+W")
        .revert_on_unload();

    let poller = Poller::spawn(
        SHARED_STATE.get().expect("just set above").clone(),
        Duration::from_secs(60),
        |api_key| fetch_snapshot(api_key).map_err(|err| err.to_string()),
    );
    POLLER
        .set(Mutex::new(poller))
        .unwrap_or_else(|_| panic!("load() called twice without unload()"));
}

fn unload() {
    log::info!("Session Tracker addon unloading");
    if let Some(poller) = POLLER.get() {
        poller.lock().unwrap().stop();
    }
}

/// Render callback for the main render pass. `nexus::gui::render!` requires
/// a plain, non-capturing `fn(&Ui)` (it stores the callback in a `const`),
/// so shared state is read from the module-level `OnceLock`s rather than
/// captured in a closure.
fn render_frame(ui: &Ui) {
    let shared = SHARED_STATE.get().expect("load() sets SHARED_STATE");

    if SHOW_SETTINGS.load(std::sync::atomic::Ordering::Relaxed) {
        let addon_dir = ADDON_DIR.get().expect("load() sets ADDON_DIR");
        render_settings_window(ui, shared, addon_dir);
    }

    if SHOW_MAIN.load(std::sync::atomic::Ordering::Relaxed) {
        render_main_window(ui, shared);
    }
}
