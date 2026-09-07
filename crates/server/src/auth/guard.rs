//! What stops the login route being guessed at.
//!
//! The count is kept for the server rather than per client address. There is
//! one account, so there is nothing to tell apart, and a client address behind
//! a reverse proxy is whatever the proxy says it is. A shared counter cannot
//! be walked around by changing address.
//!
//! The delay is capped rather than turning into a lockout, so somebody
//! hammering the login route slows an attacker down without locking the owner
//! out of their own library.

use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Typing a password wrong happens. The backoff starts after that.
const FREE_ATTEMPTS: u32 = 3;

/// Doubling from one second, stopping here. Half a minute a guess is slow
/// enough to make guessing pointless and short enough to be a wait, not a
/// lockout.
const MAX_DELAY: Duration = Duration::from_secs(30);

#[derive(Default)]
pub struct LoginGuard {
    state: Mutex<Attempts>,
}

#[derive(Default)]
struct Attempts {
    failures: u32,
    blocked_until: Option<Instant>,
}

impl LoginGuard {
    /// How much longer this attempt has to wait, if it does.
    pub fn wait(&self) -> Option<Duration> {
        let state = self.lock();
        let until = state.blocked_until?;
        until.checked_duration_since(Instant::now())
    }

    pub fn failed(&self) {
        let mut state = self.lock();
        state.failures = state.failures.saturating_add(1);
        state.blocked_until = delay_after(state.failures).map(|delay| Instant::now() + delay);
    }

    /// One correct password clears the record. Somebody who knows it should
    /// not be waiting behind a stranger's guesses.
    pub fn succeeded(&self) {
        let mut state = self.lock();
        state.failures = 0;
        state.blocked_until = None;
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Attempts> {
        self.state.lock().unwrap_or_else(|error| error.into_inner())
    }
}

fn delay_after(failures: u32) -> Option<Duration> {
    let past_free = failures.checked_sub(FREE_ATTEMPTS)?;
    if past_free == 0 {
        return None;
    }

    let seconds = 1u64.checked_shl(past_free - 1).unwrap_or(u64::MAX);
    Some(Duration::from_secs(seconds).min(MAX_DELAY))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_few_wrong_tries_are_free() {
        let guard = LoginGuard::default();

        for _ in 0..FREE_ATTEMPTS {
            guard.failed();
            assert!(guard.wait().is_none());
        }
    }

    #[test]
    fn the_delay_doubles_and_then_stops_growing() {
        assert_eq!(delay_after(FREE_ATTEMPTS), None);
        assert_eq!(delay_after(FREE_ATTEMPTS + 1), Some(Duration::from_secs(1)));
        assert_eq!(delay_after(FREE_ATTEMPTS + 2), Some(Duration::from_secs(2)));
        assert_eq!(delay_after(FREE_ATTEMPTS + 3), Some(Duration::from_secs(4)));
        assert_eq!(delay_after(FREE_ATTEMPTS + 20), Some(MAX_DELAY));
        // Far enough out that the shift itself would overflow.
        assert_eq!(delay_after(u32::MAX), Some(MAX_DELAY));
    }

    #[test]
    fn a_wrong_password_after_the_free_tries_makes_the_next_one_wait() {
        let guard = LoginGuard::default();

        for _ in 0..=FREE_ATTEMPTS {
            guard.failed();
        }

        assert!(guard.wait().is_some());
    }

    #[test]
    fn getting_it_right_clears_the_record() {
        let guard = LoginGuard::default();

        for _ in 0..=FREE_ATTEMPTS + 4 {
            guard.failed();
        }
        assert!(guard.wait().is_some());

        guard.succeeded();

        assert!(guard.wait().is_none());
        // And the next mistake starts from the beginning rather than the cap.
        guard.failed();
        assert!(guard.wait().is_none());
    }
}
