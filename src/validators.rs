//! Shared clap value validators used across multiple commands.

use std::path::PathBuf;

/// Validates that a path points to an existing file.
///
/// # Errors
///
/// Returns an error message string if the file does not exist.
pub fn existing_file(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);
    if path.exists() {
        Ok(path)
    } else {
        Err(format!("file not found: {s}"))
    }
}

/// Validates that a path points to an existing directory.
///
/// # Errors
///
/// Returns an error message string if the directory does not exist.
pub fn existing_dir(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);
    if path.is_dir() {
        Ok(path)
    } else {
        Err(format!("directory not found: {s}"))
    }
}

/// Validates that a string is a well-formed URL.
///
/// # Errors
///
/// Returns an error message string if the URL cannot be parsed.
pub fn valid_url(s: &str) -> Result<String, String> {
    url::Url::parse(s)
        .map(|_| s.to_owned())
        .map_err(|e| format!("invalid URL: {e}"))
}
