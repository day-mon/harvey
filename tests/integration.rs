#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "unwrap/expect are idiomatic in integration tests"
)]

//! Integration tests for the `harvey` library.
//!
//! These tests exercise the core HAR parsing, filtering, and statistics
//! modules using the example fixture.

use std::path::Path;

use harvey::har::filter::{self, EntryPredicate};
use harvey::har::parser::{self, ParseError};
use harvey::har::stats;

/// Path to the example HAR fixture.
fn fixture_path() -> &'static Path {
    Path::new("tests/fixtures/example.har")
}

// ---------------------------------------------------------------------------
// Parser tests
// ---------------------------------------------------------------------------

#[test]
fn parse_valid_har() {
    let har = parser::load(fixture_path());
    assert!(har.is_ok());
}

#[test]
fn parse_counts_entries() {
    let har = parser::load(fixture_path()).expect("should parse fixture");
    assert_eq!(har.log.entries.len(), 8);
}

#[test]
fn parse_file_not_found() {
    let result = parser::load(Path::new("nonexistent.har"));
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ParseError::FileNotFound(_)));
}

#[test]
fn parse_invalid_json() {
    // Create a temp file with invalid JSON.
    let path = Path::new("tests/fixtures/invalid.har");
    std::fs::write(path, "not json").expect("write temp file");
    let result = parser::load(path);
    std::fs::remove_file(path).ok();
    assert!(matches!(
        result.unwrap_err(),
        ParseError::InvalidJson { .. }
    ));
}

// ---------------------------------------------------------------------------
// Stats tests
// ---------------------------------------------------------------------------

#[test]
fn stats_total_entries() {
    let har = parser::load(fixture_path()).expect("should parse fixture");
    let stats = stats::compute(&har.log);
    assert_eq!(stats.total_entries, 8);
}

#[test]
fn stats_total_bytes_is_positive() {
    let har = parser::load(fixture_path()).expect("should parse fixture");
    let stats = stats::compute(&har.log);
    assert!(stats.total_bytes > 0);
}

#[test]
fn stats_status_distribution() {
    let har = parser::load(fixture_path()).expect("should parse fixture");
    let stats = stats::compute(&har.log);
    // Fixture has: 200×4, 201×1, 204×1, 404×1, 500×1
    assert_eq!(stats.status_distribution.get(&200), Some(&4));
    assert_eq!(stats.status_distribution.get(&201), Some(&1));
    assert_eq!(stats.status_distribution.get(&204), Some(&1));
    assert_eq!(stats.status_distribution.get(&404), Some(&1));
    assert_eq!(stats.status_distribution.get(&500), Some(&1));
}

#[test]
fn stats_method_distribution() {
    let har = parser::load(fixture_path()).expect("should parse fixture");
    let stats = stats::compute(&har.log);
    assert_eq!(stats.method_distribution.get("GET"), Some(&6));
    assert_eq!(stats.method_distribution.get("POST"), Some(&2));
}

#[test]
fn stats_unique_domains() {
    let har = parser::load(fixture_path()).expect("should parse fixture");
    let stats = stats::compute(&har.log);
    // www.example.com, api.example.com, cdn.example.com, analytics.example.com
    assert_eq!(stats.unique_domains, 4);
}

#[test]
fn stats_percentile_ordering() {
    let har = parser::load(fixture_path()).expect("should parse fixture");
    let stats = stats::compute(&har.log);
    // Percentiles should be ordered
    assert!(stats.min_time_ms <= stats.p50_time_ms);
    assert!(stats.p50_time_ms <= stats.p95_time_ms);
    assert!(stats.p95_time_ms <= stats.p99_time_ms);
    assert!(stats.p99_time_ms <= stats.max_time_ms);
}

#[test]
fn stats_empty_log() {
    let log = harvey::har::types::Log {
        version: "1.2".into(),
        creator: harvey::har::types::Creator {
            name: "test".into(),
            version: "1.0".into(),
            comment: None,
        },
        browser: None,
        pages: None,
        entries: vec![],
        comment: None,
    };
    let stats = stats::compute(&log);
    assert_eq!(stats.total_entries, 0);
    assert_eq!(stats.total_bytes, 0);
}

#[test]
fn response_size_computation() {
    let har = parser::load(fixture_path()).expect("should parse fixture");
    let entry = &har.log.entries[0]; // www.example.com HTML page
    let size = stats::response_size(entry);
    // headersSize: 320 + 280, bodySize: 0 + 12456 = 13056
    assert_eq!(size, 13056);
}

#[test]
fn extract_domain_known() {
    let har = parser::load(fixture_path()).expect("should parse fixture");
    let entry = &har.log.entries[0];
    assert_eq!(stats::extract_domain(entry), "www.example.com");
}

// ---------------------------------------------------------------------------
// Filter tests
// ---------------------------------------------------------------------------

#[test]
fn filter_by_status_500() {
    let har = parser::load(fixture_path()).expect("should parse fixture");
    let predicate = EntryPredicate::new().with_status(500);
    let matched = filter::filter_entries(&har.log.entries, &predicate);
    assert_eq!(matched.len(), 1);
    assert_eq!(matched[0].response.status, 500);
}

#[test]
fn filter_by_status_200() {
    let har = parser::load(fixture_path()).expect("should parse fixture");
    let predicate = EntryPredicate::new().with_status(200);
    let matched = filter::filter_entries(&har.log.entries, &predicate);
    assert_eq!(matched.len(), 4);
}

#[test]
fn filter_by_method_post() {
    let har = parser::load(fixture_path()).expect("should parse fixture");
    let predicate = EntryPredicate::new().with_method("POST");
    let matched = filter::filter_entries(&har.log.entries, &predicate);
    assert_eq!(matched.len(), 2);
}

#[test]
fn filter_by_mime_json() {
    let har = parser::load(fixture_path()).expect("should parse fixture");
    let predicate = EntryPredicate::new().with_mime_type("application/json");
    let matched = filter::filter_entries(&har.log.entries, &predicate);
    assert_eq!(matched.len(), 4);
}

#[test]
fn filter_by_domain_cdn() {
    let har = parser::load(fixture_path()).expect("should parse fixture");
    let predicate = EntryPredicate::new().with_domain("cdn.example.com");
    let matched = filter::filter_entries(&har.log.entries, &predicate);
    assert_eq!(matched.len(), 2);
}

#[test]
fn filter_by_url_regex() {
    let har = parser::load(fixture_path()).expect("should parse fixture");
    let predicate = EntryPredicate::new()
        .with_url_pattern(r"/v1/")
        .expect("valid regex");
    let matched = filter::filter_entries(&har.log.entries, &predicate);
    assert_eq!(matched.len(), 4); // /v1/users, /v1/events, /v1/products/nonexistent, /v1/reports/slow-endpoint
}

#[test]
fn filter_combined_predicates() {
    let har = parser::load(fixture_path()).expect("should parse fixture");
    let predicate = EntryPredicate::new()
        .with_status(200)
        .with_mime_type("application/json");
    let matched = filter::filter_entries(&har.log.entries, &predicate);
    // Only api.example.com/v1/users is both 200 and application/json.
    assert_eq!(matched.len(), 1);
}

#[test]
fn filter_bad_regex() {
    let result = EntryPredicate::new().with_url_pattern("[invalid");
    assert!(result.is_err());
}
