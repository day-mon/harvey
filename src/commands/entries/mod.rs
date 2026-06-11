//! `harvey entries` — list and filter individual HAR entries.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Args, ValueEnum};
use serde::Serialize;
use tabled::{
    builder::Builder,
    settings::{object::Rows, Color, Modify, Style},
};

use crate::cli::GlobalArgs;
use crate::har::filter::{self, EntryPredicate};
use crate::har::parser;
use crate::har::stats;
use crate::har::types::Entry;
use crate::output::OutputMode;

/// List and filter individual HTTP request/response entries from a HAR file.
///
/// Supports filtering by URL pattern, status, method, MIME type, and domain.
/// Outputs as a human-readable table or JSONL (one JSON object per line).
#[derive(Debug, Args)]
pub struct EntriesArgs {
    /// Path to the .har file.
    #[arg(value_name = "FILE", value_hint = clap::ValueHint::FilePath, value_parser = crate::validators::existing_file)]
    pub file: PathBuf,

    /// Filter entries whose URL matches this regex pattern.
    #[arg(
        long,
        value_name = "REGEX",
        allow_hyphen_values = true
    )]
    pub filter_url: Option<String>,

    /// Filter by exact HTTP status code (e.g. 200, 404, 500).
    #[arg(long, value_name = "CODE")]
    pub filter_status: Option<u16>,

    /// Filter by HTTP method: GET, POST, PUT, DELETE, etc.
    #[arg(long, value_name = "METHOD")]
    pub filter_method: Option<String>,

    /// Filter by response MIME type (e.g. application/json).
    #[arg(long, value_name = "TYPE")]
    pub filter_mime: Option<String>,

    /// Filter by domain (exact host match).
    #[arg(long, value_name = "DOMAIN")]
    pub filter_domain: Option<String>,

    /// Sort results by this field.
    #[arg(long, value_enum, default_value_t = SortBy::Time)]
    pub sort_by: SortBy,

    /// Sort direction.
    #[arg(long, value_enum, default_value_t = SortDir::Desc)]
    pub sort_dir: SortDir,

    /// Maximum number of entries to display (default: 100).
    #[arg(long, value_name = "N", default_value_t = 100)]
    pub limit: usize,

    /// Include response body text in JSON/JSONL output.
    /// By default, body text is excluded to keep output compact.
    /// When set, `response.content.text` is included (null if absent).
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub include_body: bool,
}

/// Fields available for sorting entries.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SortBy {
    /// Sort by total elapsed time.
    Time,
    /// Sort by total transfer size.
    Size,
    /// Sort by HTTP status code.
    Status,
    /// Sort by request URL.
    Url,
}

/// Sort direction.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SortDir {
    /// Ascending order.
    Asc,
    /// Descending order.
    Desc,
}

/// JSONL output wrapper for a single entry with computed fields.
#[derive(Debug, Serialize)]
struct EntryOutput<'a> {
    /// ISO 8601 start timestamp.
    #[serde(rename = "startedDateTime")]
    started_date_time: &'a str,
    /// Total elapsed time in ms.
    time: f64,
    /// The HTTP request.
    request: &'a crate::har::types::Request,
    /// The HTTP response (content.text conditionally included).
    response: ResponseOutput<'a>,
    /// Cache state.
    cache: &'a crate::har::types::Cache,
    /// Timing breakdown.
    timings: &'a crate::har::types::Timings,
    /// Computed fields (not in the HAR spec).
    #[serde(rename = "_computed")]
    computed: ComputedFields,
}

/// Response wrapper for entries output — controls body inclusion.
#[derive(Debug, Serialize)]
struct ResponseOutput<'a> {
    status: u16,
    #[serde(rename = "statusText")]
    status_text: &'a str,
    #[serde(rename = "httpVersion")]
    http_version: &'a str,
    cookies: &'a [crate::har::types::Cookie],
    headers: &'a [crate::har::types::Header],
    content: ContentOutput<'a>,
    #[serde(rename = "redirectURL")]
    redirect_url: &'a str,
    #[serde(rename = "headersSize")]
    headers_size: i64,
    #[serde(rename = "bodySize")]
    body_size: i64,
}

/// Content wrapper — text only included when present and requested.
#[derive(Debug, Serialize)]
struct ContentOutput<'a> {
    size: u64,
    #[serde(rename = "mimeType")]
    mime_type: &'a str,
    /// Only included when --include-body is set.
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<&'a str>,
}

/// Fields computed by harvey, namespaced under `_computed`.
#[derive(Debug, Serialize)]
struct ComputedFields {
    /// Total transfer size (request + response headers + bodies).
    total_bytes: u64,
    /// Domain extracted from the request URL.
    domain: String,
    /// Response content type shortcut.
    content_type: String,
}

/// Run the entries command.
///
/// # Errors
///
/// Returns an error if the HAR file cannot be loaded, or if a filter
/// regex is invalid.
pub fn run(args: &EntriesArgs, global: &GlobalArgs) -> Result<()> {
    let mode = OutputMode::from_args(global.json);
    let har = parser::load(&args.file)?;

    let mut predicate = EntryPredicate::new();
    if let Some(ref pattern) = args.filter_url {
        predicate = predicate
            .with_url_pattern(pattern)
            .context("invalid --filter-url regex")?;
    }
    if let Some(status) = args.filter_status {
        predicate = predicate.with_status(status);
    }
    if let Some(ref method) = args.filter_method {
        predicate = predicate.with_method(method);
    }
    if let Some(ref mime) = args.filter_mime {
        predicate = predicate.with_mime_type(mime);
    }
    if let Some(ref domain) = args.filter_domain {
        predicate = predicate.with_domain(domain);
    }

    let mut matched: Vec<&Entry> =
        filter::filter_entries(&har.log.entries, &predicate);

    if matched.is_empty() {
        tracing::warn!("no entries matched the given filters");
        return Err(anyhow::anyhow!("NO_RESULTS"));
    }

    sort_entries(&mut matched, args.sort_by, args.sort_dir);
    matched.truncate(args.limit);

    if matches!(mode, OutputMode::Human) {
        tracing::info!(
            "showing {} of {} matching entries (total: {})",
            matched.len(),
            matched.len(),
            har.log.entries.len()
        );
    }

    match mode {
        OutputMode::Human => render_human(&matched),
        OutputMode::Json => render_jsonl(&matched, args.include_body),
    }
}

/// Sort a mutable slice of entry references.
fn sort_entries(entries: &mut [&Entry], by: SortBy, dir: SortDir) {
    match by {
        SortBy::Time => {
            entries.sort_unstable_by(|a, b| {
                a.time
                    .partial_cmp(&b.time)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        SortBy::Size => {
            entries.sort_unstable_by_key(|e| stats::response_size(e));
        }
        SortBy::Status => {
            entries.sort_unstable_by_key(|e| e.response.status);
        }
        SortBy::Url => {
            entries.sort_unstable_by_key(|e| &e.request.url);
        }
    }

    if matches!(dir, SortDir::Desc) {
        entries.reverse();
    }
}

/// Render entries as a human-readable table.
fn render_human(entries: &[&Entry]) -> Result<()> {
    let mut builder = Builder::default();
    builder.push_record([
        "#",
        "Method",
        "Status",
        "Domain",
        "Path",
        "Time",
        "Size",
        "Content-Type",
    ]);

    for (i, e) in entries.iter().enumerate() {
        let domain = stats::extract_domain(e);
        let path = extract_path(&e.request.url);
        builder.push_record([
            (i + 1).to_string(),
            e.request.method.clone(),
            format_status(e.response.status),
            domain,
            path,
            format!("{:.0}ms", e.time),
            format_size(stats::response_size(e)),
            e.response.content.mime_type.clone(),
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

/// Render entries as JSONL (one JSON object per line).
fn render_jsonl(entries: &[&Entry], include_body: bool) -> Result<()> {
    let mut stdout = std::io::BufWriter::new(std::io::stdout());

    for entry in entries {
        let body_size = stats::response_size(entry);
        let text = if include_body {
            entry.response.content.text.as_deref()
        } else {
            None
        };

        if include_body {
            if let Some(t) = &entry.response.content.text {
                if t.len() > 1_000_000 {
                    tracing::warn!(
                        "large body: {:.1}MB for {}",
                        t.len() as f64 / 1_048_576.0,
                        entry.request.url
                    );
                }
            }
        }

        let output = EntryOutput {
            started_date_time: &entry.started_date_time,
            time: entry.time,
            request: &entry.request,
            response: ResponseOutput {
                status: entry.response.status,
                status_text: &entry.response.status_text,
                http_version: &entry.response.http_version,
                cookies: &entry.response.cookies,
                headers: &entry.response.headers,
                content: ContentOutput {
                    size: entry.response.content.size,
                    mime_type: &entry.response.content.mime_type,
                    text,
                },
                redirect_url: &entry.response.redirect_url,
                headers_size: entry.response.headers_size,
                body_size: entry.response.body_size,
            },
            cache: &entry.cache,
            timings: &entry.timings,
            computed: ComputedFields {
                total_bytes: body_size,
                domain: stats::extract_domain(entry),
                content_type: entry.response.content.mime_type.clone(),
            },
        };

        let line = serde_json::to_string(&output)
            .context("failed to serialize entry to JSON")?;
        std::io::Write::write_all(&mut stdout, line.as_bytes())
            .context("failed to write entry to stdout")?;
        std::io::Write::write_all(&mut stdout, b"\n")
            .context("failed to write newline")?;
    }

    Ok(())
}

/// Extract the path portion of a URL.
fn extract_path(url: &str) -> String {
    url::Url::parse(url).map_or_else(
        |_| url.to_owned(),
        |u| {
            let path = u.path().to_owned();
            if let Some(query) = u.query() {
                format!("{path}?{query}")
            } else {
                path
            }
        },
    )
}

/// Format a status code with color-coded category indicator.
fn format_status(code: u16) -> String {
    let category = match code {
        200..=299 => "✓",
        300..=399 => "→",
        400..=499 => "✗",
        _ => "!",
    };
    format!("{category} {code}")
}

/// Format a byte count as a compact human-readable string.
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;

    if bytes >= MB {
        format!("{:.1}M", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}K", bytes as f64 / KB as f64)
    } else {
        format!("{bytes}B")
    }
}
