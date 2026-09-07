//! The ChatGPT / Codex provider.
//!
//! Codex owns the ChatGPT login, token storage, and refresh. Command Center
//! owns discovery, process lifecycle, and the JSON-RPC conversation. No token
//! ever reaches this crate, the database, or the frontend.

pub mod account;
pub mod auth;
pub mod discovery;
pub mod launch;
pub mod lines;
pub mod models;
pub mod process;
pub mod rpc;
pub mod service;
pub mod turn;
pub mod version;

pub use discovery::{discover, CodexDiscovery, UnusableReason};
pub use account::CodexAccount;
pub use service::{CodexAvailability, CodexService};
pub use version::{CodexVersion, MINIMUM_CODEX_VERSION};
