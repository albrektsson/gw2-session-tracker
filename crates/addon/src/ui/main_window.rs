use nexus::imgui::{Image, Ui};
use std::{
    sync::{atomic::AtomicBool, Arc, Mutex},
    time::Instant,
};
use session_tracker_core::{
    format::{format_coin, format_thousands},
    stats::{pvp_rank_tier, resolve_selected_stats},
};
use session_tracker_net::state::{AppState, PollStatus};

pub static SHOW_MAIN: AtomicBool = AtomicBool::new(false);

const ICON_SIZE: f32 = 18.0;
const STALE_DATA_THRESHOLD_SECS: u64 = 5 * 60;

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

/// Splits a `render.guildwars2.com` icon URL into the `remote`/`endpoint`
/// pair Nexus's texture-from-URL API wants, e.g.
/// `"https://render.guildwars2.com/file/HASH/12345.png"` ->
/// `("https://render.guildwars2.com", "/file/HASH/12345.png")`.
fn split_icon_url(url: &str) -> Option<(&str, &str)> {
    let after_scheme = url.find("://")? + 3;
    let path_start = url[after_scheme..].find('/')? + after_scheme;
    Some((&url[..path_start], &url[path_start..]))
}

/// `identifier` must be stable for a given `icon_url`: Nexus's texture
/// cache is keyed by identifier, so reusing the same identifier for a
/// URL that later changes (e.g. a stat id, when the actual icon depends
/// on live state like a PvP rank tier) would keep serving the
/// first-ever-registered image forever.
fn render_icon(identifier: &str, icon_url: &str, icon_size: f32, ui: &Ui) {
    let Some((remote, endpoint)) = split_icon_url(icon_url) else {
        return;
    };
    if let Some(texture) = nexus::texture::get_texture_or_create_from_url(identifier, remote, endpoint) {
        Image::new(texture.id(), [icon_size, icon_size]).build(ui);
        ui.same_line();
    }
}

/// Draws `text` at the cursor, optionally faked-bold by drawing it twice
/// with a 1px offset (there's no bold font loaded, so this is the
/// zero-asset approximation). Uses the draw list directly rather than
/// `ui.text_colored` so the double-draw doesn't advance the layout cursor
/// twice; `ui.dummy` reserves the correct single-width space afterward,
/// which also keeps this hoverable for the tooltip like a normal item.
/// GW2's wallet API reports "Coin" (gold) as a raw copper count; every
/// other stat is a plain integer. Gold specifically needs the
/// gold/silver/copper breakdown to be readable.
fn format_value(id: &str, value: f64) -> String {
    if id == "gold" {
        format_coin(value)
    } else {
        format_thousands(value)
    }
}

fn draw_text(ui: &Ui, color: [f32; 4], text: &str, bold: bool) {
    let pos = ui.cursor_screen_pos();
    let draw_list = ui.get_window_draw_list();
    if bold {
        draw_list.add_text([pos[0] + 1.0, pos[1]], color, text);
    }
    draw_list.add_text(pos, color, text);
    ui.dummy(ui.calc_text_size(text));
}

pub fn render_main_window(ui: &Ui, shared: &Arc<Mutex<AppState>>) {
    let state = shared.lock().unwrap();
    nexus::imgui::Window::new("Session Tracker")
        .bg_alpha(state.background_opacity)
        .no_decoration()
        .always_auto_resize(true)
        .build(ui, || {
            // arcdps-imgui doesn't expose a safe wrapper for this - it's a
            // thin, well-established Dear ImGui call (per-window text
            // scale), same idiom as `.bg_alpha()`'s internal raw call.
            unsafe {
                nexus::imgui::sys::igSetWindowFontScale(state.text_scale);
            }

            match &state.status {
                PollStatus::AwaitingApiKey => {
                    ui.text("No API key configured yet.");
                    ui.text(format!(
                        "Open Settings (default keybind {}, rebindable in Nexus) to add one.",
                        crate::SETTINGS_KEYBIND_DEFAULT
                    ));
                    return;
                }
                PollStatus::Error(_) | PollStatus::Ok => {}
            }

            if !state.session.has_data() {
                ui.text("Waiting for first successful poll...");
                return;
            }

            if let Some(last_updated) = state.last_updated {
                let secs_ago = Instant::now().saturating_duration_since(last_updated).as_secs();
                let text = format!("Last updated {secs_ago}s ago");
                if secs_ago >= STALE_DATA_THRESHOLD_SECS {
                    ui.text_colored([1.0, 0.4, 0.4, 1.0], text);
                } else {
                    ui.text(text);
                }
            }

            let selected = resolve_selected_stats(&state.selected_stats);
            if selected.is_empty() {
                ui.text(format!(
                    "No stats selected. Open Settings (default keybind {}, rebindable in Nexus) to pick some.",
                    crate::SETTINGS_KEYBIND_DEFAULT
                ));
                return;
            }

            let icon_size = ICON_SIZE * state.text_scale;

            for stat in selected {
                if stat.id == "pvp_rank" {
                    // A real, unique badge exists for this one stat (the
                    // in-game rank insignia) - use it instead of the
                    // generic PvP category icon fallback in the catalog.
                    let rank = state.session.lifetime_value("pvp_rank") as u32;
                    let tier = pvp_rank_tier(rank);
                    let identifier = format!("SESSION_TRACKER_ICON_pvp_rank_tier_{}", tier.min_rank);
                    render_icon(&identifier, tier.icon_url, icon_size, ui);
                } else if let Some(icon_url) = stat.icon_url {
                    let identifier = format!("SESSION_TRACKER_ICON_{}", stat.id);
                    render_icon(&identifier, icon_url, icon_size, ui);
                }

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
                let lifetime_value = state.session.lifetime_value(stat.id);

                let text = format!("{} | {}", format_value(stat.id, session_value), format_value(stat.id, lifetime_value));
                draw_text(ui, state.text_color, &text, state.bold_text);
                if ui.is_item_hovered() {
                    ui.tooltip_text(stat.display_name);
                }
            }
        });
}
