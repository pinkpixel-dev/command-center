//! Serving the web bundle.
//!
//! In the container the frontend and the API are one origin, which is what
//! lets the browser client use relative paths and send its session cookie
//! without any cross-origin arrangement. In development Vite serves the
//! frontend instead and proxies `/api` here, so this is allowed to be absent.

use std::path::Path;

use axum::http::header::CACHE_CONTROL;
use axum::http::HeaderValue;
use axum::Router;
use tower::ServiceBuilder;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;

/// Vite writes content-hashed filenames into `assets`, so a given URL there
/// never changes what it answers with. Anything else is asked about again.
const IMMUTABLE: &str = "public, max-age=31536000, immutable";

/// Not "no-store": revalidating costs one 304 and keeps a stale bundle from
/// outliving a container update.
const REVALIDATE: &str = "no-cache";

/// Adds the static routes underneath the API.
///
/// Everything that is not a file becomes `index.html`, which is what any page
/// the app puts in the address bar needs. The API is nested above this, so it
/// is never what a request for a missing file falls back to.
pub fn serve(router: Router, web_dir: &Path) -> Router {
    let assets = ServiceBuilder::new()
        .layer(cache(IMMUTABLE))
        .service(ServeDir::new(web_dir.join("assets")));

    let files = ServiceBuilder::new().layer(cache(REVALIDATE)).service(
        ServeDir::new(web_dir).fallback(ServeFile::new(web_dir.join("index.html"))),
    );

    router.nest_service("/assets", assets).fallback_service(files)
}

fn cache(value: &'static str) -> SetResponseHeaderLayer<HeaderValue> {
    SetResponseHeaderLayer::overriding(CACHE_CONTROL, HeaderValue::from_static(value))
}
