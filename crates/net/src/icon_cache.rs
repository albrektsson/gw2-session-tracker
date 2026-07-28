use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

const ICON_CACHE_DIR_NAME: &str = "icon_cache";

pub fn cache_dir(addon_dir: &Path) -> PathBuf {
    addon_dir.join(ICON_CACHE_DIR_NAME)
}

/// Every GW2 render-server icon URL embeds a content hash in the path
/// (`.../file/<hash>/<id>.png`), so sanitizing the whole URL into a
/// filename is both collision-safe and needs no invalidation - the same
/// URL always maps to the same bytes forever.
pub fn cache_file_name(icon_url: &str) -> String {
    icon_url
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' { c } else { '_' })
        .collect()
}

pub fn cache_path(cache_dir: &Path, icon_url: &str) -> PathBuf {
    cache_dir.join(cache_file_name(icon_url))
}

fn agent() -> ureq::Agent {
    let config = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(10)))
        .timeout_recv_response(Some(Duration::from_secs(10)))
        .timeout_recv_body(Some(Duration::from_secs(10)))
        .build();
    ureq::Agent::new_with_config(config)
}

/// Downloads `icon_url` into `cache_dir` if it isn't already cached. On any
/// failure (a momentary API outage, a write error) this does nothing and
/// returns silently rather than erroring - the icon just keeps rendering
/// via the direct-URL fallback until a later call (the next poll tick)
/// succeeds, mirroring how a failed stat poll is retried rather than
/// treated as fatal.
fn ensure_cached(cache_dir: &Path, icon_url: &str) {
    let path = cache_path(cache_dir, icon_url);
    if path.is_file() {
        return;
    }

    let bytes = match agent().get(icon_url).call() {
        Ok(mut response) => match response.body_mut().read_to_vec() {
            Ok(bytes) => bytes,
            Err(e) => {
                log::debug!("failed to read icon body for {icon_url}: {e}");
                return;
            }
        },
        Err(e) => {
            log::debug!("failed to fetch icon {icon_url}: {e}");
            return;
        }
    };

    // Write to a temp file and rename into place, so a concurrent reader
    // (the render thread checking `cache_path(..).is_file()`) can never
    // observe a partially written file - `fs::rename` is atomic within the
    // same directory.
    let tmp_path = cache_dir.join(format!("{}.tmp", cache_file_name(icon_url)));
    if let Err(e) = fs::write(&tmp_path, &bytes) {
        log::debug!("failed to write icon cache tmp file for {icon_url}: {e}");
        return;
    }
    if let Err(e) = fs::rename(&tmp_path, &path) {
        log::debug!("failed to finalize icon cache file for {icon_url}: {e}");
    }
}

/// Attempts to cache every icon in `icon_urls` not already on disk, checking
/// `cancelled` between downloads so this can't hold up `Poller::stop()` for
/// longer than whichever single request is already in flight (mirrors
/// `gw2_client::check_cancelled`'s between-requests cancellation checks).
pub fn cache_missing_icons(cache_dir: &Path, icon_urls: &[&str], cancelled: &AtomicBool) {
    if fs::create_dir_all(cache_dir).is_err() {
        return;
    }
    for icon_url in icon_urls {
        if cancelled.load(Ordering::SeqCst) {
            return;
        }
        ensure_cached(cache_dir, icon_url);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_file_name_sanitizes_scheme_and_path_separators() {
        let name = cache_file_name("https://render.guildwars2.com/file/ABC123/619316.png");
        assert_eq!(name, "https___render.guildwars2.com_file_ABC123_619316.png");
    }

    #[test]
    fn cache_file_name_is_deterministic() {
        let url = "https://render.guildwars2.com/file/ABC123/619316.png";
        assert_eq!(cache_file_name(url), cache_file_name(url));
    }

    #[test]
    fn cache_file_name_differs_for_different_urls() {
        let a = cache_file_name("https://render.guildwars2.com/file/AAA/1.png");
        let b = cache_file_name("https://render.guildwars2.com/file/BBB/2.png");
        assert_ne!(a, b);
    }

    #[test]
    fn cache_path_joins_cache_dir_and_file_name() {
        let dir = Path::new("/tmp/icons");
        let url = "https://render.guildwars2.com/file/ABC123/619316.png";
        assert_eq!(cache_path(dir, url), dir.join(cache_file_name(url)));
    }

    #[test]
    fn ensure_cached_skips_network_when_already_cached() {
        let dir = tempfile::tempdir().unwrap();
        let url = "https://this-host-does-not-exist.invalid/icon.png";
        let path = cache_path(dir.path(), url);
        fs::write(&path, b"already here").unwrap();

        ensure_cached(dir.path(), url);

        assert_eq!(fs::read(&path).unwrap(), b"already here");
    }

    #[test]
    fn cache_missing_icons_stops_immediately_when_already_cancelled() {
        let dir = tempfile::tempdir().unwrap();
        let cancelled = AtomicBool::new(true);
        let urls = ["https://this-host-does-not-exist.invalid/icon.png"];

        let started = std::time::Instant::now();
        cache_missing_icons(dir.path(), &urls, &cancelled);

        assert!(started.elapsed() < Duration::from_millis(200));
        assert!(!cache_path(dir.path(), urls[0]).exists());
    }
}
