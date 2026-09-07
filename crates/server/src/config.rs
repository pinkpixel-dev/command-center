//! What the container tells the server, and the defaults it falls back to.

use std::net::SocketAddr;
use std::path::PathBuf;

/// Everything under one mount, so a NAS operator maps a single volume and gets
/// the library, the Codex home, and anything added later.
pub const DATA_DIR_VAR: &str = "COMMAND_CENTER_DATA_DIR";
pub const ADDR_VAR: &str = "COMMAND_CENTER_ADDR";

const DEFAULT_DATA_DIR: &str = "/var/lib/command-center";
const DEFAULT_ADDR: &str = "0.0.0.0:8787";

#[derive(Debug, Clone)]
pub struct Config {
    pub data_dir: PathBuf,
    pub addr: SocketAddr,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let data_dir = std::env::var(DATA_DIR_VAR).unwrap_or_else(|_| DEFAULT_DATA_DIR.to_owned());
        let addr = std::env::var(ADDR_VAR).unwrap_or_else(|_| DEFAULT_ADDR.to_owned());

        Ok(Self {
            data_dir: PathBuf::from(data_dir),
            addr: parse_addr(&addr)?,
        })
    }

    /// The SQLite file, next to nothing else, so a backup is one file copy.
    pub fn library_path(&self) -> PathBuf {
        self.data_dir.join("library.db")
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

    #[test]
    fn the_library_sits_directly_in_the_mounted_data_directory() {
        let config = Config {
            data_dir: PathBuf::from("/data"),
            addr: parse_addr(DEFAULT_ADDR).unwrap(),
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
}
