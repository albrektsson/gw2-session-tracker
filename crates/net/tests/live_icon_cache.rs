use session_tracker_net::icon_cache::{cache_missing_icons, cache_path};

/// Run with: cargo test -p session_tracker_net --test live_icon_cache -- --ignored
/// No API key needed - render.guildwars2.com icon URLs are public assets.
#[test]
#[ignore = "hits the live GW2 render server"]
fn cache_missing_icons_downloads_a_real_icon() {
    let dir = tempfile::tempdir().unwrap();
    let url = "https://render.guildwars2.com/file/98457F504BA2FAC8457F532C4B30EDC23929ACF9/619316.png";
    let cancelled = std::sync::atomic::AtomicBool::new(false);

    cache_missing_icons(dir.path(), &[url], &cancelled);

    let path = cache_path(dir.path(), url);
    let bytes = std::fs::read(&path).expect("icon should have been cached to disk");
    assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"), "cached file should be a valid PNG");
}
