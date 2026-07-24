use std::sync::{Mutex, MutexGuard};

/// Locks `mutex`, recovering the guard instead of panicking if a previous
/// holder panicked while locked. The addon runs as a single process with
/// no transactional invariants split across lock/unlock pairs, so a stale
/// value from an aborted update is safe to keep using rather than letting
/// every subsequent lock on the same mutex panic too.
pub fn lock_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}
