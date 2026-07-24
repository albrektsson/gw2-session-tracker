use session_tracker_net::gw2_client::fetch_snapshot;

/// Run with: GW2_API_KEY=... cargo test -p session_tracker_net --test live_api -- --ignored
/// or set GW2_API_KEY in a gitignored `.env` file instead of the env var.
#[test]
#[ignore = "hits the live GW2 API; requires a real GW2_API_KEY"]
fn fetch_snapshot_parses_the_live_api_response() {
    dotenvy::dotenv().ok();

    let Ok(api_key) = std::env::var("GW2_API_KEY") else {
        eprintln!("skipping: set GW2_API_KEY (directly or via a .env file) to run this test");
        return;
    };

    fetch_snapshot(&api_key).expect("live GW2 API response failed to parse");
}
