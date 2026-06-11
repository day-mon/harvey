//! HAR file loading and validation.

use std::path::Path;

use crate::har::types::Har;

/// Errors that can occur when loading a HAR file.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    /// The file does not exist or cannot be read.
    #[error("file not found: {0}")]
    FileNotFound(String),

    /// The file could not be read (I/O error).
    #[error("failed to read file {path}: {source}")]
    Io {
        /// The file path.
        path: String,
        /// The underlying I/O error.
        source: std::io::Error,
    },

    /// The file is not valid JSON.
    #[error("invalid JSON in {path}: {source}")]
    InvalidJson {
        /// The file path.
        path: String,
        /// The underlying serde error.
        source: serde_json::Error,
    },

    /// The JSON is valid but doesn't match the HAR schema.
    #[error("invalid HAR structure in {path}: missing 'log' field")]
    MissingLog {
        /// The file path.
        path: String,
    },

    /// The log has no entries.
    #[error("HAR file {path} contains no entries")]
    NoEntries {
        /// The file path.
        path: String,
    },
}

/// Load and parse a HAR file from the given path.
///
/// # Errors
///
/// Returns a [`ParseError`] if the file cannot be found, read, or parsed as
/// valid HAR JSON.
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
/// use harvey::har::parser::load;
///
/// let har = load(Path::new("capture.har"))?;
/// # Ok::<(), harvey::har::parser::ParseError>(())
/// ```
pub fn load(path: &Path) -> Result<Har, ParseError> {
    if !path.exists() {
        return Err(ParseError::FileNotFound(path.display().to_string()));
    }

    let raw = std::fs::read_to_string(path).map_err(|e| ParseError::Io {
        path: path.display().to_string(),
        source: e,
    })?;

    let har: Har =
        serde_json::from_str(&raw).map_err(|e| ParseError::InvalidJson {
            path: path.display().to_string(),
            source: e,
        })?;

    if har.log.entries.is_empty() {
        return Err(ParseError::NoEntries {
            path: path.display().to_string(),
        });
    }

    Ok(har)
}

#[cfg(test)]
#[path = "tests/parser.rs"]
mod tests;
