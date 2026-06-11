//! `harvey endpoints` — deduplicated API endpoint summary.

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
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

/// Show a deduplicated summary of API endpoints in a HAR file.
///
/// Groups requests by method + path (stripping query strings),
/// counts occurrences, and sorts by frequency.
#[derive(Debug, Args)]
pub struct EndpointsArgs {
    /// Path to the .har file.
    #[arg(value_name = "FILE", value_hint = clap::ValueHint::FilePath, value_parser = crate::validators::existing_file)]
    pub file: PathBuf,

    /// Filter to endpoints from a specific domain.
    #[arg(long, value_name = "DOMAIN")]
    pub filter_domain: Option<String>,
}

/// A single endpoint summary for JSON output.
#[derive(Debug, Serialize)]
struct EndpointSummary {
    method: String,
    path: String,
    count: usize,
    domains: Vec<String>,
    status_codes: BTreeMap<u16, usize>,
}

/// Run the endpoints command.
///
/// # Errors
///
/// Returns an error if the HAR file cannot be loaded.
pub fn run(args: &EndpointsArgs, global: &GlobalArgs) -> Result<()> {
    let mode = OutputMode::from_args(global.json);
    let har = parser::load(&args.file)?;

    let entries: Vec<&Entry> = if let Some(ref domain) = args.filter_domain {
        har.log
            .entries
            .iter()
            .filter(|e| stats::extract_domain(e).eq_ignore_ascii_case(domain))
            .collect()
    } else {
        har.log.entries.iter().collect()
    };

    let summaries = build_endpoints(&entries);

    if matches!(mode, OutputMode::Human) {
        let total = entries.len();
        tracing::info!(
            "{count} unique endpoints from {total} requests",
            count = summaries.len()
        );
    }

    match mode {
        OutputMode::Human => render_human(&summaries),
        OutputMode::Json => render_json(&summaries),
    }
}

/// Group entries by method + path and compute per-endpoint stats.
fn build_endpoints(entries: &[&Entry]) -> Vec<EndpointSummary> {
    let mut groups: BTreeMap<(String, String), Vec<&Entry>> = BTreeMap::new();

    for entry in entries {
        let path = strip_query(&entry.request.url);
        let key = (entry.request.method.clone(), path);
        groups.entry(key).or_default().push(entry);
    }

    let mut summaries: Vec<EndpointSummary> = groups
        .into_iter()
        .map(|((method, path), entries)| {
            let count = entries.len();

            let mut domains: Vec<String> =
                entries.iter().map(|e| stats::extract_domain(e)).collect();
            domains.sort();
            domains.dedup();

            let mut status_codes: BTreeMap<u16, usize> = BTreeMap::new();
            for entry in &entries {
                *status_codes.entry(entry.response.status).or_insert(0) += 1;
            }

            EndpointSummary {
                method,
                path,
                count,
                domains,
                status_codes,
            }
        })
        .collect();

    summaries.sort_by_key(|s| std::cmp::Reverse(s.count));
    summaries
}

/// Strip the query string from a URL, returning just the path.
fn strip_query(url: &str) -> String {
    url::Url::parse(url)
        .map_or_else(|_| url.to_owned(), |u| u.path().to_owned())
}

// ---------------------------------------------------------------------------
// Human rendering
// ---------------------------------------------------------------------------

/// Render endpoint summaries as a table.
fn render_human(summaries: &[EndpointSummary]) -> Result<()> {
    let mut builder = Builder::default();
    builder.push_record(["Method", "Endpoint", "Count"]);

    for s in summaries {
        builder.push_record([
            s.method.clone(),
            s.path.clone(),
            s.count.to_string(),
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

// ---------------------------------------------------------------------------
// JSON rendering
// ---------------------------------------------------------------------------

/// Render endpoint summaries as a JSON array.
fn render_json(summaries: &[EndpointSummary]) -> Result<()> {
    let json = serde_json::to_string_pretty(summaries)
        .context("failed to serialize endpoints")?;
    let mut stdout = std::io::stdout();
    std::io::Write::write_all(&mut stdout, json.as_bytes())
        .context("failed to write JSON to stdout")?;
    std::io::Write::write_all(&mut stdout, b"\n")
        .context("failed to write newline")?;
    Ok(())
}
