//! Everything Command Center does that is not tied to a single frontend.
//!
//! The desktop app and the self-hosted server both build on this crate. It
//! owns the library database, import and export, and the AI providers. It
//! knows nothing about Tauri, HTTP, windows, or trays, which is what lets the
//! two applications share it without sharing a shell.

pub mod ai;
pub mod db;
pub mod error;
pub mod events;
pub mod export;
pub mod import;
pub mod library;
pub mod models;
pub mod normalize;
pub mod risk;
pub mod workflows;

/// True on any build that can spawn a child process, which means every target
/// except Android and iOS.
///
/// Tauri's build script sets a `desktop` cfg with exactly this meaning, but it
/// only sets it for crates that depend on Tauri. This crate does not, so it
/// works the condition out for itself. Codex is the one feature that cares,
/// because it runs as a local child process.
pub const IS_DESKTOP: bool = !cfg!(any(target_os = "android", target_os = "ios"));
