#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "unwrap/expect are idiomatic in tests"
)]
//! Unit tests for `har::parser`.

use std::path::Path;

use super::{load, ParseError};

/// Write a string to a temp file and return its path.
fn write_temp(content: &str, name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir();
    let path = dir.join(name);
    std::fs::write(&path, content).expect("write temp file");
    path
}

#[test]
fn load_valid_minimal_har() {
    let json = r#"{
        "log": {
            "version": "1.2",
            "creator": {"name": "test", "version": "1.0"},
            "entries": [
                {
                    "startedDateTime": "2024-01-01T00:00:00Z",
                    "time": 100,
                    "request": {
                        "method": "GET",
                        "url": "https://example.com/",
                        "httpVersion": "HTTP/2.0",
                        "headersSize": 200,
                        "bodySize": 0
                    },
                    "response": {
                        "status": 200,
                        "statusText": "OK",
                        "httpVersion": "HTTP/2.0",
                        "content": {"size": 500, "mimeType": "text/html"},
                        "redirectURL": "",
                        "headersSize": 150,
                        "bodySize": 500
                    },
                    "cache": {},
                    "timings": {"send": 1, "wait": 98, "receive": 1}
                }
            ]
        }
    }"#;
    let path = write_temp(json, "test_minimal.har");
    let result = load(&path);
    std::fs::remove_file(&path).ok();
    let har = result.expect("should parse");
    assert_eq!(har.log.entries.len(), 1);
    assert_eq!(har.log.entries[0].request.method, "GET");
}

#[test]
fn load_missing_file() {
    let result = load(Path::new("/tmp/definitely_not_real_42.har"));
    assert!(matches!(result.unwrap_err(), ParseError::FileNotFound(_)));
}

#[test]
fn load_invalid_json() {
    let path = write_temp("not json at all", "test_bad.har");
    let result = load(&path);
    std::fs::remove_file(&path).ok();
    assert!(matches!(
        result.unwrap_err(),
        ParseError::InvalidJson { .. }
    ));
}

#[test]
fn load_empty_entries_errors() {
    let json = r#"{
        "log": {
            "version": "1.2",
            "creator": {"name": "test", "version": "1.0"},
            "entries": []
        }
    }"#;
    let path = write_temp(json, "test_empty.har");
    let result = load(&path);
    std::fs::remove_file(&path).ok();
    assert!(matches!(result.unwrap_err(), ParseError::NoEntries { .. }));
}
