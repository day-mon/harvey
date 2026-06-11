//! Unit tests for `har::stats`.

use super::{compute, extract_domain, percentile, response_size};
use crate::har::types::{
    Cache, Content, Creator, Entry, Log, Request, Response, Timings,
};

/// Build a minimal `Entry` for stats testing.
fn make_entry(
    status: u16,
    size: u64,
    mime: &str,
    time: f64,
    method: &str,
    url: &str,
) -> Entry {
    Entry {
        started_date_time: "2024-01-01T00:00:00Z".into(),
        time,
        request: Request {
            method: method.into(),
            url: url.into(),
            http_version: "HTTP/2.0".into(),
            cookies: vec![],
            headers: vec![],
            query_string: vec![],
            post_data: None,
            headers_size: 100,
            body_size: 0,
            comment: None,
        },
        response: Response {
            status,
            status_text: "OK".into(),
            http_version: "HTTP/2.0".into(),
            cookies: vec![],
            headers: vec![],
            content: Content {
                size,
                compression: None,
                mime_type: mime.into(),
                text: None,
                encoding: None,
                comment: None,
            },
            redirect_url: String::new(),
            headers_size: 200,
            body_size: size as i64,
            comment: None,
        },
        cache: Cache {
            before_request: None,
            after_request: None,
            comment: None,
        },
        timings: Timings {
            blocked: None,
            dns: None,
            connect: None,
            send: 1.0,
            wait: time - 2.0,
            receive: 1.0,
            ssl: None,
            comment: None,
        },
        server_ip_address: None,
        connection: None,
        pageref: None,
        comment: None,
    }
}

/// Build a `Log` from a list of entries.
fn make_log(entries: Vec<Entry>) -> Log {
    Log {
        version: "1.2".into(),
        creator: Creator {
            name: "test".into(),
            version: "1.0".into(),
            comment: None,
        },
        browser: None,
        pages: None,
        entries,
        comment: None,
    }
}

#[test]
fn compute_empty_log() {
    let log = make_log(vec![]);
    let stats = compute(&log);
    assert_eq!(stats.total_entries, 0);
    assert_eq!(stats.total_bytes, 0);
}

#[test]
fn compute_single_entry() {
    let log = make_log(vec![make_entry(
        200,
        500,
        "text/html",
        100.0,
        "GET",
        "https://example.com/",
    )]);
    let stats = compute(&log);
    assert_eq!(stats.total_entries, 1);
    // req: 100h + 0b, resp: 200h + 500b
    assert_eq!(response_size(&log.entries[0]), 800);
    assert_eq!(stats.status_distribution.get(&200), Some(&1));
}

#[test]
fn compute_status_distribution() {
    let entries = vec![
        make_entry(200, 100, "text/html", 50.0, "GET", "https://a.com/"),
        make_entry(200, 200, "text/html", 60.0, "GET", "https://a.com/"),
        make_entry(
            404,
            50,
            "application/json",
            30.0,
            "GET",
            "https://a.com/api",
        ),
    ];
    let log = make_log(entries);
    let stats = compute(&log);
    assert_eq!(stats.status_distribution.get(&200), Some(&2));
    assert_eq!(stats.status_distribution.get(&404), Some(&1));
}

#[test]
fn compute_percentile_ordering() {
    let entries = vec![
        make_entry(200, 100, "text/html", 10.0, "GET", "https://a.com/"),
        make_entry(200, 100, "text/html", 50.0, "GET", "https://a.com/"),
        make_entry(200, 100, "text/html", 90.0, "GET", "https://a.com/"),
    ];
    let log = make_log(entries);
    let stats = compute(&log);
    assert!(stats.min_time_ms <= stats.p50_time_ms);
    assert!(stats.p50_time_ms <= stats.p95_time_ms);
    assert!(stats.p95_time_ms <= stats.max_time_ms);
}

#[test]
fn response_size_with_negative_headers_size() {
    // HAR uses -1 as "unknown" sentinel — should fall back to content.size
    let mut entry =
        make_entry(200, 500, "text/html", 100.0, "GET", "https://example.com/");
    entry.response.headers_size = -1;
    entry.response.body_size = -1;
    let size = response_size(&entry);
    // Only content.size (500) + req headers (100) = 600
    assert_eq!(size, 600);
}

#[test]
fn extract_domain_from_url() {
    let entry = make_entry(
        200,
        100,
        "text/html",
        50.0,
        "GET",
        "https://api.example.com/v1/users?page=1",
    );
    assert_eq!(extract_domain(&entry), "api.example.com");
}

#[test]
fn extract_domain_from_bad_url() {
    let entry = make_entry(200, 100, "text/html", 50.0, "GET", "not-a-url");
    assert_eq!(extract_domain(&entry), "unknown");
}

#[test]
fn percentile_exact_values() {
    let sorted = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    assert!((percentile(&sorted, 50.0) - 3.0).abs() < 0.01);
    assert!((percentile(&sorted, 0.0) - 1.0).abs() < 0.01);
    assert!((percentile(&sorted, 100.0) - 5.0).abs() < 0.01);
}

#[test]
fn percentile_interpolation() {
    let sorted = vec![1.0, 3.0];
    // index = (25/100) * 1 = 0.25 → between sorted[0]=1.0 and sorted[1]=3.0
    // weight = 0.25, so 1.0 * 0.75 + 3.0 * 0.25 = 1.5
    assert!((percentile(&sorted, 25.0) - 1.5).abs() < 0.01);
}

#[test]
fn percentile_empty_slice() {
    let empty: [f64; 0] = [];
    assert_eq!(percentile(&empty, 50.0), 0.0);
}
