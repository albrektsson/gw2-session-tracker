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
    sync::{Arc, Mutex},
    time::Duration,
};
use ui::main_window::{render_main_window, SHOW_MAIN};
use ui::settings_window::{render_settings_window, SHOW_SETTINGS};
use session_tracker_core::{
    config::{load_config, save_config},
    sync::lock_recover,
};
use session_tracker_net::{
    gw2_client::fetch_snapshot,
    state::{AppState, Poller},
};

/// Everything `load()` sets up and `unload()` tears down, held behind a
/// single resettable `Mutex<Option<_>>` rather than separate `OnceLock`s —
/// a `OnceLock` can only ever be set once, so if Nexus ever re-invokes
/// `load()` in the same process without a true `FreeLibrary`/`LoadLibrary`
/// cycle in between, the old `OnceLock`s would panic instead of allowing
/// a clean reload. Dropping the `Addon` (via `Option::take` in `unload()`)
/// stops the poller through `Poller`'s `Drop` impl.
struct Addon {
    shared: Arc<Mutex<AppState>>,
    addon_dir: PathBuf,
    // Never read after construction; kept alive so its `Drop` stops the
    // polling thread when the `Addon` is dropped in `unload()`.
    #[allow(dead_code)]
    poller: Poller,
}

static ADDON: Mutex<Option<Addon>> = Mutex::new(None);

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

    let addon_dir = match get_addon_dir("session_tracker") {
        Ok(dir) => dir,
        Err(err) => {
            log::error!("failed to resolve addon directory: {err}; Session Tracker will not load");
            return;
        }
    };
    if let Err(err) = std::fs::create_dir_all(&addon_dir) {
        log::error!("failed to create addon dir {}: {err}; Session Tracker will not load", addon_dir.display());
        return;
    }
    let config = load_config(&addon_dir);
    log::info!(
        "loaded config from {}, api key present: {}",
        addon_dir.display(),
        config.api_key.is_some()
    );

    SHOW_SETTINGS.store(config.show_settings, std::sync::atomic::Ordering::Relaxed);
    SHOW_MAIN.store(config.show_main, std::sync::atomic::Ordering::Relaxed);

    let shared = Arc::new(Mutex::new(AppState::new(
        config.api_key,
        config.selected_stats,
        config.wvw_selected_stats,
        config.pvp_selected_stats,
        config.pve_selected_stats,
        config.background_opacity,
        config.text_scale,
        config.bold_text,
        config.text_color,
        config.icon_color,
    )));

    register_render(RenderType::Render, render!(render_frame)).revert_on_unload();

    let toggle_settings = keybind_handler!(|_id, is_release| {
        if !is_release {
            let current = SHOW_SETTINGS.load(std::sync::atomic::Ordering::Relaxed);
            SHOW_SETTINGS.store(!current, std::sync::atomic::Ordering::Relaxed);
            persist_window_visibility();
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
            persist_window_visibility();
        }
    });
    register_keybind_with_string("SESSION_TRACKER_TOGGLE_MAIN", toggle_main, "ALT+SHIFT+W")
        .revert_on_unload();

    let icon_cache_dir = session_tracker_net::icon_cache::cache_dir(&addon_dir);
    let shared_for_icons = shared.clone();
    let poller = Poller::spawn(shared.clone(), Duration::from_secs(60), move |api_key, shutdown| {
        let result = fetch_snapshot(api_key, shutdown).map_err(|err| err.to_string());
        let icon_urls = {
            let state = lock_recover(&shared_for_icons);
            session_tracker_core::stats::icon_urls_for_selected(&state.selected_stats)
        };
        session_tracker_net::icon_cache::cache_missing_icons(&icon_cache_dir, &icon_urls, shutdown);
        result
    });

    let mut addon = lock_recover(&ADDON);
    if addon.is_some() {
        panic!("load() called twice without unload()");
    }
    *addon = Some(Addon {
        shared,
        addon_dir,
        poller,
    });
}

fn persist_window_visibility() {
    let guard = lock_recover(&ADDON);
    let Some(addon) = guard.as_ref() else {
        return;
    };
    let state = lock_recover(&addon.shared);
    let config = ui::settings_window::config_from_state(&state);
    drop(state);
    if let Err(err) = save_config(&addon.addon_dir, &config) {
        log::warn!("failed to save session tracker config: {err}");
    }
}

fn unload() {
    log::info!("Session Tracker addon unloading");
    // Dropping the `Addon` stops the poller (`Poller::drop`).
    lock_recover(&ADDON).take();
}

/// Render callback for the main render pass. `nexus::gui::render!` requires
/// a plain, non-capturing `fn(&Ui)` (it stores the callback in a `const`),
/// so shared state is read from the module-level `ADDON` static rather than
/// captured in a closure.
///
/// `ADDON` can legitimately be `None` here: `register_render` runs before
/// `ADDON` is populated in `load()`, and `revert_on_unload()` deregisters
/// the render callback with Nexus asynchronously, so a render tick can still
/// land after `unload()` has already cleared `ADDON` via `.take()`. Both are
/// expected transient states, not a bug - skip the frame instead of
/// panicking.
fn render_frame(ui: &Ui) {
    let guard = lock_recover(&ADDON);
    let Some(addon) = guard.as_ref() else {
        return;
    };

    if let Some(link) = nexus::data_link::read_mumble_link() {
        let mut state = lock_recover(&addon.shared);
        state.session.sample_position(link.avatar.position);
        state.session.sample_combat_state(
            link.context.ui_state.contains(nexus::data_link::mumble::UiState::IS_IN_COMBAT),
        );
        state.current_map_group = session_tracker_core::map_context::map_group_for(link.context.map_type);
    }

    if SHOW_SETTINGS.load(std::sync::atomic::Ordering::Relaxed) {
        render_settings_window(ui, &addon.shared, &addon.addon_dir);
    }

    if SHOW_MAIN.load(std::sync::atomic::Ordering::Relaxed) {
        render_main_window(ui, &addon.shared, &addon.addon_dir);
    }
}
