use nexus::imgui::{Condition, MouseButton, StyleColor, StyleVar, Ui};
use std::path::Path;
use std::time::Instant;
use session_tracker_core::{
    config::{RowField, WindowAnchor},
    format::{format_coin, format_distance, format_duration, format_ratio, format_thousands},
    map_context::MapGroup,
    stat_list::resolve_selected_stats,
    stats::{self, StatDef},
};
use session_tracker_net::state::{AppState, PollStatus};

use crate::app_handle::AppHandle;
use super::stat_icon::render_stat_icon;

const ICON_SIZE: f32 = 18.0;
const STALE_DATA_THRESHOLD_SECS: u64 = 5 * 60;

/// GW2's wallet API reports "Coin" (gold) as a raw copper count needing the
/// gold/silver/copper breakdown to be readable; Session Timer/Combat Time
/// are elapsed seconds needing `HH:MM:SS`; Distance Traveled is meters
/// needing km. Everything else - including a Session Rate for any of
/// these ids - is formatted the same way its Session/Lifetime Value would
/// be, since it's the same underlying quantity.
fn format_value(id: &str, value: f64, coin_format: &str) -> String {
    match id {
        "gold" => format_coin(value, coin_format),
        "kdr" | "pvp_kdr" => format_ratio(value),
        "session_timer" | "combat_time" => format_duration(value),
        "distance_traveled" => format_distance(value),
        _ => format_thousands(value),
    }
}

/// Draws `text` at the cursor, optionally faked-bold by drawing it twice
/// with a 1px offset (there's no bold font loaded, so this is the
/// zero-asset approximation). Uses the draw list directly rather than
/// `ui.text_colored` so the double-draw doesn't advance the layout cursor
/// twice; `ui.dummy` reserves the correct single-width space afterward,
/// which also keeps this hoverable for the tooltip like a normal item.
fn draw_text(ui: &Ui, color: [f32; 4], text: &str, bold: bool) {
    let pos = ui.cursor_screen_pos();
    let draw_list = ui.get_window_draw_list();
    if bold {
        draw_list.add_text([pos[0] + 1.0, pos[1]], color, text);
    }
    draw_list.add_text(pos, color, text);
    ui.dummy(ui.calc_text_size(text));
}

/// A stat is hidden by `hide_zero_stats` only when both its Session Value
/// and Lifetime Value are zero; a stat with no Lifetime Value (the
/// MumbleLink stats) falls back to judging Session Value alone.
fn should_hide_when_zero(state: &AppState, stat: &StatDef) -> bool {
    if state.session.session_amount(stat.id) != 0.0 {
        return false;
    }
    if stats::has_lifetime(stat.id) {
        state.session.lifetime_value(stat.id) == 0.0
    } else {
        true
    }
}

/// Which of `row_fields` actually apply to `stat_id` - Lifetime and Rate
/// are gated by `stats::has_lifetime`/`stats::has_rate`; Icon, Name, and
/// Session always apply. An inapplicable field is omitted entirely from
/// the row (its value and neighboring separator both drop), not shown as
/// a placeholder. Keeps each kept field's original `row_fields` index
/// alongside it, so a skipped field's neighboring gaps can still be
/// resolved back to `Config::row_separator_visible` (see `render_row`).
fn applicable_row_fields(row_fields: &[RowField], stat_id: &str) -> Vec<(usize, RowField)> {
    row_fields
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, field)| match field {
            RowField::Lifetime => stats::has_lifetime(stat_id),
            RowField::Rate => stats::has_rate(stat_id),
            RowField::Icon | RowField::Name | RowField::Session => true,
        })
        .collect()
}

enum RowSegment {
    Icon,
    Text { color: [f32; 4], text: String },
}

fn build_row_segments(state: &AppState, stat: &StatDef) -> Vec<(usize, RowField, RowSegment)> {
    let coin_format = state.config.coin_format.as_str();
    applicable_row_fields(&state.config.row_fields, stat.id)
        .into_iter()
        .map(|(index, field)| {
            let segment = match field {
                RowField::Icon => RowSegment::Icon,
                RowField::Name => RowSegment::Text {
                    color: state.config.label_color,
                    text: stat.display_name.to_string(),
                },
                RowField::Session => RowSegment::Text {
                    color: state.config.value_color,
                    text: format_value(stat.id, state.session.session_amount(stat.id), coin_format),
                },
                RowField::Lifetime => RowSegment::Text {
                    color: state.config.value_color,
                    text: format_value(stat.id, state.session.lifetime_value(stat.id), coin_format),
                },
                RowField::Rate => RowSegment::Text {
                    color: state.config.value_color,
                    text: format_value(stat.id, state.session.displayed_rate(stat.id), coin_format),
                },
            };
            (index, field, segment)
        })
        .collect()
}

/// Renders one stat's composable row, honoring `Config::row_fields`/
/// `row_separator`/`row_separator_visible` and per-stat field
/// applicability. Returns whether the row's last segment was an Icon:
/// `render_stat_icon` leaves a pending `same_line()` the caller must
/// close with `ui.new_line()` rather than letting it bleed into the next
/// row.
fn render_row(ui: &Ui, state: &AppState, stat: &StatDef, cache_dir: &Path, icon_size: f32) -> bool {
    let segments = build_row_segments(state, stat);
    let mut fixed_column_x: Option<f32> = None;
    let mut last_was_icon = false;
    let mut prev_index: Option<usize> = None;

    for (index, field, segment) in segments.iter() {
        if let Some(gap_index) = prev_index {
            match fixed_column_x.take() {
                Some(x) => ui.same_line_with_pos(x),
                None => ui.same_line(),
            }
            // The gap between this segment and the previous visible one is
            // keyed by the previous segment's own original `row_fields`
            // index (its outgoing gap) - see `Config::remove_row_field`,
            // which merges gaps under the same "keep the left one" rule
            // when a field is removed from the list entirely.
            let visible = state.config.row_separator_visible.get(gap_index).copied().unwrap_or(true);
            if visible {
                draw_text(ui, state.config.value_color, &state.config.row_separator, false);
                ui.same_line();
            }
        }
        prev_index = Some(*index);

        match segment {
            RowSegment::Icon => {
                render_stat_icon(stat, state, cache_dir, icon_size, ui);
                last_was_icon = true;
            }
            RowSegment::Text { color, text } => {
                let start_x = ui.cursor_pos()[0];
                draw_text(ui, *color, text, state.config.bold_text);
                last_was_icon = false;
                if *field == RowField::Name && state.config.fix_label_width {
                    fixed_column_x = Some(start_x + state.config.label_width);
                }
            }
        }
    }

    last_was_icon
}

const HISTORY_TOOLTIP_ROWS: usize = 10;

/// Rich hover tooltip for one stat: icon+name header, a Lifetime section
/// (omitted for the MumbleLink stats - `stats::has_lifetime`), a Session
/// section (Value, Rate when `stats::has_rate` applies, and the session's
/// overall elapsed Duration - omitted for Session Timer, whose Value line
/// already is that same number), and a history table of the most recent
/// History Snapshots.
fn render_stat_tooltip(ui: &Ui, state: &AppState, stat: &StatDef, cache_dir: &Path, icon_size: f32) {
    ui.tooltip(|| {
        render_stat_icon(stat, state, cache_dir, icon_size, ui);
        ui.text(stat.display_name);
        ui.separator();

        let coin_format = state.config.coin_format.as_str();

        if stats::has_lifetime(stat.id) {
            ui.text("Lifetime");
            let value = format_value(stat.id, state.session.lifetime_value(stat.id), coin_format);
            ui.text(format!("  Value: {value}"));
        }

        ui.text("Session");
        let session_value = format_value(stat.id, state.session.session_amount(stat.id), coin_format);
        ui.text(format!("  Value: {session_value}"));
        if stats::has_rate(stat.id) {
            let rate = format_value(stat.id, state.session.displayed_rate(stat.id), coin_format);
            ui.text(format!("  Rate: {rate}/hr"));
        }
        // Session Timer's own Value line above *is* the session's elapsed
        // time, so a Duration line here would just repeat it.
        if stat.id != "session_timer" {
            let duration = format_duration(state.session.elapsed().as_secs_f64());
            ui.text(format!("  Duration: {duration}"));
        }

        render_history_table(ui, state, stat);
    });
}

fn render_history_table(ui: &Ui, state: &AppState, stat: &StatDef) {
    let entries = state.session.history().entries();
    if entries.is_empty() {
        return;
    }

    ui.separator();
    ui.text("History");

    let show_rate = stats::has_rate(stat.id);
    let column_count = if show_rate { 3 } else { 2 };
    let coin_format = state.config.coin_format.as_str();

    let Some(_table) = ui.begin_table("session_tracker_history", column_count) else {
        return;
    };

    ui.table_next_row();
    ui.table_next_column();
    ui.text("Time");
    ui.table_next_column();
    ui.text("Value");
    if show_rate {
        ui.table_next_column();
        ui.text("Rate");
    }

    for snapshot in entries.iter().rev().take(HISTORY_TOOLTIP_ROWS) {
        let value = snapshot.values.get(stat.id).copied().unwrap_or(0.0);
        let elapsed_hours = snapshot.elapsed.as_secs_f64() / 3600.0;

        ui.table_next_row();
        ui.table_next_column();
        ui.text(format_duration(snapshot.elapsed.as_secs_f64()));
        ui.table_next_column();
        ui.text(format_value(stat.id, value, coin_format));
        if show_rate {
            ui.table_next_column();
            let rate = if elapsed_hours > 0.0 { value / elapsed_hours } else { 0.0 };
            ui.text(format_value(stat.id, rate, coin_format));
        }
    }
}

/// Self-inverse for a fixed anchor: converts an offset-from-anchor to an
/// absolute top-left window position, or (called again with the same
/// anchor) an absolute position back to an offset - same formula either
/// direction, since solving `display - size - x = y` for `x` given `y`
/// yields the same formula back. Also used to recompute `window_offset`
/// when the anchor itself changes: apply with the *old* anchor to turn
/// the old offset into an absolute position, then apply again with the
/// *new* anchor to turn that absolute position into the new offset.
pub(crate) fn anchor_offset_position(
    anchor: WindowAnchor,
    value: [f32; 2],
    window_size: [f32; 2],
    display_size: [f32; 2],
) -> [f32; 2] {
    let far = |value: f32, size: f32, display: f32| display - size - value;
    match anchor {
        WindowAnchor::TopLeft => value,
        WindowAnchor::TopRight => [far(value[0], window_size[0], display_size[0]), value[1]],
        WindowAnchor::BottomLeft => [value[0], far(value[1], window_size[1], display_size[1])],
        WindowAnchor::BottomRight => [
            far(value[0], window_size[0], display_size[0]),
            far(value[1], window_size[1], display_size[1]),
        ],
    }
}

pub fn render_main_window(ui: &Ui, app: &AppHandle) {
    let cache_dir = session_tracker_net::icon_cache::cache_dir(app.addon_dir());
    let mut state = app.lock();

    let position = if state.config.window_drag_enabled {
        None
    } else {
        Some(anchor_offset_position(
            state.config.window_anchor,
            state.config.window_offset,
            state.main_window_size,
            ui.io().display_size,
        ))
    };

    let bg = state.config.background_color;
    let color_token = ui.push_style_color(StyleColor::WindowBg, [bg[0], bg[1], bg[2], state.config.background_opacity]);
    let padding_token = ui.push_style_var(StyleVar::WindowPadding([state.config.padding, state.config.padding]));

    let mut window = nexus::imgui::Window::new("Session Tracker")
        .no_decoration()
        .always_auto_resize(true)
        .mouse_inputs(!state.config.click_through_enabled);
    if let Some(pos) = position {
        window = window.position(pos, Condition::Always);
    }
    if state.config.fixed_window_height {
        window = window
            .size_constraints([0.0, state.config.window_height], [f32::MAX, state.config.window_height])
            .scroll_bar(true);
    }

    let mut persist_offset = false;

    window.build(ui, || {
        state.main_window_size = ui.window_size();

        if state.config.window_drag_enabled {
            let offset = anchor_offset_position(
                state.config.window_anchor,
                ui.window_pos(),
                state.main_window_size,
                ui.io().display_size,
            );
            state.config.window_offset = offset;
            if ui.is_mouse_released(MouseButton::Left) {
                persist_offset = true;
            }
        }

        // arcdps-imgui doesn't expose a safe wrapper for this - it's a
        // thin, well-established Dear ImGui call (per-window text
        // scale), same idiom as the style-color/style-var pushes above.
        unsafe {
            nexus::imgui::sys::igSetWindowFontScale(state.config.text_scale);
        }

        match &state.status {
            PollStatus::AwaitingApiKey => {
                ui.text("No API key configured yet.");
                ui.text("Open Session Tracker's Options in Nexus's addon list to add one.");
                return;
            }
            PollStatus::Pending | PollStatus::Error(_) | PollStatus::Ok => {}
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

        let mut selected = resolve_selected_stats(&state.config.selected_stats);
        if let Some(group) = state.current_map_group {
            let mode_ids = match group {
                MapGroup::Wvw => &state.config.wvw_selected_stats,
                MapGroup::Pvp => &state.config.pvp_selected_stats,
                MapGroup::Pve => &state.config.pve_selected_stats,
            };
            for stat in resolve_selected_stats(mode_ids) {
                if !selected.iter().any(|s| s.id == stat.id) {
                    selected.push(stat);
                }
            }
        }
        if state.config.hide_zero_stats {
            selected.retain(|stat| !should_hide_when_zero(&state, stat));
        }
        if selected.is_empty() {
            ui.text("No stats selected. Open Session Tracker's Options in Nexus's addon list to pick some.");
            return;
        }

        let icon_size = ICON_SIZE * state.config.text_scale;

        for stat in selected {
            let last_was_icon = render_row(ui, &state, stat, &cache_dir, icon_size);
            if ui.is_item_hovered() {
                render_stat_tooltip(ui, &state, stat, &cache_dir, icon_size);
            }
            if state.config.window_right_margin > 0.0 {
                ui.same_line();
                ui.dummy([state.config.window_right_margin, 0.0]);
            } else if last_was_icon {
                ui.new_line();
            }
        }
    });

    color_token.pop();
    padding_token.pop();

    drop(state);
    if persist_offset {
        app.persist();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use session_tracker_core::{config::Config, stats::STAT_CATALOG};

    #[test]
    fn top_left_offset_is_the_position_directly() {
        let pos = anchor_offset_position(WindowAnchor::TopLeft, [20.0, 30.0], [100.0, 50.0], [1920.0, 1080.0]);
        assert_eq!(pos, [20.0, 30.0]);
    }

    #[test]
    fn top_right_offset_is_measured_from_the_right_edge() {
        let pos = anchor_offset_position(WindowAnchor::TopRight, [20.0, 30.0], [100.0, 50.0], [1920.0, 1080.0]);
        assert_eq!(pos, [1800.0, 30.0]);
    }

    #[test]
    fn bottom_left_offset_is_measured_from_the_bottom_edge() {
        let pos = anchor_offset_position(WindowAnchor::BottomLeft, [20.0, 30.0], [100.0, 50.0], [1920.0, 1080.0]);
        assert_eq!(pos, [20.0, 1000.0]);
    }

    #[test]
    fn bottom_right_offset_is_measured_from_both_far_edges() {
        let pos = anchor_offset_position(WindowAnchor::BottomRight, [20.0, 30.0], [100.0, 50.0], [1920.0, 1080.0]);
        assert_eq!(pos, [1800.0, 1000.0]);
    }

    #[test]
    fn applying_the_same_anchor_twice_is_self_inverse() {
        let window_size = [100.0, 50.0];
        let display_size = [1920.0, 1080.0];
        let absolute = anchor_offset_position(WindowAnchor::BottomRight, [20.0, 30.0], window_size, display_size);
        let recovered_offset = anchor_offset_position(WindowAnchor::BottomRight, absolute, window_size, display_size);
        assert_eq!(recovered_offset, [20.0, 30.0]);
    }

    #[test]
    fn changing_anchor_preserves_absolute_position() {
        let window_size = [100.0, 50.0];
        let display_size = [1920.0, 1080.0];
        let old_offset = [20.0, 30.0];
        let absolute = anchor_offset_position(WindowAnchor::TopLeft, old_offset, window_size, display_size);
        let new_offset = anchor_offset_position(WindowAnchor::BottomRight, absolute, window_size, display_size);
        let recovered_absolute = anchor_offset_position(WindowAnchor::BottomRight, new_offset, window_size, display_size);
        assert_eq!(recovered_absolute, absolute);
    }

    #[test]
    fn applicable_row_fields_omits_lifetime_and_rate_for_session_timer() {
        let fields = vec![RowField::Icon, RowField::Session, RowField::Lifetime, RowField::Rate];
        assert_eq!(
            applicable_row_fields(&fields, "session_timer"),
            vec![(0, RowField::Icon), (1, RowField::Session)]
        );
    }

    #[test]
    fn applicable_row_fields_omits_only_rate_for_kdr() {
        let fields = vec![RowField::Icon, RowField::Session, RowField::Lifetime, RowField::Rate];
        assert_eq!(
            applicable_row_fields(&fields, "kdr"),
            vec![(0, RowField::Icon), (1, RowField::Session), (2, RowField::Lifetime)]
        );
    }

    #[test]
    fn applicable_row_fields_keeps_everything_for_an_ordinary_stat() {
        let fields = vec![RowField::Icon, RowField::Name, RowField::Session, RowField::Lifetime, RowField::Rate];
        assert_eq!(applicable_row_fields(&fields, "gold"), fields.into_iter().enumerate().collect::<Vec<_>>());
    }

    #[test]
    fn applicable_row_fields_keeps_the_left_survivors_original_index_when_a_field_is_skipped() {
        let fields = vec![RowField::Icon, RowField::Lifetime, RowField::Session];
        assert_eq!(
            applicable_row_fields(&fields, "session_timer"),
            vec![(0, RowField::Icon), (2, RowField::Session)]
        );
    }

    #[test]
    fn should_hide_when_zero_is_false_if_session_value_is_nonzero() {
        let mut state = AppState::new(Config::default());
        state.session.update([("gold", 10.0)].into_iter().collect());
        state.session.update([("gold", 0.0)].into_iter().collect());
        let stat = STAT_CATALOG.iter().find(|s| s.id == "gold").unwrap();
        // lifetime is now 0, but session_value = 0 - 10 = -10 (nonzero) -
        // isolates that a nonzero session alone is enough to not hide,
        // independent of lifetime.
        assert!(!should_hide_when_zero(&state, stat));
    }

    #[test]
    fn should_hide_when_zero_is_true_if_both_session_and_lifetime_are_zero() {
        let mut state = AppState::new(Config::default());
        state.session.update([("gold", 0.0)].into_iter().collect());
        let stat = STAT_CATALOG.iter().find(|s| s.id == "gold").unwrap();
        assert!(should_hide_when_zero(&state, stat));
    }

    #[test]
    fn should_hide_when_zero_is_false_if_lifetime_is_nonzero_even_with_zero_session() {
        let mut state = AppState::new(Config::default());
        state.session.update([("gold", 100.0)].into_iter().collect());
        state.session.reset(); // rebaselines: session value is 0, lifetime stays 100
        let stat = STAT_CATALOG.iter().find(|s| s.id == "gold").unwrap();
        assert!(!should_hide_when_zero(&state, stat));
    }

    #[test]
    fn should_hide_when_zero_falls_back_to_session_alone_for_mumblelink_stats() {
        let state = AppState::new(Config::default());
        let stat = STAT_CATALOG.iter().find(|s| s.id == "session_timer").unwrap();
        assert!(should_hide_when_zero(&state, stat));
    }
}
