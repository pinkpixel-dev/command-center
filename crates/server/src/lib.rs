//! Self-hosted Command Center.
//!
//! The same library, the same import and export, and the same two AI
//! providers as the desktop app, served over HTTP instead of into a webview.
//! Everything that is not about being a server lives in `command-center-core`,
//! which the desktop app builds on too.

pub mod auth;
pub mod config;
pub mod credentials;
pub mod download;
pub mod error;
pub mod events;
pub mod json;
pub mod routes;
pub mod state;
pub mod upload;
pub mod web;
