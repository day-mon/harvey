//! `harvey inspect` — detailed view of a single HAR entry.

use std::fmt::Write as FmtWrite;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use regex::Regex;
use serde::Serialize;

use crate::cli::GlobalArgs;
use crate::har::parser;
use crate::har::stats;
use crate::har::types::Entry;
use crate::output::OutputMode;

/// Inspect a single HAR entry in full detail.
///
/// Shows the complete request and response including headers, cookies,
/// query string, post data, timings, and a body preview. Use `--filter-url`
/// to target an entry by URL pattern instead of index.
#[derive(Debug, Args)]
pub struct InspectArgs {
    /// Path to the .har file.
    #[arg(value_name = "FILE", value_hint = clap::ValueHint::FilePath, value_parser = crate::validators::existing_file)]
    pub file: PathBuf,

    /// Entry index to inspect (1-based, from the raw entries array).
    #[arg(long, value_name = "INDEX", default_value_t = 1)]
    pub entry: usize,

    /// Find and inspect the first entry whose URL matches this regex.
    /// Mutually exclusive with --entry (the default index is ignored
    /// when this flag is set).
    #[arg(
        long,
        value_name = "REGEX",
        allow_hyphen_values = true
    )]
    pub filter_url: Option<String>,

    /// When used with --filter-url, output all matching entries as
    /// NDJSON (one JSON object per line) instead of just the first.
    #[arg(long, action = clap::ArgAction::SetTrue, requires = "filter_url")]
    pub all: bool,
}

/// JSON output for the inspect command.
#[derive(Debug, Serialize)]
struct InspectOutput<'a> {
    /// Schema version.
    format_version: &'static str,
    /// The 1-based entry index.
    entry_index: usize,
    /// ISO 8601 start time.
    #[serde(rename = "startedDateTime")]
    started_date_time: &'a str,
    /// Total elapsed time in ms.
    time: f64,
    /// Full request object.
    request: &'a crate::har::types::Request,
    /// Full response object.
    response: &'a crate::har::types::Response,
    /// Timing breakdown.
    timings: &'a crate::har::types::Timings,
    /// Cache state.
    cache: &'a crate::har::types::Cache,
    /// Harvey-computed metadata.
    #[serde(rename = "_computed")]
    computed: InspectComputed,
}

#[derive(Debug, Serialize)]
struct InspectComputed {
    /// Total transfer size in bytes.
    total_bytes: u64,
    /// Domain extracted from URL.
    domain: String,
    /// Shortcut to response content type.
    content_type: String,
    /// The slowest timing phase (e.g. "wait", "receive").
    bottleneck: String,
}

/// Run the inspect command.
///
/// # Errors
///
/// Returns an error if the HAR file cannot be loaded, the entry
/// index is out of bounds, or the filter regex is invalid.
pub fn run(args: &InspectArgs, global: &GlobalArgs) -> Result<()> {
    let mode = OutputMode::from_args(global.json);
    let har = parser::load(&args.file)?;

    if let Some(ref pattern) = args.filter_url {
        let re = Regex::new(pattern).context("invalid --filter-url regex")?;
        let matches: Vec<(usize, &Entry)> = har
            .log
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| re.is_match(&e.request.url))
            .map(|(i, e)| (i + 1, e))
            .collect();

        if matches.is_empty() {
            tracing::warn!("no entries matched --filter-url '{pattern}'");
            anyhow::bail!("NO_MATCH");
        }

        if args.all {
            return match mode {
                OutputMode::Human => render_multi_human(&matches),
                OutputMode::Json => render_multi_json(&matches),
            };
        }

        let (idx, entry) = matches[0];
        return match mode {
            OutputMode::Human => render_human(idx, entry, matches.len()),
            OutputMode::Json => render_json(idx, entry),
        };
    }

    // --entry mode (default)
    let idx = args.entry;
    if idx < 1 || idx > har.log.entries.len() {
        anyhow::bail!(
            "entry index {idx} is out of range (file has {} entries)",
            har.log.entries.len()
        );
    }

    let entry = &har.log.entries[idx - 1];

    match mode {
        OutputMode::Human => render_human(idx, entry, har.log.entries.len()),
        OutputMode::Json => render_json(idx, entry),
    }
}

// ---------------------------------------------------------------------------
// Human rendering
// ---------------------------------------------------------------------------

/// Print a detailed human-readable view of a single entry.
fn render_human(idx: usize, entry: &Entry, total: usize) -> Result<()> {
    let mut out = String::new();

    // ── Header ──
    let line = "═".repeat(48);
    let _ = writeln!(
        out,
        "{} Entry {idx} of {total} {}",
        &line[..12],
        &line[12..]
    );
    let _ = writeln!(
        out,
        "Started: {}   Total time: {:.0}ms\n",
        entry.started_date_time, entry.time
    );

    render_request_section(&mut out, entry);
    render_response_section(&mut out, entry);
    render_timings_section(&mut out, entry);

    let mut stdout = std::io::stdout();
    std::io::Write::write_all(&mut stdout, out.as_bytes())
        .context("failed to write inspect output to stdout")?;
    std::io::Write::write_all(&mut stdout, b"\n")
        .context("failed to write newline")?;

    Ok(())
}

/// Render the request section: method, URL, headers, cookies, query, body.
fn render_request_section(out: &mut String, entry: &Entry) {
    let _ = writeln!(
        out,
        "── Request ──\n{method} {url} {version}\n",
        method = entry.request.method,
        url = entry.request.url,
        version = entry.request.http_version,
    );

    if !entry.request.headers.is_empty() {
        out.push_str("Headers:\n");
        for h in &entry.request.headers {
            let _ = writeln!(
                out,
                "  {name:<20} {value}",
                name = h.name,
                value = h.value
            );
        }
        out.push('\n');
    }

    if !entry.request.cookies.is_empty() {
        out.push_str("Cookies:\n");
        for c in &entry.request.cookies {
            let _ = writeln!(
                out,
                "  {name} = {value}",
                name = c.name,
                value = c.value
            );
        }
        out.push('\n');
    }

    if !entry.request.query_string.is_empty() {
        out.push_str("Query String:\n");
        for q in &entry.request.query_string {
            let _ = writeln!(
                out,
                "  {name:<20} {value}",
                name = q.name,
                value = q.value
            );
        }
        out.push('\n');
    }

    if let Some(ref post) = entry.request.post_data {
        let _ = writeln!(
            out,
            "Body: {mime} {body}\n",
            mime = post.mime_type,
            body = truncate(&post.text, 500)
        );
    }
}

/// Render the response section: status, headers, cookies, content, body preview.
fn render_response_section(out: &mut String, entry: &Entry) {
    let _ = writeln!(
        out,
        "── Response ──\n{status} {text} {version}\n",
        status = entry.response.status,
        text = entry.response.status_text,
        version = entry.response.http_version,
    );

    if !entry.response.headers.is_empty() {
        out.push_str("Headers:\n");
        for h in &entry.response.headers {
            let _ = writeln!(
                out,
                "  {name:<20} {value}",
                name = h.name,
                value = h.value
            );
        }
        out.push('\n');
    }

    if !entry.response.cookies.is_empty() {
        out.push_str("Cookies:\n");
        for c in &entry.response.cookies {
            let _ = writeln!(
                out,
                "  {name} = {value}",
                name = c.name,
                value = c.value
            );
        }
        out.push('\n');
    }

    let content = &entry.response.content;
    let _ = writeln!(
        out,
        "Content: {size}  {mime}",
        size = format_bytes(content.size),
        mime = content.mime_type,
    );
    if let Some(text) = &content.text {
        let _ = writeln!(out, "Body preview: {}", truncate(text, 300));
    }
    if !entry.response.redirect_url.is_empty() {
        let _ = writeln!(out, "Redirect → {}", entry.response.redirect_url);
    }
    out.push('\n');
}

/// Render the timings section with bottleneck marker.
fn render_timings_section(out: &mut String, entry: &Entry) {
    out.push_str("── Timings ──\n");
    let t = &entry.timings;
    let bottleneck = timing_bottleneck(t);
    for (label, val, is_bottleneck) in [
        ("Blocked", t.blocked, bottleneck == "blocked"),
        ("DNS", t.dns, bottleneck == "dns"),
        ("Connect", t.connect, bottleneck == "connect"),
        ("SSL", t.ssl, bottleneck == "ssl"),
        ("Send", Some(t.send), bottleneck == "send"),
        ("Wait", Some(t.wait), bottleneck == "wait"),
        ("Receive", Some(t.receive), bottleneck == "receive"),
    ] {
        let marker = if is_bottleneck {
            "  ◄── bottleneck"
        } else {
            ""
        };
        match val {
            Some(v) if v < 0.0 => {
                let _ = writeln!(out, "  {label:<10} - (unknown){marker}");
            }
            Some(v) => {
                let _ = writeln!(out, "  {label:<10} {v:.1}ms{marker}");
            }
            None => {
                let _ = writeln!(out, "  {label:<10} -{marker}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// JSON rendering
// ---------------------------------------------------------------------------

/// Write the entry as a single JSON object to stdout.
fn render_json(idx: usize, entry: &Entry) -> Result<()> {
    let output = InspectOutput {
        format_version: "1.0",
        entry_index: idx,
        started_date_time: &entry.started_date_time,
        time: entry.time,
        request: &entry.request,
        response: &entry.response,
        timings: &entry.timings,
        cache: &entry.cache,
        computed: InspectComputed {
            total_bytes: stats::response_size(entry),
            domain: stats::extract_domain(entry),
            content_type: entry.response.content.mime_type.clone(),
            bottleneck: timing_bottleneck(&entry.timings),
        },
    };

    let json = serde_json::to_string_pretty(&output)
        .context("failed to serialize inspect output")?;
    let mut stdout = std::io::stdout();
    std::io::Write::write_all(&mut stdout, json.as_bytes())
        .context("failed to write JSON to stdout")?;
    std::io::Write::write_all(&mut stdout, b"\n")
        .context("failed to write newline")?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Multi-match rendering (--filter-url --all)
// ---------------------------------------------------------------------------

/// Render multiple entries in human mode with separators.
fn render_multi_human(matches: &[(usize, &Entry)]) -> Result<()> {
    for (i, (idx, entry)) in matches.iter().enumerate() {
        if i > 0 {
            let mut stdout = std::io::stdout();
            std::io::Write::write_all(&mut stdout, b"\n").ok();
        }
        render_human(*idx, entry, matches.len())?;
    }
    Ok(())
}

/// Render multiple entries as NDJSON (one JSON object per line).
fn render_multi_json(matches: &[(usize, &Entry)]) -> Result<()> {
    let mut stdout = std::io::BufWriter::new(std::io::stdout());

    for (idx, entry) in matches {
        let output = build_output(*idx, entry);
        let line = serde_json::to_string(&output)
            .context("failed to serialize inspect output")?;
        std::io::Write::write_all(&mut stdout, line.as_bytes())
            .context("failed to write entry to stdout")?;
        std::io::Write::write_all(&mut stdout, b"\n")
            .context("failed to write newline")?;
    }

    Ok(())
}

/// Build an `InspectOutput` for a single entry.
fn build_output(idx: usize, entry: &Entry) -> InspectOutput<'_> {
    InspectOutput {
        format_version: "1.0",
        entry_index: idx,
        started_date_time: &entry.started_date_time,
        time: entry.time,
        request: &entry.request,
        response: &entry.response,
        timings: &entry.timings,
        cache: &entry.cache,
        computed: InspectComputed {
            total_bytes: stats::response_size(entry),
            domain: stats::extract_domain(entry),
            content_type: entry.response.content.mime_type.clone(),
            bottleneck: timing_bottleneck(&entry.timings),
        },
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Identify which timing phase is the slowest.
fn timing_bottleneck(t: &crate::har::types::Timings) -> String {
    let phases: [(&str, Option<f64>); 7] = [
        ("blocked", t.blocked),
        ("dns", t.dns),
        ("connect", t.connect),
        ("ssl", t.ssl),
        ("send", Some(t.send)),
        ("wait", Some(t.wait)),
        ("receive", Some(t.receive)),
    ];

    phases
        .iter()
        .filter(|(_, v)| v.is_some_and(|x| x >= 0.0))
        .max_by(|a, b| f64::total_cmp(&a.1.unwrap_or(0.0), &b.1.unwrap_or(0.0)))
        .map_or("unknown", |(name, _)| name)
        .to_owned()
}

/// Truncate a string to `max_len` characters, appending "…" if shortened.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_owned()
    } else {
        let end = s.floor_char_boundary(max_len);
        format!("{}…", &s[..end])
    }
}

/// Format a byte count as a human-readable string.
fn format_bytes(bytes: u64) -> String {
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

#[cfg(test)]
mod tests {
    use super::timing_bottleneck;
    use crate::har::types::Timings;

    #[test]
    fn bottleneck_identifies_wait_phase() {
        let t = Timings {
            blocked: Some(1.0),
            dns: Some(2.0),
            connect: Some(3.0),
            send: 0.5,
            wait: 150.0,
            receive: 10.0,
            ssl: Some(1.0),
            comment: None,
        };
        assert_eq!(timing_bottleneck(&t), "wait");
    }

    #[test]
    fn bottleneck_ignores_negative_values() {
        let t = Timings {
            blocked: Some(-1.0),
            dns: Some(5.0),
            connect: None,
            send: 1.0,
            wait: 2.0,
            receive: 3.0,
            ssl: None,
            comment: None,
        };
        assert_eq!(timing_bottleneck(&t), "dns");
    }

    #[test]
    fn bottleneck_all_negative_returns_unknown() {
        let t = Timings {
            blocked: Some(-1.0),
            dns: Some(-2.0),
            connect: Some(-3.0),
            send: -1.0,
            wait: -2.0,
            receive: -3.0,
            ssl: Some(-1.0),
            comment: None,
        };
        assert_eq!(timing_bottleneck(&t), "unknown");
    }
}
