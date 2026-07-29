use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use session_tracker_core::{api::ApiSnapshot, map_context::MapGroup, session::SessionTracker, stats, sync::lock_recover};

#[derive(Debug, Clone)]
pub enum PollStatus {
    AwaitingApiKey,
    Pending,
    Ok,
    Error(String),
}

/// Identifies one of the four fixed stat lists a stat can be selected into.
/// `Global` is always shown; `Wvw`/`Pvp`/`Pve` only render while
/// `AppState::current_map_group` matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatListKind {
    Global,
    Wvw,
    Pvp,
    Pve,
}

impl StatListKind {
    pub const ALL: [StatListKind; 4] = [Self::Global, Self::Wvw, Self::Pvp, Self::Pve];

    pub fn label(self) -> &'static str {
        match self {
            Self::Global => "Global",
            Self::Wvw => "WvW",
            Self::Pvp => "PvP",
            Self::Pve => "PvE",
        }
    }
}

pub struct AppState {
    pub api_key: Option<String>,
    pub selected_stats: Vec<String>,
    pub wvw_selected_stats: Vec<String>,
    pub pvp_selected_stats: Vec<String>,
    pub pve_selected_stats: Vec<String>,
    pub background_opacity: f32,
    pub text_scale: f32,
    pub bold_text: bool,
    pub text_color: [f32; 4],
    pub icon_color: [f32; 4],
    pub session: SessionTracker,
    pub status: PollStatus,
    pub last_updated: Option<Instant>,
    /// Live, derived every frame from MumbleLink (see `render_frame`) - not
    /// part of persisted config.
    pub current_map_group: Option<MapGroup>,
}

impl AppState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        api_key: Option<String>,
        selected_stats: Vec<String>,
        wvw_selected_stats: Vec<String>,
        pvp_selected_stats: Vec<String>,
        pve_selected_stats: Vec<String>,
        background_opacity: f32,
        text_scale: f32,
        bold_text: bool,
        text_color: [f32; 4],
        icon_color: [f32; 4],
    ) -> Self {
        let status = if api_key.is_some() {
            PollStatus::Pending
        } else {
            PollStatus::AwaitingApiKey
        };
        Self {
            api_key,
            selected_stats,
            wvw_selected_stats,
            pvp_selected_stats,
            pve_selected_stats,
            background_opacity,
            text_scale,
            bold_text,
            text_color,
            icon_color,
            session: SessionTracker::new(),
            status,
            last_updated: None,
            current_map_group: None,
        }
    }

    pub fn stat_list(&self, kind: StatListKind) -> &Vec<String> {
        match kind {
            StatListKind::Global => &self.selected_stats,
            StatListKind::Wvw => &self.wvw_selected_stats,
            StatListKind::Pvp => &self.pvp_selected_stats,
            StatListKind::Pve => &self.pve_selected_stats,
        }
    }

    pub fn stat_list_mut(&mut self, kind: StatListKind) -> &mut Vec<String> {
        match kind {
            StatListKind::Global => &mut self.selected_stats,
            StatListKind::Wvw => &mut self.wvw_selected_stats,
            StatListKind::Pvp => &mut self.pvp_selected_stats,
            StatListKind::Pve => &mut self.pve_selected_stats,
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
        F: Fn(&str, &AtomicBool) -> Result<ApiSnapshot, String> + Send + 'static,
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
    F: Fn(&str, &AtomicBool) -> Result<ApiSnapshot, String>,
{
    let poll_slice = Duration::from_millis(200);

    loop {
        if shutdown.load(Ordering::SeqCst) {
            return;
        }

        let api_key = lock_recover(&shared).api_key.clone();
        if let Some(api_key) = api_key {
            log::info!("polling GW2 API for WvW stats");
            let result = fetch(&api_key, &shutdown);
            if shutdown.load(Ordering::SeqCst) {
                // Shutting down mid-poll: `result` may just be the
                // cancellation error `fetch` bailed out with, or a real
                // one raced against it - either way there's no point
                // logging/storing it, the addon is unloading.
                return;
            }
            let mut state = lock_recover(&shared);
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
    fn stat_list_kind_routes_to_the_right_field() {
        let mut state = AppState::new(
            None,
            vec!["kills".to_string()],
            vec!["wvw_rank".to_string()],
            vec!["pvp_rank".to_string()],
            vec!["karma".to_string()],
            0.35,
            1.0,
            false,
            [1.0, 0.85, 0.3, 1.0],
            [1.0, 0.85, 0.3, 1.0],
        );

        assert_eq!(state.stat_list(StatListKind::Global), &vec!["kills".to_string()]);
        assert_eq!(state.stat_list(StatListKind::Wvw), &vec!["wvw_rank".to_string()]);
        assert_eq!(state.stat_list(StatListKind::Pvp), &vec!["pvp_rank".to_string()]);
        assert_eq!(state.stat_list(StatListKind::Pve), &vec!["karma".to_string()]);

        state.stat_list_mut(StatListKind::Pve).push("gold".to_string());
        assert_eq!(state.pve_selected_stats, vec!["karma".to_string(), "gold".to_string()]);
    }

    #[test]
    fn poller_updates_state_and_stops_on_shutdown() {
        let shared = Arc::new(Mutex::new(AppState::new(Some("test-key".to_string()), vec![], vec![], vec![], vec![], 0.35, 1.0, false, [1.0, 0.85, 0.3, 1.0], [1.0, 0.85, 0.3, 1.0])));
        let call_count = Arc::new(AtomicUsize::new(0));
        let fetch_call_count = call_count.clone();

        let fetch = move |_key: &str, _shutdown: &AtomicBool| {
            fetch_call_count.fetch_add(1, Ordering::SeqCst);
            Ok(ApiSnapshot {
                wvw_rank: 10,
                achievements: Default::default(),
                total_deaths: 2,
                currencies: Default::default(),
                pvp_rank: 0,
                pvp_wins: 0,
                pvp_losses: 0,
                pvp_ranking_points: 0,
                pvp_ranked_wins: 0,
                pvp_ranked_losses: 0,
                pvp_unranked_wins: 0,
                pvp_unranked_losses: 0,
                items: Default::default(),
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
    fn new_state_with_api_key_starts_pending_not_awaiting_api_key() {
        let state = AppState::new(Some("test-key".to_string()), vec![], vec![], vec![], vec![], 0.35, 1.0, false, [1.0, 0.85, 0.3, 1.0], [1.0, 0.85, 0.3, 1.0]);
        assert!(matches!(state.status, PollStatus::Pending));
    }

    #[test]
    fn new_state_without_api_key_starts_awaiting_api_key() {
        let state = AppState::new(None, vec![], vec![], vec![], vec![], 0.35, 1.0, false, [1.0, 0.85, 0.3, 1.0], [1.0, 0.85, 0.3, 1.0]);
        assert!(matches!(state.status, PollStatus::AwaitingApiKey));
    }

    #[test]
    fn poller_without_api_key_never_calls_fetch() {
        let shared = Arc::new(Mutex::new(AppState::new(None, vec![], vec![], vec![], vec![], 0.35, 1.0, false, [1.0, 0.85, 0.3, 1.0], [1.0, 0.85, 0.3, 1.0])));
        let call_count = Arc::new(AtomicUsize::new(0));
        let fetch_call_count = call_count.clone();

        let fetch = move |_key: &str, _shutdown: &AtomicBool| {
            fetch_call_count.fetch_add(1, Ordering::SeqCst);
            Ok(ApiSnapshot {
                wvw_rank: 0,
                achievements: Default::default(),
                total_deaths: 0,
                currencies: Default::default(),
                pvp_rank: 0,
                pvp_wins: 0,
                pvp_losses: 0,
                pvp_ranking_points: 0,
                pvp_ranked_wins: 0,
                pvp_ranked_losses: 0,
                pvp_unranked_wins: 0,
                pvp_unranked_losses: 0,
                items: Default::default(),
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
        let shared = Arc::new(Mutex::new(AppState::new(Some("bad-key".to_string()), vec![], vec![], vec![], vec![], 0.35, 1.0, false, [1.0, 0.85, 0.3, 1.0], [1.0, 0.85, 0.3, 1.0])));
        let fetch = |_key: &str, _shutdown: &AtomicBool| Err("401 Unauthorized".to_string());

        let mut poller = Poller::spawn(shared.clone(), Duration::from_millis(20), fetch);
        thread::sleep(Duration::from_millis(100));
        poller.stop();

        let state = shared.lock().unwrap();
        assert!(matches!(&state.status, PollStatus::Error(msg) if msg.contains("401")));
    }

    #[test]
    fn stop_returns_promptly_when_fetch_cooperates_with_shutdown() {
        // Simulates a `fetch_snapshot`-shaped call that only checks for
        // cancellation between several sequential steps, rather than
        // instantly - `stop()` must not need to wait for the whole
        // simulated call to finish, just for it to notice `shutdown`.
        let shared = Arc::new(Mutex::new(AppState::new(Some("test-key".to_string()), vec![], vec![], vec![], vec![], 0.35, 1.0, false, [1.0, 0.85, 0.3, 1.0], [1.0, 0.85, 0.3, 1.0])));
        let started = Arc::new(AtomicUsize::new(0));
        let fetch_started = started.clone();

        let fetch = move |_key: &str, shutdown: &AtomicBool| {
            fetch_started.fetch_add(1, Ordering::SeqCst);
            for _ in 0..8 {
                if shutdown.load(Ordering::SeqCst) {
                    return Err("cancelled".to_string());
                }
                thread::sleep(Duration::from_millis(20));
            }
            Err("never cancelled".to_string())
        };

        let mut poller = Poller::spawn(shared.clone(), Duration::from_secs(60), fetch);

        let deadline = Instant::now() + Duration::from_secs(2);
        while started.load(Ordering::SeqCst) == 0 && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(started.load(Ordering::SeqCst), 1);

        let stop_started = Instant::now();
        poller.stop();
        assert!(stop_started.elapsed() < Duration::from_millis(500));
    }
}
