use nexus::imgui::{Image, Ui};
use nexus::texture::Texture;
use std::path::Path;
use session_tracker_core::stats::{pvp_rank_tier, StatDef};
use session_tracker_net::state::AppState;

use super::icons::embedded_icon_bytes;

/// Reserves `icon_size`x`icon_size` of row space and leaves a pending
/// `same_line()`, whether or not `texture` has finished loading yet -
/// `render_row` (main_window.rs) assumes every segment places exactly one
/// item, so a still-loading texture must still occupy its slot with a
/// placeholder rather than silently drawing nothing, which would leave a
/// dangling `same_line()` for the *next* stat's row to land on instead.
fn render_icon_slot(texture: Option<Texture>, tint: Option<[f32; 4]>, icon_size: f32, ui: &Ui) {
    match texture {
        Some(texture) => {
            let mut image = Image::new(texture.id(), [icon_size, icon_size]);
            if let Some(tint) = tint {
                image = image.tint_col(tint);
            }
            image.build(ui);
        }
        None => ui.dummy([icon_size, icon_size]),
    }
    ui.same_line();
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
///
/// Prefers a locally cached copy (populated by the poller, see
/// `icon_cache`) over the network: identical content either way, since
/// the cache is keyed by the same URL, so which path serves a given
/// render is invisible to the user - it just avoids a redundant fetch of
/// something already on disk.
fn render_icon(identifier: &str, icon_url: &str, cache_dir: &Path, icon_size: f32, ui: &Ui) {
    let cached_path = session_tracker_net::icon_cache::cache_path(cache_dir, icon_url);
    let texture = if cached_path.is_file() {
        nexus::texture::get_texture_or_create_from_file(identifier, &cached_path)
    } else {
        split_icon_url(icon_url)
            .and_then(|(remote, endpoint)| nexus::texture::get_texture_or_create_from_url(identifier, remote, endpoint))
    };
    render_icon_slot(texture, None, icon_size, ui);
}

/// Renders a vendored monochrome icon (see `icons.rs`), tinted to match
/// the configured icon color - unlike the full-color GW2 icons rendered by
/// `render_icon`, which are never tinted.
fn render_embedded_icon(identifier: &str, bytes: &'static [u8], icon_size: f32, tint: [f32; 4], ui: &Ui) {
    let texture = nexus::texture::get_texture_or_create_from_memory(identifier, bytes);
    render_icon_slot(texture, Some(tint), icon_size, ui);
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
        render_embedded_icon(&identifier, bytes, icon_size, state.config.icon_color, ui);
    } else if let Some(icon_url) = stat.icon_url {
        let identifier = format!("SESSION_TRACKER_ICON_{icon_url}");
        render_icon(&identifier, icon_url, cache_dir, icon_size, ui);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus::imgui::{Condition, Context, Window};

    /// Mirrors `render_row`'s gap logic in `main_window.rs`: an icon
    /// segment followed by same-line text segments.
    fn render_row_icon_first(ui: &Ui) {
        render_icon_slot(None, None, 18.0, ui);
        ui.same_line();
        ui.text("Name");
        ui.same_line();
        ui.text("123");
    }

    #[test]
    fn a_still_loading_texture_does_not_swallow_the_next_rows_line_break() {
        let mut ctx = Context::create();
        {
            let io = ctx.io_mut();
            io.display_size = [1024.0, 768.0];
            io.delta_time = 1.0 / 60.0;
        }
        ctx.fonts().build_rgba32_texture();

        let ui = ctx.frame();
        let mut row_start_ys = Vec::new();
        Window::new("test")
            .position([0.0, 0.0], Condition::Always)
            .always_auto_resize(true)
            .build(&ui, || {
                for _ in 0..3 {
                    row_start_ys.push(ui.cursor_screen_pos()[1]);
                    render_row_icon_first(&ui);
                }
                row_start_ys.push(ui.cursor_screen_pos()[1]);
            });

        for pair in row_start_ys.windows(2) {
            assert!(
                pair[1] - pair[0] > 0.5,
                "two consecutive rows landed on the same line (breakline swallowed): {row_start_ys:?}"
            );
        }
    }
}
