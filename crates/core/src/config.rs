use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub api_key: Option<String>,
    #[serde(default = "default_selected_stats")]
    pub selected_stats: Vec<String>,
    #[serde(default = "default_background_opacity")]
    pub background_opacity: f32,
    #[serde(default = "default_text_scale")]
    pub text_scale: f32,
    #[serde(default)]
    pub bold_text: bool,
    #[serde(default = "default_text_color")]
    pub text_color: [f32; 4],
    #[serde(default = "default_icon_color")]
    pub icon_color: [f32; 4],
    #[serde(default)]
    pub show_settings: bool,
    #[serde(default)]
    pub show_main: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key: None,
            selected_stats: default_selected_stats(),
            background_opacity: default_background_opacity(),
            text_scale: default_text_scale(),
            bold_text: false,
            text_color: default_text_color(),
            icon_color: default_icon_color(),
            show_settings: false,
            show_main: false,
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

fn default_text_color() -> [f32; 4] {
    [1.0, 0.85, 0.3, 1.0]
}

fn default_icon_color() -> [f32; 4] {
    default_text_color()
}

const CONFIG_FILE_NAME: &str = "session_tracker_config.json";

pub fn load_config(dir: &Path) -> Config {
    let path = dir.join(CONFIG_FILE_NAME);
    match fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => Config::default(),
    }
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
    fn save_then_load_round_trips_api_key_and_selected_stats() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config {
            api_key: Some("ABC-123".to_string()),
            selected_stats: vec!["kdr".to_string(), "kills".to_string()],
            background_opacity: 0.75,
            text_scale: 1.5,
            bold_text: true,
            text_color: [0.1, 0.2, 0.3, 1.0],
            icon_color: [0.4, 0.5, 0.6, 1.0],
            show_settings: true,
            show_main: true,
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
    fn load_config_defaults_selected_stats_when_missing_from_old_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(CONFIG_FILE_NAME), r#"{"api_key": "ABC"}"#).unwrap();
        let config = load_config(dir.path());
        assert_eq!(config.api_key, Some("ABC".to_string()));
        assert_eq!(config.selected_stats, default_selected_stats());
        assert_eq!(config.background_opacity, default_background_opacity());
        assert_eq!(config.text_scale, default_text_scale());
        assert!(!config.bold_text);
        assert_eq!(config.text_color, default_text_color());
        assert_eq!(config.icon_color, default_icon_color());
        assert!(!config.show_settings);
        assert!(!config.show_main);
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
}
