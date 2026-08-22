//! Codex version parsing and the supported-version floor.
//!
//! Command Center does not ship Codex, so the user's installed version can
//! move without a Command Center release. The floor is the version this app
//! was actually tested against, not a guess about when each protocol feature
//! landed upstream.

use std::fmt;

/// The oldest Codex build Command Center has been verified against.
///
/// Raising this is a user-visible change: anyone below it sees the
/// "Codex is too old" state until they update. Only raise it after testing
/// the new floor, and record the reason in the changelog.
pub const MINIMUM_CODEX_VERSION: CodexVersion = CodexVersion {
    major: 0,
    minor: 147,
    patch: 0,
};

/// `codex --version` prints a short line. Anything longer is not a version.
const MAX_VERSION_OUTPUT: usize = 256;

/// A three-part Codex version. Pre-release and build suffixes are parsed off
/// and ignored, so `0.148.0-alpha.1` compares equal to `0.148.0`.
///
/// Field order matters: the derived ordering compares major, then minor, then
/// patch, which is exactly the comparison we want.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CodexVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl CodexVersion {
    /// Reads the version out of whatever `codex --version` printed.
    ///
    /// Accepts `codex-cli 0.147.0`, `codex 0.147.0`, `0.147.0`, and a leading
    /// `v`. Returns `None` for anything that does not contain three numbers,
    /// rather than guessing at a partial version.
    pub fn parse(raw: &str) -> Option<Self> {
        if raw.len() > MAX_VERSION_OUTPUT {
            return None;
        }

        // The version is the last whitespace-separated token that starts with
        // a digit once an optional `v` is removed. Taking the last token alone
        // would break on future suffixes like `0.147.0 (nightly)`.
        raw.split_whitespace()
            .filter_map(Self::parse_token)
            .next_back()
    }

    fn parse_token(token: &str) -> Option<Self> {
        let token = token.strip_prefix('v').unwrap_or(token);
        // Drop a pre-release or build suffix before splitting on dots, so the
        // `0` in `0.148.0-alpha` does not turn into an unparsable patch.
        let core = token.split(['-', '+']).next()?;

        let mut parts = core.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next()?.parse().ok()?;
        // A fourth numeric part means this is not a Codex version string.
        if parts.next().is_some() {
            return None;
        }

        Some(Self {
            major,
            minor,
            patch,
        })
    }

    pub fn meets_minimum(&self) -> bool {
        *self >= MINIMUM_CODEX_VERSION
    }
}

impl fmt::Display for CodexVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_real_cli_output() {
        // Exactly what `codex --version` printed during the protocol spike.
        let parsed = CodexVersion::parse("codex-cli 0.147.0\n").unwrap();
        assert_eq!(parsed, CodexVersion { major: 0, minor: 147, patch: 0 });
    }

    #[test]
    fn parses_bare_and_prefixed_versions() {
        let bare = CodexVersion::parse("0.148.2").unwrap();
        assert_eq!(bare.to_string(), "0.148.2");
        assert_eq!(CodexVersion::parse("v1.0.0").unwrap().major, 1);
        assert_eq!(CodexVersion::parse("codex 2.10.3").unwrap().minor, 10);
    }

    #[test]
    fn prerelease_and_build_suffixes_are_ignored() {
        let alpha = CodexVersion::parse("codex-cli 0.148.0-alpha.1").unwrap();
        let release = CodexVersion::parse("0.148.0").unwrap();
        assert_eq!(alpha, release);
        assert_eq!(CodexVersion::parse("0.148.0+build7").unwrap(), release);
    }

    #[test]
    fn unparsable_output_is_rejected_rather_than_guessed() {
        assert_eq!(CodexVersion::parse(""), None);
        assert_eq!(CodexVersion::parse("not a version"), None);
        assert_eq!(CodexVersion::parse("0.147"), None);
        assert_eq!(CodexVersion::parse("1.2.3.4"), None);
        // A shim that prints a Node version instead of Codex's own.
        assert_eq!(CodexVersion::parse("command not found: codex"), None);
    }

    #[test]
    fn absurdly_long_output_is_rejected_without_scanning_it() {
        let flood = "0.147.0 ".repeat(1_000);
        assert_eq!(CodexVersion::parse(&flood), None);
    }

    #[test]
    fn ordering_compares_each_part_in_turn() {
        let older = CodexVersion::parse("0.99.9").unwrap();
        let newer = CodexVersion::parse("0.147.0").unwrap();
        let major = CodexVersion::parse("1.0.0").unwrap();

        assert!(older < newer);
        assert!(newer < major);
    }

    #[test]
    fn the_floor_is_the_version_that_was_actually_tested() {
        assert_eq!(MINIMUM_CODEX_VERSION.to_string(), "0.147.0");
        assert!(CodexVersion::parse("codex-cli 0.147.0").unwrap().meets_minimum());
        assert!(CodexVersion::parse("0.148.0").unwrap().meets_minimum());
        assert!(!CodexVersion::parse("0.146.9").unwrap().meets_minimum());
    }
}
