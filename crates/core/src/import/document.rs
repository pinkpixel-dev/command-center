//! What counts as a document worth parsing, whichever way it arrived.
//!
//! The desktop app reads a path the user picked. The server is handed an
//! upload. Both end up with a filename and some bytes, and both have to answer
//! the same three questions: is this a kind of file we read, is it small
//! enough, and is it text. Keeping the answers here means the two apps refuse
//! the same file with the same sentence.

use serde::Serialize;

use crate::error::{AppError, AppResult};

/// Extensions the file picker, drag-drop, and the upload endpoint accept.
/// Anything else is almost certainly not a document worth parsing.
pub const READABLE_EXTENSIONS: [&str; 9] = [
    "md", "markdown", "mdx", "txt", "text", "rst", "adoc", "org", "sh",
];

/// A file bigger than this is not a cheat sheet, and parsing it would freeze
/// the window, or tie up the server, for no good reason.
pub const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;

/// A document ready for the importer, read on this side of the boundary so the
/// webview never needs filesystem access of its own.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportDocument {
    pub name: Option<String>,
    pub content: String,
}

/// Checks the filename before anything is read. Files with no extension at all
/// pass, because README and LICENSE style files are worth importing.
pub fn check_name(name: &str) -> AppResult<()> {
    let extension = name
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_lowercase())
        .unwrap_or_default();

    if extension.is_empty() || READABLE_EXTENSIONS.contains(&extension.as_str()) {
        return Ok(());
    }

    Err(AppError::invalid(format!(
        "Command Center reads text and Markdown files. \"{name}\" is a .{extension} file."
    )))
}

/// The size the file itself reports, checked before the bytes are read.
pub fn check_size(bytes: u64) -> AppResult<()> {
    if bytes > MAX_FILE_BYTES {
        return Err(AppError::invalid(
            "That file is larger than 4 MB. Paste the part you want instead.",
        ));
    }

    Ok(())
}

/// Turns bytes into a document, applying every rule. Safe to call on an upload
/// whose size was never announced, since the length is checked here too.
pub fn from_bytes(name: Option<String>, bytes: Vec<u8>) -> AppResult<ImportDocument> {
    if let Some(name) = name.as_deref() {
        check_name(name)?;
    }

    check_size(bytes.len() as u64)?;

    let content = String::from_utf8(bytes)
        .map_err(|_| AppError::invalid("That file is not text Command Center can read"))?;

    Ok(ImportDocument { name, content })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_markdown_file_is_readable_whatever_case_it_is_written_in() {
        assert!(check_name("NOTES.MD").is_ok());
        assert!(check_name("notes.md").is_ok());
    }

    /// README and LICENSE have no extension, and both are worth importing.
    #[test]
    fn a_file_with_no_extension_is_allowed() {
        assert!(check_name("README").is_ok());
    }

    #[test]
    fn a_refused_file_is_named_along_with_its_type() {
        let error = check_name("diagram.png").unwrap_err().to_string();

        assert!(error.contains("diagram.png"));
        assert!(error.contains(".png"));
    }

    #[test]
    fn a_file_over_the_limit_is_refused_with_the_limit_named() {
        let error = check_size(MAX_FILE_BYTES + 1).unwrap_err().to_string();

        assert!(error.contains("4 MB"));
        assert!(check_size(MAX_FILE_BYTES).is_ok());
    }

    /// An upload arrives with no length to trust, so the bytes themselves have
    /// to be measured rather than whatever the client claimed.
    #[test]
    fn oversized_bytes_are_refused_even_without_a_declared_size() {
        let error = from_bytes(Some("big.md".to_owned()), vec![b'x'; 4 * 1024 * 1024 + 1])
            .unwrap_err()
            .to_string();

        assert!(error.contains("4 MB"));
    }

    #[test]
    fn a_binary_upload_is_refused_rather_than_mangled() {
        let error = from_bytes(Some("notes.md".to_owned()), vec![0xff, 0xfe, 0x00])
            .unwrap_err()
            .to_string();

        assert!(error.contains("not text"));
    }

    #[test]
    fn a_text_upload_keeps_its_name_and_content() {
        let document = from_bytes(Some("notes.md".to_owned()), b"# Notes\n".to_vec()).unwrap();

        assert_eq!(document.name.as_deref(), Some("notes.md"));
        assert_eq!(document.content, "# Notes\n");
    }
}
