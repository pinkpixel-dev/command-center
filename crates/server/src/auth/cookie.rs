//! The session cookie.
//!
//! `HttpOnly` because no script has any reason to read it, and `SameSite=Lax`
//! because every command is a POST and Lax refuses cross-site ones. That is
//! what stands in for a CSRF token here.

use std::time::Duration;

pub const COOKIE_NAME: &str = "command_center_session";

/// Builds the header that grants the session.
///
/// `Secure` is added only when the request arrived over HTTPS. The server
/// itself speaks plain HTTP and expects TLS to be terminated in front of it,
/// so setting it unconditionally would make signing in impossible on the LAN
/// deployment this is mostly built for.
pub fn grant(token: &str, lifetime: Duration, secure: bool) -> String {
    let mut header = format!(
        "{COOKIE_NAME}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        lifetime.as_secs()
    );
    if secure {
        header.push_str("; Secure");
    }
    header
}

/// Builds the header that takes it away. The attributes have to match the
/// ones it was set with or the browser keeps the old cookie alongside this.
pub fn revoke(secure: bool) -> String {
    let mut header = format!("{COOKIE_NAME}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0");
    if secure {
        header.push_str("; Secure");
    }
    header
}

/// Pulls the session token out of a `Cookie` header. The value is a hex token
/// this server issued, so there is no quoting or encoding to undo.
pub fn token_from(header: Option<&str>) -> Option<&str> {
    header?
        .split(';')
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, _)| *name == COOKIE_NAME)
        .map(|(_, value)| value)
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_granted_cookie_cannot_be_read_by_a_script_or_sent_cross_site() {
        let header = grant("abc123", Duration::from_secs(60), false);

        assert!(header.starts_with("command_center_session=abc123;"));
        assert!(header.contains("HttpOnly"));
        assert!(header.contains("SameSite=Lax"));
        assert!(header.contains("Max-Age=60"));
        assert!(!header.contains("Secure"));
    }

    /// Over HTTPS the cookie must not be allowed to travel in the clear.
    #[test]
    fn https_adds_secure_to_both_headers() {
        assert!(grant("abc123", Duration::from_secs(60), true).contains("; Secure"));
        assert!(revoke(true).contains("; Secure"));
    }

    #[test]
    fn revoking_expires_the_cookie_rather_than_replacing_its_value() {
        let header = revoke(false);

        assert!(header.contains("Max-Age=0"));
        assert!(header.starts_with("command_center_session=;"));
    }

    #[test]
    fn the_token_is_found_among_other_cookies() {
        assert_eq!(
            token_from(Some("theme=dark; command_center_session=abc123; other=1")),
            Some("abc123")
        );
        assert_eq!(token_from(Some("command_center_session=abc123")), Some("abc123"));
    }

    #[test]
    fn anything_that_is_not_our_cookie_reads_as_no_session() {
        assert_eq!(token_from(None), None);
        assert_eq!(token_from(Some("")), None);
        assert_eq!(token_from(Some("theme=dark")), None);
        // An emptied cookie a browser is still sending is not a session.
        assert_eq!(token_from(Some("command_center_session=")), None);
        // A cookie whose name merely ends with ours must not match.
        assert_eq!(token_from(Some("not_command_center_session=abc")), None);
    }
}
