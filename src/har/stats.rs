//! Aggregate statistics for HAR files.

use std::collections::{BTreeMap, HashSet};

use serde::Serialize;

use crate::har::types::{Entry, Log};

/// Computed statistics for an entire HAR log.
#[derive(Debug, Clone, Serialize)]
pub struct Stats {
    /// Total number of entries.
    pub total_entries: usize,
    /// Sum of all response content sizes in bytes.
    pub total_bytes: u64,
    /// Average request time in milliseconds.
    pub avg_time_ms: f64,
    /// Median (50th percentile) request time in milliseconds.
    pub p50_time_ms: f64,
    /// 95th percentile request time in milliseconds.
    pub p95_time_ms: f64,
    /// 99th percentile request time in milliseconds.
    pub p99_time_ms: f64,
    /// Fastest request time in milliseconds.
    pub min_time_ms: f64,
    /// Slowest request time in milliseconds.
    pub max_time_ms: f64,
    /// Distribution of HTTP status codes: code → count.
    pub status_distribution: BTreeMap<u16, usize>,
    /// Distribution of MIME types: type → count.
    pub content_type_distribution: BTreeMap<String, usize>,
    /// Distribution of HTTP methods: method → count.
    pub method_distribution: BTreeMap<String, usize>,
    /// Number of unique domains contacted.
    pub unique_domains: usize,
}

/// Compute aggregate statistics from a HAR log.
///
/// # Examples
///
/// ```
/// # use harvey::har::stats::compute;
/// # use harvey::har::types::{Har, Log, Creator, Entry, Request, Response, Content, Cache, Timings};
/// let log = Log {
///     version: "1.2".into(),
///     creator: Creator { name: "test".into(), version: "1.0".into(), comment: None },
///     browser: None,
///     pages: None,
///     entries: vec![],
///     comment: None,
/// };
/// let stats = compute(&log);
/// assert_eq!(stats.total_entries, 0);
/// ```
#[must_use]
pub fn compute(log: &Log) -> Stats {
    let total_entries = log.entries.len();

    if total_entries == 0 {
        return Stats {
            total_entries: 0,
            total_bytes: 0,
            avg_time_ms: 0.0,
            p50_time_ms: 0.0,
            p95_time_ms: 0.0,
            p99_time_ms: 0.0,
            min_time_ms: 0.0,
            max_time_ms: 0.0,
            status_distribution: BTreeMap::new(),
            content_type_distribution: BTreeMap::new(),
            method_distribution: BTreeMap::new(),
            unique_domains: 0,
        };
    }

    let mut times: Vec<f64> = log.entries.iter().map(|e| e.time).collect();
    times.sort_unstable_by(|a, b| {
        a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
    });

    let total_bytes: u64 = log.entries.iter().map(response_size).sum();

    let sum_time: f64 = times.iter().sum();
    let avg_time_ms = sum_time / total_entries as f64;
    let min_time_ms = times.first().copied().unwrap_or(0.0);
    let max_time_ms = times.last().copied().unwrap_or(0.0);
    let p50_time_ms = percentile(&times, 50.0);
    let p95_time_ms = percentile(&times, 95.0);
    let p99_time_ms = percentile(&times, 99.0);

    let mut status_distribution: BTreeMap<u16, usize> = BTreeMap::new();
    let mut content_type_distribution: BTreeMap<String, usize> =
        BTreeMap::new();
    let mut method_distribution: BTreeMap<String, usize> = BTreeMap::new();
    let mut domains: HashSet<String> = HashSet::new();

    for entry in &log.entries {
        *status_distribution
            .entry(entry.response.status)
            .or_insert(0) += 1;
        *content_type_distribution
            .entry(entry.response.content.mime_type.clone())
            .or_insert(0) += 1;
        *method_distribution
            .entry(entry.request.method.clone())
            .or_insert(0) += 1;
        domains.insert(extract_domain(entry));
    }

    let unique_domains = domains.len();

    Stats {
        total_entries,
        total_bytes,
        avg_time_ms,
        p50_time_ms,
        p95_time_ms,
        p99_time_ms,
        min_time_ms,
        max_time_ms,
        status_distribution,
        content_type_distribution,
        method_distribution,
        unique_domains,
    }
}

/// Compute total transfer size (content + headers) for an entry.
///
/// Uses `content.size` as the primary size source. Falls back to
/// `bodySize` and `headersSize` fields only if they are non-negative
/// (HAR uses -1 as an "unknown" sentinel).
#[must_use]
pub const fn response_size(entry: &Entry) -> u64 {
    let body: u64 = if entry.response.body_size >= 0 {
        entry.response.body_size as u64
    } else {
        entry.response.content.size
    };
    let headers: u64 = if entry.response.headers_size >= 0 {
        entry.response.headers_size as u64
    } else {
        0
    };
    let req_headers: u64 = if entry.request.headers_size >= 0 {
        entry.request.headers_size as u64
    } else {
        0
    };
    let req_body: u64 = if entry.request.body_size >= 0 {
        entry.request.body_size as u64
    } else {
        0
    };

    body + headers + req_headers + req_body
}

/// Extract the domain from an entry's request URL.
///
/// Returns the host portion of the URL, or `"unknown"` if the URL
/// cannot be parsed.
#[must_use]
pub fn extract_domain(entry: &Entry) -> String {
    url::Url::parse(&entry.request.url)
        .ok()
        .and_then(|u| u.host_str().map(std::string::ToString::to_string))
        .unwrap_or_else(|| "unknown".to_owned())
}

/// Compute a percentile value from a sorted slice.
///
/// Uses linear interpolation between the two closest data points.
#[must_use]
fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }

    let n = sorted.len() as f64;
    let index = (p / 100.0) * (n - 1.0);

    let lower = index.floor() as usize;
    let upper = index.ceil() as usize;

    if lower == upper {
        return sorted[lower];
    }

    let weight = index - lower as f64;
    sorted[lower].mul_add(1.0 - weight, sorted[upper] * weight)
}

#[cfg(test)]
#[path = "tests/stats.rs"]
mod tests;
