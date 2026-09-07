//! What the container tells the server, and the defaults it falls back to.

use std::net::SocketAddr;
use std::path::PathBuf;

/// Everything under one mount, so a NAS operator maps a single volume and gets
/// the library, the Codex home, and anything added later.
pub const DATA_DIR_VAR: &str = "COMMAND_CENTER_DATA_DIR";
pub const ADDR_VAR: &str = "COMMAND_CENTER_ADDR";
pub const WEB_DIR_VAR: &str = "COMMAND_CENTER_WEB_DIR";

const DEFAULT_DATA_DIR: &str = "/var/lib/command-center";
const DEFAULT_ADDR: &str = "0.0.0.0:8787";
/// Where the image puts the built frontend. Absent everywhere else, which is
/// how a development server ends up API-only without being told.
const DEFAULT_WEB_DIR: &str = "/usr/share/command-center/web";

#[derive(Debug, Clone)]
pub struct Config {
    pub data_dir: PathBuf,
    pub addr: SocketAddr,
    /// None when there is no bundle to serve, which means the API alone.
    pub web_dir: Option<PathBuf>,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let data_dir = std::env::var(DATA_DIR_VAR).unwrap_or_else(|_| DEFAULT_DATA_DIR.to_owned());
        let addr = std::env::var(ADDR_VAR).unwrap_or_else(|_| DEFAULT_ADDR.to_owned());

        Ok(Self {
            data_dir: PathBuf::from(data_dir),
            addr: parse_addr(&addr)?,
            web_dir: web_dir()?,
        })
    }

    /// The SQLite file, next to nothing else, so a backup is one file copy.
    pub fn library_path(&self) -> PathBuf {
        self.data_dir.join("library.db")
    }
}

/// A directory the operator named has to be there. The default one not being
/// there is normal: it means this is not the container, and Vite is serving
/// the frontend somewhere else.
fn web_dir() -> Result<Option<PathBuf>, String> {
    let configured = std::env::var(WEB_DIR_VAR)
        .ok()
        .filter(|value| !value.trim().is_empty());

    let path = PathBuf::from(configured.clone().unwrap_or_else(|| DEFAULT_WEB_DIR.to_owned()));

    if path.join("index.html").is_file() {
        return Ok(Some(path));
    }

    match configured {
        Some(_) => Err(format!(
            "{WEB_DIR_VAR} is set to {} but there is no index.html in it.",
            path.display()
        )),
        None => Ok(None),
    }
}

fn parse_addr(value: &str) -> Result<SocketAddr, String> {
    value.parse().map_err(|_| {
        format!("{ADDR_VAR} is not a valid address and port: \"{value}\". Try 0.0.0.0:8787.")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn the_library_sits_directly_in_the_mounted_data_directory() {
        let config = Config {
            data_dir: PathBuf::from("/data"),
            addr: parse_addr(DEFAULT_ADDR).unwrap(),
            web_dir: None,
        };

        assert_eq!(config.library_path(), PathBuf::from("/data/library.db"));
    }

    #[test]
    fn a_misspelled_address_is_refused_with_the_variable_named() {
        let error = parse_addr("8787").unwrap_err();

        assert!(error.contains(ADDR_VAR));
        assert!(error.contains("0.0.0.0:8787"));
    }

    #[test]
    fn an_address_with_a_port_parses() {
        assert_eq!(parse_addr("127.0.0.1:9000").unwrap().port(), 9000);
    }

    /// These read the process environment, which every test in this binary
    /// shares, so they take turns.
    static ENV: Mutex<()> = Mutex::new(());

    #[test]
    fn a_bundle_is_found_by_its_index_file() {
        let _guard = ENV.lock().unwrap_or_else(|error| error.into_inner());
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("index.html"), "<!doctype html>").unwrap();

        std::env::set_var(WEB_DIR_VAR, dir.path());
        let found = web_dir();
        std::env::remove_var(WEB_DIR_VAR);

        assert_eq!(found.unwrap().as_deref(), Some(dir.path()));
    }

    /// Pointing at the wrong directory has to be a startup failure. Silently
    /// serving the API alone would look like a frontend that failed to load.
    #[test]
    fn a_named_directory_with_no_bundle_in_it_stops_the_server() {
        let _guard = ENV.lock().unwrap_or_else(|error| error.into_inner());
        let dir = tempfile::tempdir().unwrap();

        std::env::set_var(WEB_DIR_VAR, dir.path());
        let error = web_dir().unwrap_err();
        std::env::remove_var(WEB_DIR_VAR);

        assert!(error.contains(WEB_DIR_VAR));
        assert!(error.contains("index.html"));
    }

    #[test]
    fn no_bundle_and_nobody_asking_for_one_is_the_api_alone() {
        let _guard = ENV.lock().unwrap_or_else(|error| error.into_inner());
        std::env::remove_var(WEB_DIR_VAR);

        assert_eq!(web_dir().unwrap(), None);
    }
}
