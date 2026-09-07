//! How the Codex app server is started: arguments, hardening overrides, and
//! the environment it is allowed to see.
//!
//! Everything here is pure so the launch contract can be asserted in tests
//! without starting a process. Each override was checked against Codex 0.147.0
//! with `--strict-config`, which validates both key names and values.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The client name Codex reports upstream. It identifies this app in OpenAI's
/// compliance logging, so it must stay stable and must not be a placeholder or
/// borrowed from another product.
pub const CLIENT_NAME: &str = "pinkpixel_command_center";
pub const CLIENT_TITLE: &str = "Command Center";

/// Hardening overrides passed as `-c key=value`.
///
/// `--strict-config` turns a renamed or removed key into a startup failure
/// instead of a silently dropped safety setting. That matters because Command
/// Center does not control which Codex version the user has installed.
pub const CONFIG_OVERRIDES: &[&str] = &[
    // Credentials go to the operating-system store or nowhere. `file` and
    // `auto` can both write tokens into the Codex home directory.
    r#"cli_auth_credentials_store="keyring""#,
    // Tool containment. Neither of these removes Codex's shell or file tools,
    // because no such switch exists. `read-only` is the boundary that matters:
    // no writes, and no network unless it is asked for, which it is not.
    //
    // `never` is not "never ask and always run", which is what it sounds like.
    // In Codex's own safety check it is the value that *rejects* anything the
    // sandbox will not allow, where `on-request` would ask a user who is not
    // there. `untrusted` used to be set here and was stricter still, but Codex
    // 0.149.1 retired it as a config key, and `--strict-config` turns a
    // retired key into a startup failure by design.
    r#"sandbox_mode="read-only""#,
    r#"approval_policy="never""#,
    // Command Center sends its own bounded input and expects structured JSON
    // back. None of these belong in that loop.
    r#"web_search="disabled""#,
    "analytics.enabled=false",
    r#"history.persistence="none""#,
    "mcp_servers={}",
    "notify=[]",
];

/// Environment variables the process is allowed to inherit.
///
/// The list is an allowlist rather than a denylist because Codex captures a
/// shell snapshot into its home directory at thread start. What is not passed
/// in cannot be captured, which makes this a privacy control and not only
/// process hygiene.
const INHERITED_ENV: &[&str] = &[
    // Process startup.
    "PATH",
    // Certificate discovery and corporate proxies. Without these, a user
    // behind a proxy cannot reach OpenAI at all.
    "SSL_CERT_FILE",
    "SSL_CERT_DIR",
    "HTTPS_PROXY",
    "https_proxy",
    "HTTP_PROXY",
    "http_proxy",
    "NO_PROXY",
    "no_proxy",
    // Locale, so Codex formats its own output sensibly.
    "LANG",
    "LC_ALL",
];

/// Unix-only additions. The Secret Service keyring is reached over D-Bus, so
/// omitting these would make keyring-only credential storage fail on Linux.
#[cfg(unix)]
const INHERITED_ENV_PLATFORM: &[&str] = &[
    "HOME",
    "USER",
    "DBUS_SESSION_BUS_ADDRESS",
    "XDG_RUNTIME_DIR",
    "XDG_DATA_HOME",
    "XDG_CONFIG_HOME",
];

/// Windows needs its system paths for process and TLS startup, and the
/// Credential Manager is reached through the user profile.
#[cfg(windows)]
const INHERITED_ENV_PLATFORM: &[&str] = &[
    "USERPROFILE",
    "APPDATA",
    "LOCALAPPDATA",
    "SystemRoot",
    "SystemDrive",
    "windir",
    "TEMP",
    "TMP",
    "PATHEXT",
    "NUMBER_OF_PROCESSORS",
    "PROCESSOR_ARCHITECTURE",
];

/// A complete description of how to start the app server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchPlan {
    pub args: Vec<String>,
    /// The full environment for the child. Nothing else is inherited.
    pub env: BTreeMap<String, OsString>,
    pub codex_home: PathBuf,
    pub working_dir: PathBuf,
}

/// Builds the launch plan.
///
/// `codex_home` is Command Center's own Codex directory, kept away from the
/// user's `~/.codex` so connecting or disconnecting here never disturbs their
/// own Codex login. `working_dir` is the empty private directory that threads
/// run in.
pub fn launch_plan(codex_home: &Path, working_dir: &Path, source_env: &EnvSource) -> LaunchPlan {
    let mut args = vec!["app-server".to_owned(), "--strict-config".to_owned()];
    for override_value in CONFIG_OVERRIDES {
        args.push("-c".to_owned());
        args.push((*override_value).to_owned());
    }

    let mut env = BTreeMap::new();
    for name in INHERITED_ENV.iter().chain(INHERITED_ENV_PLATFORM) {
        if let Some(value) = source_env.get(name) {
            if !value.is_empty() {
                env.insert((*name).to_owned(), value);
            }
        }
    }
    // Set last so it cannot be shadowed by an inherited value.
    env.insert("CODEX_HOME".to_owned(), codex_home.as_os_str().to_owned());

    LaunchPlan {
        args,
        env,
        codex_home: codex_home.to_path_buf(),
        working_dir: working_dir.to_path_buf(),
    }
}

/// Where environment values are read from. Tests supply a fixed map instead of
/// the real process environment.
pub enum EnvSource {
    Process,
    Fixed(BTreeMap<String, OsString>),
}

impl EnvSource {
    fn get(&self, name: &str) -> Option<OsString> {
        match self {
            Self::Process => std::env::var_os(name),
            Self::Fixed(values) => values.get(name).cloned(),
        }
    }
}

/// The `initialize` parameters Command Center sends.
pub fn initialize_params(app_version: &str) -> serde_json::Value {
    serde_json::json!({
        "clientInfo": {
            "name": CLIENT_NAME,
            "title": CLIENT_TITLE,
            "version": app_version,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed_env() -> EnvSource {
        let mut values = BTreeMap::new();
        values.insert("PATH".to_owned(), OsString::from("/usr/bin"));
        values.insert("LANG".to_owned(), OsString::from("en_US.UTF-8"));
        values.insert("OPENAI_API_KEY".to_owned(), OsString::from("sk-secret"));
        values.insert("AWS_SECRET_ACCESS_KEY".to_owned(), OsString::from("secret"));
        values.insert("CODEX_HOME".to_owned(), OsString::from("/home/user/.codex"));
        values.insert("EMPTY".to_owned(), OsString::new());
        EnvSource::Fixed(values)
    }

    fn plan() -> LaunchPlan {
        launch_plan(
            Path::new("/app-data/codex-home"),
            Path::new("/app-data/codex-work"),
            &fixed_env(),
        )
    }

    #[test]
    fn the_app_server_starts_with_strict_config() {
        let plan = plan();
        assert_eq!(plan.args[0], "app-server");
        assert!(
            plan.args.contains(&"--strict-config".to_owned()),
            "strict config is what turns a renamed hardening key into a loud failure",
        );
    }

    #[test]
    fn every_hardening_override_is_passed() {
        let plan = plan();
        for expected in CONFIG_OVERRIDES {
            assert!(
                plan.args.iter().any(|arg| arg == expected),
                "missing override {expected}",
            );
        }
        // Each override needs its own -c flag.
        let flags = plan.args.iter().filter(|arg| *arg == "-c").count();
        assert_eq!(flags, CONFIG_OVERRIDES.len());
    }

    #[test]
    fn credentials_are_pinned_to_the_operating_system_keyring() {
        assert!(CONFIG_OVERRIDES.contains(&r#"cli_auth_credentials_store="keyring""#));
        // A file or automatic store could put tokens on disk.
        assert!(!CONFIG_OVERRIDES.iter().any(|value| value.contains("\"file\"")
            || value.contains("\"auto\"")));
    }

    #[test]
    fn the_sandbox_is_read_only_and_nothing_escalates_out_of_it() {
        assert!(CONFIG_OVERRIDES.contains(&r#"sandbox_mode="read-only""#));
        assert!(CONFIG_OVERRIDES.contains(&r#"approval_policy="never""#));
        // Codex 0.149.1 retired this as a config key, and --strict-config
        // turns a retired key into a startup failure. Setting it here again
        // would stop the provider connecting on every current Codex.
        assert!(!CONFIG_OVERRIDES
            .iter()
            .any(|value| value.contains(r#"approval_policy="untrusted""#)));
        // The one value that must never appear: it would ask a user who is not
        // sitting there, and an unanswered prompt is a wedged turn.
        assert!(!CONFIG_OVERRIDES
            .iter()
            .any(|value| value.contains(r#"approval_policy="on-request""#)));
    }

    #[test]
    fn the_child_environment_is_an_allowlist() {
        let plan = plan();

        assert_eq!(plan.env.get("PATH").unwrap(), &OsString::from("/usr/bin"));
        assert_eq!(plan.env.get("LANG").unwrap(), &OsString::from("en_US.UTF-8"));
        assert!(!plan.env.contains_key("OPENAI_API_KEY"));
        assert!(!plan.env.contains_key("AWS_SECRET_ACCESS_KEY"));
    }

    #[test]
    fn empty_values_are_not_forwarded() {
        let plan = plan();
        assert!(!plan.env.contains_key("EMPTY"));
    }

    #[test]
    fn our_codex_home_overrides_any_inherited_one() {
        let plan = plan();
        assert_eq!(
            plan.env.get("CODEX_HOME").unwrap(),
            &OsString::from("/app-data/codex-home"),
        );
    }

    #[cfg(unix)]
    #[test]
    fn the_dbus_address_is_forwarded_so_the_linux_keyring_can_be_reached() {
        let mut values = BTreeMap::new();
        values.insert(
            "DBUS_SESSION_BUS_ADDRESS".to_owned(),
            OsString::from("unix:path=/run/user/1000/bus"),
        );
        let plan = launch_plan(
            Path::new("/home"),
            Path::new("/work"),
            &EnvSource::Fixed(values),
        );

        assert!(plan.env.contains_key("DBUS_SESSION_BUS_ADDRESS"));
    }

    #[test]
    fn the_client_identifies_itself_honestly() {
        let params = initialize_params("1.0.3");
        assert_eq!(params["clientInfo"]["name"], CLIENT_NAME);
        assert_eq!(params["clientInfo"]["version"], "1.0.3");
        // A placeholder or borrowed client name would misreport this app
        // to OpenAI's compliance logging.
        assert!(CLIENT_NAME.contains("command_center"));
        assert_ne!(CLIENT_NAME, "codex");
    }
}
