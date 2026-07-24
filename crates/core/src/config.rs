use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub api_key: Option<String>,
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
    fn save_then_load_round_trips_api_key() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config { api_key: Some("ABC-123".to_string()) };
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
}
