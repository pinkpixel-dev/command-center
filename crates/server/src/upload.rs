//! Files arriving at the server.
//!
//! The desktop app is handed a path and reads it. A browser has no path worth
//! sending, so it posts the bytes as `multipart/form-data`, exactly the shape
//! a file input or a drop produces. What makes a document acceptable is core's
//! decision either way.

use axum::extract::multipart::MultipartError;
use axum::extract::{DefaultBodyLimit, Multipart};
use axum::http::StatusCode;

use command_center_core::error::{AppError, AppResult};
use command_center_core::import::document::{self, ImportDocument, MAX_FILE_BYTES};

/// The multipart envelope, the field headers, and the boundaries all sit on
/// top of the file, so the request is allowed to be a little larger than the
/// file limit itself. Anything past this is rejected before it is buffered.
pub const MAX_UPLOAD_BYTES: usize = MAX_FILE_BYTES as usize + 64 * 1024;

/// The body limit for the upload routes. Axum's default is 2 MB, which would
/// refuse a file core is happy to read, and refuse it less clearly.
pub fn body_limit() -> DefaultBodyLimit {
    DefaultBodyLimit::max(MAX_UPLOAD_BYTES)
}

/// Pulls the uploaded document out of the request. The first part carrying a
/// filename wins, so a client that adds fields later does not break this.
pub async fn document(mut multipart: Multipart) -> AppResult<ImportDocument> {
    while let Some(field) = multipart.next_field().await.map_err(rejected)? {
        let Some(name) = field.file_name().map(str::to_owned) else {
            continue;
        };

        // Checked before the bytes are collected, so a file we will not read
        // is refused without being buffered first.
        document::check_name(&name)?;

        let bytes = field.bytes().await.map_err(rejected)?;

        return document::from_bytes(Some(name), bytes.to_vec());
    }

    Err(AppError::invalid("No file was attached to that upload"))
}

/// Turns a multipart failure into something worth reading. A request stopped
/// at the body limit is really a file over the size limit, and saying so is
/// the difference between a person shrinking their file and giving up.
fn rejected(error: MultipartError) -> AppError {
    rejection_message(error.status(), &error.to_string())
}

fn rejection_message(status: StatusCode, detail: &str) -> AppError {
    if status == StatusCode::PAYLOAD_TOO_LARGE {
        // The one sentence core already uses for a file this size. The request
        // was stopped by the body limit, but the file limit is the number the
        // person can do something about.
        return document::check_size(MAX_FILE_BYTES + 1)
            .expect_err("a file over the limit is refused");
    }

    AppError::invalid(format!("Could not read that upload: {detail}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The body limit and the file limit are different numbers, and the one
    /// the person can act on is the file limit.
    #[test]
    fn a_request_stopped_at_the_body_limit_names_the_file_limit() {
        let error = rejection_message(StatusCode::PAYLOAD_TOO_LARGE, "length limit exceeded");

        assert!(error.to_string().contains("4 MB"));
    }

    #[test]
    fn a_broken_upload_says_so_rather_than_blaming_the_size() {
        let error = rejection_message(StatusCode::BAD_REQUEST, "boundary not found");

        assert!(error.to_string().contains("boundary not found"));
    }

    /// The limit has to leave room for the envelope, or a file right at the
    /// size core allows would be refused by the layer above it.
    #[test]
    fn the_request_limit_is_larger_than_the_file_limit() {
        assert!(MAX_UPLOAD_BYTES > MAX_FILE_BYTES as usize);
    }
}
