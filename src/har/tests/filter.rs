//! Unit tests for `har::filter`.

use super::EntryPredicate;
use crate::har::types::{Cache, Content, Entry, Request, Response, Timings};

/// Build a minimal `Entry` for filter testing.
fn make_entry(status: u16, method: &str, mime: &str, url: &str) -> Entry {
    Entry {
        started_date_time: "2024-01-01T00:00:00Z".into(),
        time: 100.0,
        request: Request {
            method: method.into(),
            url: url.into(),
            http_version: "HTTP/2.0".into(),
            cookies: vec![],
            headers: vec![],
            query_string: vec![],
            post_data: None,
            headers_size: 0,
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
                size: 100,
                compression: None,
                mime_type: mime.into(),
                text: None,
                encoding: None,
                comment: None,
            },
            redirect_url: String::new(),
            headers_size: 0,
            body_size: 100,
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
            wait: 98.0,
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

#[test]
fn empty_predicate_matches_all() {
    let e = make_entry(200, "GET", "text/html", "https://example.com/");
    let pred = EntryPredicate::new();
    assert!(pred.matches(&e));
}

#[test]
fn filter_by_status_match() {
    let e = make_entry(404, "GET", "text/html", "https://example.com/");
    let pred = EntryPredicate::new().with_status(404);
    assert!(pred.matches(&e));
}

#[test]
fn filter_by_status_no_match() {
    let e = make_entry(200, "GET", "text/html", "https://example.com/");
    let pred = EntryPredicate::new().with_status(404);
    assert!(!pred.matches(&e));
}

#[test]
fn filter_by_method_case_insensitive() {
    let e = make_entry(200, "POST", "application/json", "https://example.com/");
    let pred = EntryPredicate::new().with_method("post");
    assert!(pred.matches(&e));
}

#[test]
fn filter_by_mime_partial_match_fails() {
    let e = make_entry(200, "GET", "application/json", "https://example.com/");
    let pred = EntryPredicate::new().with_mime_type("json");
    assert!(!pred.matches(&e));
}

#[test]
fn filter_by_url_regex_match() {
    let e =
        make_entry(200, "GET", "text/html", "https://api.example.com/v1/users");
    let pred = EntryPredicate::new()
        .with_url_pattern(r"/v1/")
        .expect("valid regex");
    assert!(pred.matches(&e));
}

#[test]
fn filter_combined_and_logic() {
    let e = make_entry(
        200,
        "GET",
        "application/json",
        "https://api.example.com/v1/data",
    );
    let pred = EntryPredicate::new()
        .with_status(200)
        .with_method("GET")
        .with_mime_type("application/json");
    assert!(pred.matches(&e));
}

#[test]
fn filter_combined_one_mismatch_fails() {
    let e = make_entry(
        500,
        "GET",
        "application/json",
        "https://api.example.com/v1/data",
    );
    let pred = EntryPredicate::new()
        .with_status(200)
        .with_method("GET")
        .with_mime_type("application/json");
    assert!(!pred.matches(&e));
}

#[test]
fn filter_bad_regex_returns_err() {
    let result = EntryPredicate::new().with_url_pattern("[invalid");
    assert!(result.is_err());
}
