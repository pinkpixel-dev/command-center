//! A length-bounded newline-delimited reader.
//!
//! `AsyncBufReadExt::lines` grows its buffer until it finds a newline, so a
//! process that never sends one could make the app allocate without limit.
//! This reader refuses a line once it passes the ceiling instead, which turns
//! a runaway stream into one clean failure.

use tokio::io::{AsyncRead, AsyncReadExt};

/// How much is pulled from the pipe per read.
const CHUNK_BYTES: usize = 8 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub enum LineError {
    /// A single line passed the ceiling before a newline arrived.
    TooLong,
    /// The underlying stream failed.
    Io(String),
}

pub struct BoundedLines<R> {
    reader: R,
    buffer: Vec<u8>,
    max_line: usize,
}

impl<R: AsyncRead + Unpin> BoundedLines<R> {
    pub fn new(reader: R, max_line: usize) -> Self {
        Self {
            reader,
            buffer: Vec::new(),
            max_line,
        }
    }

    /// Returns the next line without its terminator, or `None` at end of
    /// stream. Invalid UTF-8 is replaced rather than failing the connection,
    /// because the parser rejects the result anyway and a lost byte should not
    /// look like a crash.
    pub async fn next_line(&mut self) -> Result<Option<String>, LineError> {
        loop {
            if let Some(index) = self.buffer.iter().position(|byte| *byte == b'\n') {
                // Check the completed line's own length. A single read can
                // deliver an oversized line and its terminator together, so
                // testing only the pending buffer would let it through.
                if index > self.max_line {
                    return Err(LineError::TooLong);
                }
                let mut line: Vec<u8> = self.buffer.drain(..=index).collect();
                line.pop();
                if line.last() == Some(&b'\r') {
                    line.pop();
                }
                return Ok(Some(String::from_utf8_lossy(&line).into_owned()));
            }

            if self.buffer.len() > self.max_line {
                return Err(LineError::TooLong);
            }

            let mut chunk = [0_u8; CHUNK_BYTES];
            let read = self
                .reader
                .read(&mut chunk)
                .await
                .map_err(|error| LineError::Io(error.kind().to_string()))?;

            if read == 0 {
                if self.buffer.is_empty() {
                    return Ok(None);
                }
                if self.buffer.len() > self.max_line {
                    return Err(LineError::TooLong);
                }
                // A final line without a trailing newline is still a line.
                let line = std::mem::take(&mut self.buffer);
                return Ok(Some(String::from_utf8_lossy(&line).into_owned()));
            }

            self.buffer.extend_from_slice(&chunk[..read]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    async fn collect(input: &str, max: usize) -> Result<Vec<String>, LineError> {
        let mut lines = BoundedLines::new(Cursor::new(input.as_bytes().to_vec()), max);
        let mut out = Vec::new();
        while let Some(line) = lines.next_line().await? {
            out.push(line);
        }
        Ok(out)
    }

    #[tokio::test]
    async fn splits_on_newlines() {
        let lines = collect("{\"a\":1}\n{\"b\":2}\n", 1024).await.unwrap();
        assert_eq!(lines, vec![r#"{"a":1}"#, r#"{"b":2}"#]);
    }

    #[tokio::test]
    async fn a_final_line_without_a_newline_is_still_returned() {
        let lines = collect("first\nsecond", 1024).await.unwrap();
        assert_eq!(lines, vec!["first", "second"]);
    }

    #[tokio::test]
    async fn carriage_returns_are_stripped() {
        let lines = collect("first\r\nsecond\r\n", 1024).await.unwrap();
        assert_eq!(lines, vec!["first", "second"]);
    }

    #[tokio::test]
    async fn an_empty_stream_ends_immediately() {
        assert!(collect("", 1024).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn blank_lines_are_preserved_rather_than_skipped() {
        // The dispatcher decides what to do with a blank line; the reader does
        // not get to silently drop input.
        let lines = collect("a\n\nb\n", 1024).await.unwrap();
        assert_eq!(lines, vec!["a", "", "b"]);
    }

    #[tokio::test]
    async fn a_line_past_the_ceiling_fails_instead_of_growing() {
        let flood = format!("{}\n", "x".repeat(4096));
        assert_eq!(collect(&flood, 512).await, Err(LineError::TooLong));
    }

    #[tokio::test]
    async fn a_stream_that_never_sends_a_newline_fails() {
        let endless = "x".repeat(4096);
        assert_eq!(collect(&endless, 512).await, Err(LineError::TooLong));
    }

    #[tokio::test]
    async fn lines_split_across_reads_are_reassembled() {
        // Longer than one chunk, so the reader has to stitch it together.
        let payload = "y".repeat(CHUNK_BYTES * 3);
        let lines = collect(&format!("{payload}\n"), CHUNK_BYTES * 8).await.unwrap();

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].len(), CHUNK_BYTES * 3);
    }

    #[tokio::test]
    async fn invalid_utf8_is_replaced_rather_than_killing_the_connection() {
        let mut raw = b"good\n".to_vec();
        raw.extend_from_slice(&[0xff, 0xfe]);
        raw.push(b'\n');

        let mut lines = BoundedLines::new(Cursor::new(raw), 1024);
        assert_eq!(lines.next_line().await.unwrap().as_deref(), Some("good"));
        let replaced = lines.next_line().await.unwrap().unwrap();
        assert!(replaced.contains('\u{fffd}'));
    }
}
