//! Finding the Codex executable the user already installed.
//!
//! Command Center does not ship Codex and does not install it. Discovery
//! resolves a real executable path, checks its version against the supported
//! floor, and reports a specific reason when it cannot.
//!
//! The pure parts of discovery (which paths to consider, how to judge a
//! candidate) are separated from the one impure part (running `--version`) so
//! the ordering rules can be tested without a Codex install present.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;

use super::version::{CodexVersion, MINIMUM_CODEX_VERSION};

/// `codex --version` starts a process. It is fast, but a wedged or wrong
/// executable must not hold Settings open.
const VERSION_PROBE_TIMEOUT: Duration = Duration::from_secs(10);

/// Where a candidate path came from. A path the user typed gets a different
/// failure message than one we guessed at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateSource {
    /// A path saved in AI Settings by the user.
    UserProvided,
    /// Found by walking the `PATH` environment variable.
    SearchPath,
    /// A well-known install location for the official package.
    KnownLocation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub path: PathBuf,
    pub source: CandidateSource,
}

/// The result of discovery. Every variant is a Settings state, not an error to
/// swallow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodexDiscovery {
    /// Usable: found, runnable, and at or above the floor.
    Found {
        path: PathBuf,
        version: CodexVersion,
        source: CandidateSource,
    },
    /// Found and runnable, but older than Command Center supports.
    TooOld { path: PathBuf, version: CodexVersion },
    /// The path exists but could not be run, or did not report a version.
    Unusable { path: PathBuf, reason: UnusableReason },
    /// Nothing to try.
    NotFound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UnusableReason {
    /// The saved path does not exist.
    Missing,
    /// The file exists but is not marked executable.
    NotExecutable,
    /// A Windows `.cmd` or shell shim, which cannot be started directly.
    ShimNotSupported,
    /// The process started but did not print a version we could read.
    NoVersion,
    /// The process did not finish within the probe timeout.
    Timeout,
}

impl CodexDiscovery {
    pub fn usable_path(&self) -> Option<&Path> {
        match self {
            Self::Found { path, .. } => Some(path),
            _ => None,
        }
    }
}

/// The executable name to look for on this platform.
#[cfg(windows)]
const EXECUTABLE_NAMES: &[&str] = &["codex.exe"];
#[cfg(not(windows))]
const EXECUTABLE_NAMES: &[&str] = &["codex"];

/// Shim names that will be found on `PATH` but cannot be spawned directly.
/// Reporting them beats silently skipping them, because the user can see
/// Codex on their `PATH` and would otherwise be told it was not found.
#[cfg(windows)]
const SHIM_NAMES: &[&str] = &["codex.cmd", "codex.ps1", "codex"];
#[cfg(not(windows))]
const SHIM_NAMES: &[&str] = &[];

/// Builds the ordered list of paths to try.
///
/// A user-provided path is considered alone. If the user pointed Command
/// Center at something, silently falling back to a different Codex would hide
/// their mistake and use an executable they did not choose.
pub fn candidate_paths(
    saved_path: Option<&Path>,
    search_path: Option<&str>,
    home: Option<&Path>,
) -> Vec<Candidate> {
    if let Some(path) = saved_path {
        return vec![Candidate {
            path: path.to_path_buf(),
            source: CandidateSource::UserProvided,
        }];
    }

    let mut candidates = Vec::new();
    let mut seen = std::collections::HashSet::new();

    let mut push = |path: PathBuf, source: CandidateSource, seen: &mut std::collections::HashSet<PathBuf>| {
        if seen.insert(path.clone()) {
            candidates.push(Candidate { path, source });
        }
    };

    if let Some(raw) = search_path {
        for dir in std::env::split_paths(raw) {
            if dir.as_os_str().is_empty() {
                continue;
            }
            for name in EXECUTABLE_NAMES.iter().chain(SHIM_NAMES) {
                push(dir.join(name), CandidateSource::SearchPath, &mut seen);
            }
        }
    }

    for location in known_locations(home) {
        push(location, CandidateSource::KnownLocation, &mut seen);
    }

    candidates
}

/// Install locations the official package uses that are not always on `PATH`,
/// notably when the app is launched from a desktop entry rather than a shell.
fn known_locations(home: Option<&Path>) -> Vec<PathBuf> {
    let mut locations = Vec::new();

    #[cfg(windows)]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            locations.push(PathBuf::from(appdata).join("npm").join("codex.exe"));
        }
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            locations.push(PathBuf::from(local).join("Programs").join("codex").join("codex.exe"));
        }
    }

    #[cfg(not(windows))]
    {
        locations.push(PathBuf::from("/usr/local/bin/codex"));
        locations.push(PathBuf::from("/usr/bin/codex"));
        locations.push(PathBuf::from("/opt/homebrew/bin/codex"));
        if let Some(home) = home {
            locations.push(home.join(".local/bin/codex"));
            locations.push(home.join(".npm-global/bin/codex"));
            locations.push(home.join(".bun/bin/codex"));
        }
    }

    let _ = home;
    locations
}

/// Judges a candidate path before spending a process on it. Returns `Ok(())`
/// when the path is worth probing.
pub fn inspect_path(path: &Path) -> Result<(), UnusableReason> {
    if is_shim(path) {
        return Err(UnusableReason::ShimNotSupported);
    }

    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(_) => return Err(UnusableReason::Missing),
    };
    if !metadata.is_file() {
        return Err(UnusableReason::Missing);
    }
    if !is_executable(&metadata, path) {
        return Err(UnusableReason::NotExecutable);
    }
    Ok(())
}

fn is_shim(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    SHIM_NAMES.iter().any(|shim| shim.eq_ignore_ascii_case(name))
}

#[cfg(unix)]
fn is_executable(metadata: &std::fs::Metadata, _path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_metadata: &std::fs::Metadata, path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
}

/// Turns a successfully probed version into a final discovery result.
pub fn classify(path: PathBuf, source: CandidateSource, version: CodexVersion) -> CodexDiscovery {
    if version.meets_minimum() {
        CodexDiscovery::Found {
            path,
            version,
            source,
        }
    } else {
        CodexDiscovery::TooOld { path, version }
    }
}

/// Runs discovery. The only part of this module that starts a process.
pub async fn discover(saved_path: Option<&Path>) -> CodexDiscovery {
    let search_path = std::env::var("PATH").ok();
    let home = home_dir();
    let candidates = candidate_paths(saved_path, search_path.as_deref(), home.as_deref());

    // The first candidate that fails in an interesting way is remembered, so a
    // user who has a shim on PATH is told about the shim rather than being
    // told nothing was found.
    let mut first_problem: Option<CodexDiscovery> = None;

    for candidate in candidates {
        let problem = match inspect_path(&candidate.path) {
            Ok(()) => match probe_version(&candidate.path).await {
                Ok(version) => return classify(candidate.path, candidate.source, version),
                Err(reason) => reason,
            },
            // A guessed path that simply is not there is the normal case and
            // never worth reporting.
            Err(UnusableReason::Missing) if candidate.source != CandidateSource::UserProvided => {
                continue
            }
            Err(reason) => reason,
        };

        let result = CodexDiscovery::Unusable {
            path: candidate.path,
            reason: problem,
        };
        // A path the user typed is authoritative: report its failure directly
        // instead of continuing to look elsewhere.
        if candidate.source == CandidateSource::UserProvided {
            return result;
        }
        first_problem.get_or_insert(result);
    }

    first_problem.unwrap_or(CodexDiscovery::NotFound)
}

async fn probe_version(path: &Path) -> Result<CodexVersion, UnusableReason> {
    let mut command = tokio::process::Command::new(path);
    command
        .arg("--version")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    #[cfg(windows)]
    {
        // Do not flash a console window when the app probes for Codex.
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let output = match tokio::time::timeout(VERSION_PROBE_TIMEOUT, command.output()).await {
        Err(_) => return Err(UnusableReason::Timeout),
        Ok(Err(_)) => return Err(UnusableReason::NotExecutable),
        Ok(Ok(output)) => output,
    };

    if !output.status.success() {
        return Err(UnusableReason::NoVersion);
    }

    // Bound the text before parsing. A wrong executable can print anything.
    let printed = String::from_utf8_lossy(&output.stdout);
    CodexVersion::parse(printed.trim()).ok_or(UnusableReason::NoVersion)
}

fn home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    let raw = std::env::var_os("USERPROFILE");
    #[cfg(not(windows))]
    let raw = std::env::var_os("HOME");
    raw.map(PathBuf::from).filter(|path| !path.as_os_str().is_empty())
}

pub fn minimum_version() -> CodexVersion {
    MINIMUM_CODEX_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_saved_path_is_the_only_candidate() {
        let saved = PathBuf::from("/opt/custom/codex");
        let candidates = candidate_paths(Some(&saved), Some("/usr/bin:/usr/local/bin"), None);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].path, saved);
        assert_eq!(candidates[0].source, CandidateSource::UserProvided);
    }

    #[test]
    fn search_path_entries_come_before_known_locations() {
        let candidates = candidate_paths(None, Some("/first:/second"), None);
        let sources: Vec<_> = candidates.iter().map(|c| c.source).collect();

        let last_search = sources
            .iter()
            .rposition(|source| *source == CandidateSource::SearchPath)
            .expect("expected PATH candidates");
        let first_known = sources
            .iter()
            .position(|source| *source == CandidateSource::KnownLocation);

        if let Some(first_known) = first_known {
            assert!(last_search < first_known);
        }
    }

    #[test]
    fn duplicate_paths_are_only_tried_once() {
        let candidates = candidate_paths(None, Some("/usr/bin:/usr/bin:/usr/bin"), None);
        let matching = candidates
            .iter()
            .filter(|candidate| candidate.path.starts_with("/usr/bin"))
            .count();

        assert_eq!(matching, EXECUTABLE_NAMES.len() + SHIM_NAMES.len());
    }

    #[test]
    fn empty_path_segments_are_skipped() {
        let candidates = candidate_paths(None, Some(""), None);
        assert!(candidates
            .iter()
            .all(|candidate| candidate.source != CandidateSource::SearchPath));
    }

    #[test]
    fn a_missing_file_is_reported_as_missing() {
        let dir = tempfile::tempdir().unwrap();
        let absent = dir.path().join("codex");
        assert_eq!(inspect_path(&absent), Err(UnusableReason::Missing));
    }

    #[test]
    fn a_directory_is_not_mistaken_for_an_executable() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(inspect_path(dir.path()), Err(UnusableReason::Missing));
    }

    #[cfg(unix)]
    #[test]
    fn a_non_executable_file_is_rejected_before_it_is_run() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("codex");
        std::fs::write(&path, "#!/bin/sh\necho 0.147.0\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

        assert_eq!(inspect_path(&path), Err(UnusableReason::NotExecutable));
    }

    #[cfg(unix)]
    #[test]
    fn an_executable_file_is_worth_probing() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("codex");
        std::fs::write(&path, "#!/bin/sh\necho 0.147.0\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();

        assert_eq!(inspect_path(&path), Ok(()));
    }

    #[test]
    fn the_version_floor_decides_found_from_too_old() {
        let path = PathBuf::from("/usr/bin/codex");
        let current = CodexVersion::parse("0.147.0").unwrap();
        let ancient = CodexVersion::parse("0.100.0").unwrap();

        assert!(matches!(
            classify(path.clone(), CandidateSource::SearchPath, current),
            CodexDiscovery::Found { .. }
        ));
        assert!(matches!(
            classify(path, CandidateSource::SearchPath, ancient),
            CodexDiscovery::TooOld { .. }
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_saved_path_that_is_missing_does_not_fall_back_to_path() {
        let dir = tempfile::tempdir().unwrap();
        let absent = dir.path().join("nowhere/codex");

        let result = discover(Some(&absent)).await;

        assert_eq!(
            result,
            CodexDiscovery::Unusable {
                path: absent,
                reason: UnusableReason::Missing,
            }
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_stand_in_executable_reporting_a_current_version_is_accepted() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("codex");
        std::fs::write(&path, "#!/bin/sh\necho 'codex-cli 0.147.0'\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();

        match discover(Some(&path)).await {
            CodexDiscovery::Found { version, .. } => {
                assert_eq!(version.to_string(), "0.147.0");
            }
            other => panic!("expected a usable Codex, got {other:?}"),
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn an_old_stand_in_executable_reports_the_version_it_printed() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("codex");
        std::fs::write(&path, "#!/bin/sh\necho 'codex-cli 0.100.0'\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();

        match discover(Some(&path)).await {
            CodexDiscovery::TooOld { version, .. } => {
                assert_eq!(version.to_string(), "0.100.0");
            }
            other => panic!("expected a too-old result, got {other:?}"),
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn an_executable_that_prints_nothing_useful_is_unusable() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("codex");
        std::fs::write(&path, "#!/bin/sh\necho 'hello'\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();

        assert_eq!(
            discover(Some(&path)).await,
            CodexDiscovery::Unusable {
                path,
                reason: UnusableReason::NoVersion,
            }
        );
    }
}
