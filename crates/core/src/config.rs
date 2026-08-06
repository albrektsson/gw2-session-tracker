use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path};

/// One slot in a Main Window row's composable field list (Row Format).
/// Omitted entirely (value and neighboring separator both drop) for a
/// stat that doesn't apply - see `stats::has_lifetime`/`stats::has_rate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RowField {
    Icon,
    Name,
    Session,
    Lifetime,
    Rate,
}

/// Which corner of the screen the Main Window is pinned to; paired with
/// `Config::window_offset`, a pixel offset from that corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum WindowAnchor {
    #[default]
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub api_key: Option<String>,
    #[serde(default = "default_selected_stats")]
    pub selected_stats: Vec<String>,
    #[serde(default)]
    pub wvw_selected_stats: Vec<String>,
    #[serde(default)]
    pub pvp_selected_stats: Vec<String>,
    #[serde(default)]
    pub pve_selected_stats: Vec<String>,
    #[serde(default = "default_background_opacity")]
    pub background_opacity: f32,
    #[serde(default = "default_text_scale")]
    pub text_scale: f32,
    #[serde(default)]
    pub bold_text: bool,
    #[serde(default = "default_label_value_color")]
    pub label_color: [f32; 4],
    #[serde(default = "default_label_value_color")]
    pub value_color: [f32; 4],
    #[serde(default = "default_label_value_color")]
    pub icon_color: [f32; 4],
    #[serde(default)]
    pub background_color: [f32; 3],
    #[serde(default)]
    pub show_main: bool,
    #[serde(default = "default_row_fields")]
    pub row_fields: Vec<RowField>,
    #[serde(default = "default_row_separator")]
    pub row_separator: String,
    /// One entry per gap between consecutive `row_fields` (length
    /// `row_fields.len().saturating_sub(1)`), controlling whether
    /// `row_separator` is drawn in that gap.
    #[serde(default = "default_row_separator_visible")]
    pub row_separator_visible: Vec<bool>,
    #[serde(default)]
    pub fixed_window_height: bool,
    #[serde(default = "default_window_height")]
    pub window_height: f32,
    #[serde(default)]
    pub window_right_margin: f32,
    #[serde(default = "default_padding")]
    pub padding: f32,
    #[serde(default)]
    pub fix_label_width: bool,
    #[serde(default = "default_label_width")]
    pub label_width: f32,
    #[serde(default)]
    pub window_anchor: WindowAnchor,
    #[serde(default = "default_window_offset")]
    pub window_offset: [f32; 2],
    #[serde(default = "default_window_drag_enabled")]
    pub window_drag_enabled: bool,
    #[serde(default = "default_menu_icon_enabled")]
    pub menu_icon_enabled: bool,
    #[serde(default)]
    pub click_through_enabled: bool,
    #[serde(default = "default_coin_format")]
    pub coin_format: String,
    #[serde(default)]
    pub hide_zero_stats: bool,
}

impl Config {
    /// Pads or truncates `row_separator_visible` to exactly
    /// `row_fields.len().saturating_sub(1)` entries, padding new gaps as
    /// visible. `row_separator_visible` shipped after `row_fields`, so a
    /// config saved before that (with a `row_fields` list already longer
    /// than the fresh default) would otherwise deserialize with too few
    /// gaps for its own field list - self-heals that on every load, and
    /// guards `remove_row_field` against indexing out of bounds.
    pub fn reconcile_row_separator_visible(&mut self) {
        let needed = self.row_fields.len().saturating_sub(1);
        self.row_separator_visible.resize(needed, true);
    }

    /// Adds `field` to the end of `row_fields` and appends a matching
    /// visible-by-default gap to `row_separator_visible`, keeping the two
    /// in sync.
    pub fn push_row_field(&mut self, field: RowField) {
        self.row_fields.push(field);
        self.reconcile_row_separator_visible();
    }

    /// Removes `row_fields[index]` and merges its neighboring gaps in
    /// `row_separator_visible`, keeping the left gap's value (or the
    /// remaining gap's value, when `index` is at either end).
    pub fn remove_row_field(&mut self, index: usize) {
        self.reconcile_row_separator_visible();
        self.row_fields.remove(index);
        if index < self.row_separator_visible.len() {
            self.row_separator_visible.remove(index);
        } else if index > 0 {
            self.row_separator_visible.remove(index - 1);
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key: None,
            selected_stats: default_selected_stats(),
            wvw_selected_stats: Vec::new(),
            pvp_selected_stats: Vec::new(),
            pve_selected_stats: Vec::new(),
            background_opacity: default_background_opacity(),
            text_scale: default_text_scale(),
            bold_text: false,
            label_color: default_label_value_color(),
            value_color: default_label_value_color(),
            icon_color: default_label_value_color(),
            background_color: [0.0, 0.0, 0.0],
            show_main: false,
            row_fields: default_row_fields(),
            row_separator: default_row_separator(),
            row_separator_visible: default_row_separator_visible(),
            fixed_window_height: false,
            window_height: default_window_height(),
            window_right_margin: 0.0,
            padding: default_padding(),
            fix_label_width: false,
            label_width: default_label_width(),
            window_anchor: WindowAnchor::default(),
            window_offset: default_window_offset(),
            window_drag_enabled: default_window_drag_enabled(),
            menu_icon_enabled: default_menu_icon_enabled(),
            click_through_enabled: false,
            coin_format: default_coin_format(),
            hide_zero_stats: false,
        }
    }
}

fn default_selected_stats() -> Vec<String> {
    ["kills", "deaths", "kdr", "wvw_rank"]
        .into_iter()
        .map(String::from)
        .collect()
}

fn default_background_opacity() -> f32 {
    0.35
}

fn default_text_scale() -> f32 {
    1.0
}

fn default_label_value_color() -> [f32; 4] {
    [1.0, 0.85, 0.3, 1.0]
}

/// Matches today's hardcoded main-window row shape (`icon, "session |
/// lifetime"`), so upgrading an existing config doesn't change what's
/// displayed.
fn default_row_fields() -> Vec<RowField> {
    vec![RowField::Icon, RowField::Session, RowField::Lifetime]
}

fn default_row_separator() -> String {
    "|".to_string()
}

/// Matches `default_row_fields`'s two gaps: hidden between Icon and
/// Session, shown between Session and Lifetime.
fn default_row_separator_visible() -> Vec<bool> {
    vec![false, true]
}

fn default_window_height() -> f32 {
    200.0
}

fn default_padding() -> f32 {
    8.0
}

fn default_label_width() -> f32 {
    80.0
}

fn default_window_offset() -> [f32; 2] {
    [20.0, 20.0]
}

/// Unlocked by default: forcing a position (locked mode) would otherwise
/// override wherever Dear ImGui already remembers the window from before
/// this addon had any position-management code of its own.
fn default_window_drag_enabled() -> bool {
    true
}

fn default_menu_icon_enabled() -> bool {
    true
}

fn default_coin_format() -> String {
    "{g}g {s}s {c}c".to_string()
}

const CONFIG_FILE_NAME: &str = "session_tracker_config.json";

pub fn load_config(dir: &Path) -> Config {
    let path = dir.join(CONFIG_FILE_NAME);
    let mut config = match fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => Config::default(),
    };
    config.reconcile_row_separator_visible();
    config
}

pub fn save_config(dir: &Path, config: &Config) -> io::Result<()> {
    let path = dir.join(CONFIG_FILE_NAME);
    let contents = serde_json::to_string_pretty(config)
        .expect("Config only contains an Option<String>, always serializes");
    fs::write(path, contents)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn load_config_returns_default_when_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        let config = load_config(dir.path());
        assert_eq!(config, Config::default());
    }

    #[test]
    fn save_then_load_round_trips_every_field() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config {
            api_key: Some("ABC-123".to_string()),
            selected_stats: vec!["kdr".to_string(), "kills".to_string()],
            wvw_selected_stats: vec!["wvw_rank".to_string()],
            pvp_selected_stats: vec!["pvp_rank".to_string()],
            pve_selected_stats: vec!["karma".to_string()],
            background_opacity: 0.75,
            text_scale: 1.5,
            bold_text: true,
            label_color: [0.1, 0.2, 0.3, 1.0],
            value_color: [0.7, 0.8, 0.9, 1.0],
            icon_color: [0.4, 0.5, 0.6, 1.0],
            background_color: [0.9, 0.1, 0.2],
            show_main: true,
            row_fields: vec![RowField::Icon, RowField::Name, RowField::Rate],
            row_separator: "/".to_string(),
            row_separator_visible: vec![true, false],
            fixed_window_height: true,
            window_height: 300.0,
            window_right_margin: 5.0,
            padding: 12.0,
            fix_label_width: true,
            label_width: 100.0,
            window_anchor: WindowAnchor::BottomRight,
            window_offset: [42.0, 7.0],
            window_drag_enabled: true,
            menu_icon_enabled: false,
            click_through_enabled: true,
            coin_format: "{g}g".to_string(),
            hide_zero_stats: true,
        };
        save_config(dir.path(), &config).unwrap();
        let loaded = load_config(dir.path());
        assert_eq!(loaded, config);
    }

    #[test]
    fn load_config_returns_default_for_corrupt_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(CONFIG_FILE_NAME), "not json").unwrap();
        let config = load_config(dir.path());
        assert_eq!(config, Config::default());
    }

    #[test]
    fn load_config_pads_row_separator_visible_for_a_row_fields_list_saved_before_it_existed() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join(CONFIG_FILE_NAME),
            r#"{"api_key": "ABC", "row_fields": ["Icon", "Session", "Lifetime", "Rate"]}"#,
        )
        .unwrap();
        let config = load_config(dir.path());
        assert_eq!(config.row_fields.len(), 4);
        assert_eq!(config.row_separator_visible, vec![false, true, true]);
    }

    #[test]
    fn remove_row_field_does_not_panic_when_row_separator_visible_is_under_sized() {
        let mut config = Config {
            row_fields: vec![RowField::Icon, RowField::Session, RowField::Lifetime, RowField::Rate],
            row_separator_visible: vec![false, true],
            ..Default::default()
        };
        config.remove_row_field(3);
        assert_eq!(config.row_fields, vec![RowField::Icon, RowField::Session, RowField::Lifetime]);
        assert_eq!(config.row_separator_visible, vec![false, true]);
    }

    #[test]
    fn load_config_defaults_selected_stats_when_missing_from_old_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(CONFIG_FILE_NAME), r#"{"api_key": "ABC"}"#).unwrap();
        let config = load_config(dir.path());
        assert_eq!(config.api_key, Some("ABC".to_string()));
        assert_eq!(config.selected_stats, default_selected_stats());
        assert!(config.wvw_selected_stats.is_empty());
        assert!(config.pvp_selected_stats.is_empty());
        assert!(config.pve_selected_stats.is_empty());
        assert_eq!(config.background_opacity, default_background_opacity());
        assert_eq!(config.text_scale, default_text_scale());
        assert!(!config.bold_text);
        assert_eq!(config.label_color, default_label_value_color());
        assert_eq!(config.value_color, default_label_value_color());
        assert_eq!(config.icon_color, default_label_value_color());
        assert_eq!(config.background_color, [0.0, 0.0, 0.0]);
        assert!(!config.show_main);
        assert_eq!(config.row_fields, default_row_fields());
        assert_eq!(config.row_separator, default_row_separator());
        assert_eq!(config.row_separator_visible, default_row_separator_visible());
        assert!(!config.fixed_window_height);
        assert_eq!(config.window_height, default_window_height());
        assert_eq!(config.window_right_margin, 0.0);
        assert_eq!(config.padding, default_padding());
        assert!(!config.fix_label_width);
        assert_eq!(config.label_width, default_label_width());
        assert_eq!(config.window_anchor, WindowAnchor::TopLeft);
        assert_eq!(config.window_offset, default_window_offset());
        assert!(config.window_drag_enabled);
        assert!(config.menu_icon_enabled);
        assert!(!config.click_through_enabled);
        assert_eq!(config.coin_format, default_coin_format());
        assert!(!config.hide_zero_stats);
    }

    #[test]
    fn old_config_with_a_stale_text_color_key_ignores_it_without_error() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join(CONFIG_FILE_NAME),
            r#"{"api_key": "ABC", "text_color": [0.1, 0.2, 0.3, 1.0]}"#,
        )
        .unwrap();
        let config = load_config(dir.path());
        assert_eq!(config.api_key, Some("ABC".to_string()));
        assert_eq!(config.label_color, default_label_value_color());
    }

    #[test]
    fn default_config_has_non_empty_selected_stats() {
        let config = Config::default();
        assert_eq!(
            config.selected_stats,
            vec!["kills", "deaths", "kdr", "wvw_rank"]
        );
    }

    #[test]
    fn default_config_has_default_background_opacity() {
        let config = Config::default();
        assert_eq!(config.background_opacity, 0.35);
    }

    #[test]
    fn push_row_field_appends_a_visible_gap() {
        let mut config = Config::default();
        config.push_row_field(RowField::Rate);
        assert_eq!(config.row_fields, vec![RowField::Icon, RowField::Session, RowField::Lifetime, RowField::Rate]);
        assert_eq!(config.row_separator_visible, vec![false, true, true]);
    }

    #[test]
    fn remove_row_field_from_the_middle_keeps_the_left_gap() {
        let mut config = Config {
            row_fields: vec![RowField::Icon, RowField::Name, RowField::Session],
            row_separator_visible: vec![false, true],
            ..Default::default()
        };
        config.remove_row_field(1);
        assert_eq!(config.row_fields, vec![RowField::Icon, RowField::Session]);
        assert_eq!(config.row_separator_visible, vec![false]);
    }

    #[test]
    fn remove_row_field_from_the_start_drops_its_only_gap() {
        let mut config = Config {
            row_fields: vec![RowField::Icon, RowField::Session, RowField::Lifetime],
            row_separator_visible: vec![false, true],
            ..Default::default()
        };
        config.remove_row_field(0);
        assert_eq!(config.row_fields, vec![RowField::Session, RowField::Lifetime]);
        assert_eq!(config.row_separator_visible, vec![true]);
    }

    #[test]
    fn remove_row_field_from_the_end_drops_its_only_gap() {
        let mut config = Config {
            row_fields: vec![RowField::Icon, RowField::Session, RowField::Lifetime],
            row_separator_visible: vec![false, true],
            ..Default::default()
        };
        config.remove_row_field(2);
        assert_eq!(config.row_fields, vec![RowField::Icon, RowField::Session]);
        assert_eq!(config.row_separator_visible, vec![false]);
    }
}
