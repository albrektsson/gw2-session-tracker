use nexus::imgui::{Image, Ui};
use std::path::Path;
use session_tracker_core::stats::{pvp_rank_tier, StatDef};
use session_tracker_net::state::AppState;

use super::icons::embedded_icon_bytes;

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
///
/// Prefers a locally cached copy (populated by the poller, see
/// `icon_cache`) over the network: identical content either way, since
/// the cache is keyed by the same URL, so which path serves a given
/// render is invisible to the user - it just avoids a redundant fetch of
/// something already on disk.
fn render_icon(identifier: &str, icon_url: &str, cache_dir: &Path, icon_size: f32, ui: &Ui) {
    let cached_path = session_tracker_net::icon_cache::cache_path(cache_dir, icon_url);
    if cached_path.is_file()
        && let Some(texture) = nexus::texture::get_texture_or_create_from_file(identifier, &cached_path)
    {
        Image::new(texture.id(), [icon_size, icon_size]).build(ui);
        ui.same_line();
        return;
    }

    let Some((remote, endpoint)) = split_icon_url(icon_url) else {
        return;
    };
    if let Some(texture) = nexus::texture::get_texture_or_create_from_url(identifier, remote, endpoint) {
        Image::new(texture.id(), [icon_size, icon_size]).build(ui);
        ui.same_line();
    }
}

/// Renders a vendored monochrome icon (see `icons.rs`), tinted to match
/// the configured icon color - unlike the full-color GW2 icons rendered by
/// `render_icon`, which are never tinted.
fn render_embedded_icon(identifier: &str, bytes: &'static [u8], icon_size: f32, tint: [f32; 4], ui: &Ui) {
    if let Some(texture) = nexus::texture::get_texture_or_create_from_memory(identifier, bytes) {
        Image::new(texture.id(), [icon_size, icon_size]).tint_col(tint).build(ui);
        ui.same_line();
    }
}

/// Renders `stat`'s icon - the same resolution order the main HUD window
/// uses (live PvP rank badge, then vendored embedded icon, then a real
/// GW2 API icon) - so picker rows and the HUD stay visually consistent.
pub fn render_stat_icon(stat: &StatDef, state: &AppState, cache_dir: &Path, icon_size: f32, ui: &Ui) {
    if stat.id == "pvp_rank" {
        // A real, unique badge exists for this one stat (the in-game rank
        // insignia) - use it instead of the vendored generic rank icon.
        let rank = state.session.lifetime_value("pvp_rank") as u32;
        let tier = pvp_rank_tier(rank);
        let identifier = format!("SESSION_TRACKER_ICON_pvp_rank_tier_{}", tier.min_rank);
        render_icon(&identifier, tier.icon_url, cache_dir, icon_size, ui);
    } else if let Some(bytes) = embedded_icon_bytes(stat.id) {
        let identifier = format!("SESSION_TRACKER_ICON_EMBED_{}", stat.id);
        render_embedded_icon(&identifier, bytes, icon_size, state.icon_color, ui);
    } else if let Some(icon_url) = stat.icon_url {
        let identifier = format!("SESSION_TRACKER_ICON_{icon_url}");
        render_icon(&identifier, icon_url, cache_dir, icon_size, ui);
    }
}
