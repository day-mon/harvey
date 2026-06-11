//! `harvey domains` — per-domain breakdown of a HAR capture.

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Args, ValueEnum};
use serde::Serialize;
use tabled::{
    builder::Builder,
    settings::{object::Rows, Color, Modify, Style},
};

use crate::cli::GlobalArgs;
use crate::har::parser;
use crate::har::stats;
use crate::har::types::Entry;
use crate::output::OutputMode;

/// Show a per-domain breakdown of all requests in a HAR file.
///
/// Useful for auditing third-party services and understanding where
/// traffic is going.
#[derive(Debug, Args)]
pub struct DomainsArgs {
    /// Path to the .har file.
    #[arg(value_name = "FILE", value_hint = clap::ValueHint::FilePath, value_parser = crate::validators::existing_file)]
    pub file: PathBuf,

    /// Sort domains by this metric.
    #[arg(long, value_enum, default_value_t = DomainSortBy::Requests)]
    pub sort_by: DomainSortBy,
}

/// Fields available for sorting domain output.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum DomainSortBy {
    /// Sort by number of requests.
    Requests,
    /// Sort by total bytes transferred.
    Bytes,
    /// Sort by average request time.
    #[value(name = "avg-time")]
    AvgTime,
}

/// JSON output wrapper for the domains command.
#[derive(Debug, Serialize)]
struct DomainsOutput {
    /// Schema version identifier.
    format_version: String,
    /// Per-domain summaries, sorted.
    domains: Vec<DomainSummary>,
}

/// Aggregated stats for a single domain.
#[derive(Debug, Clone, Serialize)]
struct DomainSummary {
    /// The domain name.
    domain: String,
    /// Number of requests to this domain.
    request_count: usize,
    /// Total bytes transferred (requests + responses).
    total_bytes: u64,
    /// Average request time in milliseconds.
    avg_time_ms: f64,
    /// HTTP status code distribution: code → count.
    status_summary: BTreeMap<u16, usize>,
}

/// Run the domains command.
///
/// # Errors
///
/// Returns an error if the HAR file cannot be loaded.
pub fn run(args: &DomainsArgs, global: &GlobalArgs) -> Result<()> {
    let mode = OutputMode::from_args(global.json);
    let har = parser::load(&args.file)?;
    let summaries = build_summaries(&har.log.entries, args.sort_by);

    match mode {
        OutputMode::Human => render_human(&summaries),
        OutputMode::Json => render_json(&summaries),
    }
}

/// Group entries by domain and compute per-domain statistics.
fn build_summaries(
    entries: &[Entry],
    sort_by: DomainSortBy,
) -> Vec<DomainSummary> {
    let mut domains: BTreeMap<String, Vec<&Entry>> = BTreeMap::new();

    for entry in entries {
        let domain = stats::extract_domain(entry);
        domains.entry(domain).or_default().push(entry);
    }

    let mut summaries: Vec<DomainSummary> = domains
        .into_iter()
        .map(|(domain, entries)| {
            let request_count = entries.len();
            let total_bytes: u64 =
                entries.iter().map(|e| stats::response_size(e)).sum();
            let total_time: f64 = entries.iter().map(|e| e.time).sum();
            let avg_time_ms = if request_count > 0 {
                total_time / request_count as f64
            } else {
                0.0
            };

            let mut status_summary: BTreeMap<u16, usize> = BTreeMap::new();
            for entry in &entries {
                *status_summary.entry(entry.response.status).or_insert(0) += 1;
            }

            DomainSummary {
                domain,
                request_count,
                total_bytes,
                avg_time_ms,
                status_summary,
            }
        })
        .collect();

    match sort_by {
        DomainSortBy::Requests => {
            summaries.sort_by_key(|b| std::cmp::Reverse(b.request_count));
        }
        DomainSortBy::Bytes => {
            summaries.sort_by_key(|b| std::cmp::Reverse(b.total_bytes));
        }
        DomainSortBy::AvgTime => {
            summaries
                .sort_by(|a, b| f64::total_cmp(&b.avg_time_ms, &a.avg_time_ms));
        }
    }

    summaries
}

/// Render domain summaries as a human-readable table.
fn render_human(summaries: &[DomainSummary]) -> Result<()> {
    let mut builder = Builder::default();
    builder.push_record([
        "Domain",
        "Requests",
        "Bytes",
        "Avg Time",
        "Status Codes",
    ]);

    for s in summaries {
        let status_str: Vec<String> = s
            .status_summary
            .iter()
            .map(|(code, count)| format!("{code}×{count}"))
            .collect();
        builder.push_record([
            s.domain.clone(),
            s.request_count.to_string(),
            format_size(s.total_bytes),
            format!("{:.0}ms", s.avg_time_ms),
            status_str.join(" "),
        ]);
    }

    let mut table = builder.build();
    table
        .with(Style::rounded())
        .with(Modify::new(Rows::first()).with(Color::BOLD));

    let mut stdout = std::io::stdout();
    std::io::Write::write_all(&mut stdout, table.to_string().as_bytes())
        .context("failed to write table to stdout")?;
    std::io::Write::write_all(&mut stdout, b"\n")
        .context("failed to write newline")?;
    Ok(())
}

/// Render domain summaries as JSON.
fn render_json(summaries: &[DomainSummary]) -> Result<()> {
    let output = DomainsOutput {
        format_version: "1.0".into(),
        domains: summaries.to_vec(),
    };

    let json = serde_json::to_string_pretty(&output)
        .context("failed to serialize domains output")?;
    let mut stdout = std::io::stdout();
    std::io::Write::write_all(&mut stdout, json.as_bytes())
        .context("failed to write JSON to stdout")?;
    std::io::Write::write_all(&mut stdout, b"\n")
        .context("failed to write newline")?;
    Ok(())
}

/// Format a byte count as a human-readable string.
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;

    if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}
