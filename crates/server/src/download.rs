//! Files leaving the server.
//!
//! On the desktop, exporting ends at a save dialog. In a browser it ends at a
//! download, so the same core function that produced a file now produces a
//! response body with a name attached to it.

use axum::body::Body;
use axum::http::header::{CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::http::StatusCode;
use axum::response::Response;

pub const MARKDOWN: &str = "text/markdown; charset=utf-8";
pub const SQLITE: &str = "application/vnd.sqlite3";

/// Builds the download. The filename is quoted and stripped of anything that
/// would let a library or collection name break out of the header.
pub fn attachment(bytes: Vec<u8>, content_type: &str, filename: &str) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, content_type)
        .header(
            CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", safe_filename(filename)),
        )
        // An export is a snapshot of the moment it was asked for. A cached one
        // is a stale one.
        .header(CACHE_CONTROL, "no-store")
        .body(Body::from(bytes))
        // The only way to fail here is a malformed header, and every part of
        // this one is either a constant or already sanitised.
        .expect("the download response is well formed")
}

/// Keeps a name to plain characters. A collection can be called anything at
/// all, including something with a quote or a newline in it.
fn safe_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|character| match character {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '-' | '_' => character,
            _ => '-',
        })
        .collect();

    let trimmed = cleaned.trim_matches('-');

    if trimmed.is_empty() {
        "export".to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// Turns a collection name into the middle of a filename: lowercase, words
/// joined by single dashes, nothing else.
pub fn slug(name: &str) -> String {
    let mut slug = String::new();
    let mut pending_dash = false;

    for character in name.chars() {
        if character.is_ascii_alphanumeric() {
            if pending_dash && !slug.is_empty() {
                slug.push('-');
            }
            pending_dash = false;
            slug.push(character.to_ascii_lowercase());
        } else {
            pending_dash = true;
        }
    }

    slug
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_with_a_quote_in_it_cannot_break_the_header() {
        assert_eq!(safe_filename("we\"ird\r\nname.md"), "we-ird--name.md");
    }

    #[test]
    fn a_name_with_nothing_usable_left_still_downloads() {
        assert_eq!(safe_filename("///"), "export");
    }

    #[test]
    fn a_collection_name_becomes_dashed_lowercase() {
        assert_eq!(slug("Docker & Compose"), "docker-compose");
        assert_eq!(slug("  Git  "), "git");
    }

    /// A collection named entirely in characters a filename cannot hold still
    /// has to produce a download, so the caller falls back on an empty slug.
    #[test]
    fn a_collection_name_with_no_ascii_letters_slugs_to_nothing() {
        assert!(slug("日本語").is_empty());
    }

    #[test]
    fn the_response_names_the_file_and_refuses_caching() {
        let response = attachment(b"# Notes".to_vec(), MARKDOWN, "library.md");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(CONTENT_DISPOSITION)
                .unwrap()
                .to_str()
                .unwrap(),
            "attachment; filename=\"library.md\""
        );
        assert_eq!(response.headers().get(CACHE_CONTROL).unwrap(), "no-store");
    }
}
