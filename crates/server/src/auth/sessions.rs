//! Who is signed in right now.
//!
//! Sessions live in memory rather than in the library, because the library is
//! the desktop app's file too and it has no concept of a user. A restart signs
//! everyone out, which is the cost of not putting a table nobody else needs
//! into a shared schema.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Long, because the web bundle is used from a phone and typing a password on
/// one is nobody's idea of a good time. It slides, so an account in daily use
/// never reaches it.
pub const SESSION_LIFETIME: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// 256 bits from the operating system's generator, hex encoded so it travels
/// in a cookie without escaping.
const TOKEN_BYTES: usize = 32;

#[derive(Default)]
pub struct Sessions {
    active: Mutex<HashMap<String, Instant>>,
}

impl Sessions {
    /// Grants a session and returns the token that names it.
    pub fn start(&self) -> String {
        let token = new_token();
        let mut active = self.lock();

        // Expired entries are dropped here rather than on a timer. One person
        // signing in is the only thing that adds to this map.
        let now = Instant::now();
        active.retain(|_, expiry| *expiry > now);
        active.insert(token.clone(), now + SESSION_LIFETIME);

        token
    }

    /// True when the token names a live session, which is also renewed. An
    /// account in use stays signed in; one left alone expires.
    pub fn renew(&self, token: &str) -> bool {
        let mut active = self.lock();
        let now = Instant::now();

        match active.get(token) {
            Some(expiry) if *expiry > now => {
                active.insert(token.to_owned(), now + SESSION_LIFETIME);
                true
            }
            // An expired entry is removed on the way past, so a token cannot
            // be renewed back to life.
            Some(_) => {
                active.remove(token);
                false
            }
            None => false,
        }
    }

    pub fn end(&self, token: &str) {
        self.lock().remove(token);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Instant>> {
        // A panic while holding this lock would leave the map intact, and
        // refusing every login afterwards helps nobody.
        self.active.lock().unwrap_or_else(|error| error.into_inner())
    }
}

fn new_token() -> String {
    let mut bytes = [0u8; TOKEN_BYTES];
    // The operating system's generator is the only source here. There is no
    // fallback, because a predictable session token is worse than no server.
    getrandom::fill(&mut bytes).expect("the operating system random generator is available");

    let mut token = String::with_capacity(TOKEN_BYTES * 2);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(token, "{byte:02x}");
    }
    token
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_started_session_is_active_and_an_invented_token_is_not() {
        let sessions = Sessions::default();
        let token = sessions.start();

        assert!(sessions.renew(&token));
        assert!(!sessions.renew("0".repeat(64).as_str()));
        assert!(!sessions.renew(""));
    }

    #[test]
    fn signing_out_ends_that_session_and_leaves_the_others() {
        let sessions = Sessions::default();
        let phone = sessions.start();
        let laptop = sessions.start();

        sessions.end(&phone);

        assert!(!sessions.renew(&phone));
        assert!(sessions.renew(&laptop));
    }

    #[test]
    fn two_sessions_never_share_a_token() {
        let sessions = Sessions::default();

        let first = sessions.start();
        let second = sessions.start();

        assert_ne!(first, second);
        assert_eq!(first.len(), TOKEN_BYTES * 2);
        assert!(first.chars().all(|character| character.is_ascii_hexdigit()));
    }

    #[test]
    fn an_expired_session_is_refused_and_forgotten() {
        let sessions = Sessions::default();
        let token = sessions.start();

        // Reaching past `start` is the only way to age a session without
        // waiting thirty days for one.
        sessions
            .lock()
            .insert(token.clone(), Instant::now() - Duration::from_secs(1));

        assert!(!sessions.renew(&token));
        assert!(!sessions.lock().contains_key(&token));
    }
}
