use crate::session::SessionSnapshot;
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

const SESSION_STATE_FILE_NAME: &str = "session_tracker_session.json";

/// A `SessionSnapshot` plus the wall-clock instant it was written -
/// `automatic_reset::decide` compares this against "now" at the next load
/// to decide whether the Session it describes should carry forward.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedSession {
    pub saved_at_unix: u64,
    pub snapshot: SessionSnapshot,
}

pub fn now_unix() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

/// `None` for a missing or corrupt file - both mean "nothing to restore",
/// same as a session that was never persisted at all.
pub fn load_session_state(dir: &Path) -> Option<PersistedSession> {
    let contents = fs::read_to_string(dir.join(SESSION_STATE_FILE_NAME)).ok()?;
    serde_json::from_str(&contents).ok()
}

/// Writes `snapshot` (stamped with the current wall-clock time), or clears
/// any previously-persisted state when `snapshot` is `None` - there's
/// nothing meaningful to carry forward (the Session never got any data
/// this run), and leaving a stale file around risks restoring it later.
pub fn save_session_state(dir: &Path, snapshot: Option<&SessionSnapshot>) -> io::Result<()> {
    let path = dir.join(SESSION_STATE_FILE_NAME);
    match snapshot {
        Some(snapshot) => {
            let persisted = PersistedSession { saved_at_unix: now_unix(), snapshot: snapshot.clone() };
            let contents =
                serde_json::to_string_pretty(&persisted).expect("PersistedSession only contains plain data, always serializes");
            fs::write(path, contents)
        }
        None => match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::SessionTracker;
    use std::collections::HashMap;

    fn values(pairs: &[(&'static str, f64)]) -> HashMap<&'static str, f64> {
        pairs.iter().copied().collect()
    }

    #[test]
    fn load_returns_none_when_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load_session_state(dir.path()).is_none());
    }

    #[test]
    fn load_returns_none_for_corrupt_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(SESSION_STATE_FILE_NAME), "not json").unwrap();
        assert!(load_session_state(dir.path()).is_none());
    }

    #[test]
    fn save_then_load_round_trips_the_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let mut tracker = SessionTracker::new();
        tracker.update(values(&[("kills", 7.0)]));
        let snapshot = tracker.snapshot().unwrap();

        save_session_state(dir.path(), Some(&snapshot)).unwrap();
        let loaded = load_session_state(dir.path()).unwrap();

        assert_eq!(loaded.snapshot.lifetime.get("kills"), Some(&7.0));
        assert!(loaded.saved_at_unix > 0);
    }

    #[test]
    fn saving_none_clears_a_previously_persisted_file() {
        let dir = tempfile::tempdir().unwrap();
        let mut tracker = SessionTracker::new();
        tracker.update(values(&[("kills", 1.0)]));
        let snapshot = tracker.snapshot().unwrap();
        save_session_state(dir.path(), Some(&snapshot)).unwrap();
        assert!(load_session_state(dir.path()).is_some());

        save_session_state(dir.path(), None).unwrap();
        assert!(load_session_state(dir.path()).is_none());
    }

    #[test]
    fn saving_none_when_no_file_exists_is_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        save_session_state(dir.path(), None).unwrap();
    }
}
