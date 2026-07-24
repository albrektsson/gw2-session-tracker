use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use session_tracker_core::{api::ApiSnapshot, session::SessionTracker, stats};

#[derive(Debug, Clone)]
pub enum PollStatus {
    AwaitingApiKey,
    Ok,
    Error(String),
}

pub struct AppState {
    pub api_key: Option<String>,
    pub session: SessionTracker,
    pub status: PollStatus,
    pub last_updated: Option<Instant>,
}

impl AppState {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            api_key,
            session: SessionTracker::new(),
            status: PollStatus::AwaitingApiKey,
            last_updated: None,
        }
    }
}

pub struct Poller {
    shutdown: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Poller {
    pub fn spawn<F>(shared: Arc<Mutex<AppState>>, interval: Duration, fetch: F) -> Self
    where
        F: Fn(&str) -> Result<ApiSnapshot, String> + Send + 'static,
    {
        let shutdown = Arc::new(AtomicBool::new(false));
        let thread_shutdown = shutdown.clone();
        let handle = thread::spawn(move || {
            run_poller(shared, thread_shutdown, interval, fetch);
        });
        Self {
            shutdown,
            handle: Some(handle),
        }
    }

    pub fn stop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for Poller {
    fn drop(&mut self) {
        self.stop();
    }
}

fn run_poller<F>(
    shared: Arc<Mutex<AppState>>,
    shutdown: Arc<AtomicBool>,
    interval: Duration,
    fetch: F,
) where
    F: Fn(&str) -> Result<ApiSnapshot, String>,
{
    let poll_slice = Duration::from_millis(200);

    loop {
        if shutdown.load(Ordering::SeqCst) {
            return;
        }

        let api_key = shared.lock().unwrap().api_key.clone();
        if let Some(api_key) = api_key {
            log::info!("polling GW2 API for WvW stats");
            let result = fetch(&api_key);
            let mut state = shared.lock().unwrap();
            match result {
                Ok(snapshot) => {
                    log::info!(
                        "GW2 API poll succeeded (wvw_rank={}, {} achievements tracked, {} total deaths)",
                        snapshot.wvw_rank,
                        snapshot.achievements.len(),
                        snapshot.total_deaths
                    );
                    let values = stats::compute_lifetime_values(&snapshot);
                    state.session.update(values);
                    state.status = PollStatus::Ok;
                    state.last_updated = Some(Instant::now());
                }
                Err(err) => {
                    log::warn!("GW2 API poll failed: {err}");
                    state.status = PollStatus::Error(err);
                }
            }
        } else {
            log::info!("no API key configured yet, skipping poll");
        }

        let mut waited = Duration::ZERO;
        while waited < interval {
            if shutdown.load(Ordering::SeqCst) {
                return;
            }
            thread::sleep(poll_slice);
            waited += poll_slice;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::atomic::AtomicUsize,
        time::{Duration, Instant},
    };
    use session_tracker_core::api::ApiSnapshot;

    #[test]
    fn poller_updates_state_and_stops_on_shutdown() {
        let shared = Arc::new(Mutex::new(AppState::new(Some("test-key".to_string()))));
        let call_count = Arc::new(AtomicUsize::new(0));
        let fetch_call_count = call_count.clone();

        let fetch = move |_key: &str| {
            fetch_call_count.fetch_add(1, Ordering::SeqCst);
            Ok(ApiSnapshot {
                wvw_rank: 10,
                achievements: Default::default(),
                total_deaths: 2,
            })
        };

        let mut poller = Poller::spawn(shared.clone(), Duration::from_millis(50), fetch);

        let deadline = Instant::now() + Duration::from_secs(2);
        while call_count.load(Ordering::SeqCst) == 0 && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }

        poller.stop();

        assert!(call_count.load(Ordering::SeqCst) >= 1);
        let state = shared.lock().unwrap();
        assert!(matches!(state.status, PollStatus::Ok));
        assert_eq!(state.session.lifetime_value("wvw_rank"), 10.0);
    }

    #[test]
    fn poller_without_api_key_never_calls_fetch() {
        let shared = Arc::new(Mutex::new(AppState::new(None)));
        let call_count = Arc::new(AtomicUsize::new(0));
        let fetch_call_count = call_count.clone();

        let fetch = move |_key: &str| {
            fetch_call_count.fetch_add(1, Ordering::SeqCst);
            Ok(ApiSnapshot {
                wvw_rank: 0,
                achievements: Default::default(),
                total_deaths: 0,
            })
        };

        let mut poller = Poller::spawn(shared.clone(), Duration::from_millis(20), fetch);
        thread::sleep(Duration::from_millis(100));
        poller.stop();

        assert_eq!(call_count.load(Ordering::SeqCst), 0);
        let state = shared.lock().unwrap();
        assert!(matches!(state.status, PollStatus::AwaitingApiKey));
    }

    #[test]
    fn poller_records_fetch_errors_without_crashing() {
        let shared = Arc::new(Mutex::new(AppState::new(Some("bad-key".to_string()))));
        let fetch = |_key: &str| Err("401 Unauthorized".to_string());

        let mut poller = Poller::spawn(shared.clone(), Duration::from_millis(20), fetch);
        thread::sleep(Duration::from_millis(100));
        poller.stop();

        let state = shared.lock().unwrap();
        assert!(matches!(&state.status, PollStatus::Error(msg) if msg.contains("401")));
    }
}
